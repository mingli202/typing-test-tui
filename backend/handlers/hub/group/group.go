package group

import (
	"encoding/json"
	"fmt"
	"iter"
	"log"
	"maps"
	"strings"
	"sync"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
)

type Group struct {
	mu            sync.RWMutex
	id            string
	users         map[string]*user.User
	leaderId      *string
	data          models.Data
	progress      map[string]*models.Progress
	isGameRunning bool
}

func (group *Group) Id() string {
	return group.id
}

// Makes a new group with the given id and data
func NewGroup(id string, data models.Data) Group {
	return Group{
		id:            id,
		users:         make(map[string]*user.User),
		data:          data,
		progress:      make(map[string]*models.Progress),
		isGameRunning: false,
	}
}

// Adds the given user to this group
// Sets the given user's groupId to this group's id
// There can be no duplicate users
func (group *Group) AddUser(u *user.User) {
	group.mu.Lock()
	defer group.mu.Unlock()

	group.users[u.Id()] = u
	u.GroupId = &group.id

	if group.leaderId == nil {
		group.newLeader()
	}
}

// Removes the given user to this grouptitle
// Removes the given user's groupId
// Returns whether this group is empty
func (group *Group) RemoveUser(u *user.User) bool {
	group.mu.Lock()
	defer group.mu.Unlock()

	delete(group.users, u.Id())
	u.GroupId = nil

	userId := u.Id()
	if *group.leaderId == userId {
		group.newLeader()
	}

	delete(group.progress, userId)

	return len(group.users) == 0
}

// Gets a snapshot of the group's user ids
func (group *Group) GetUserIdsSnapshot() []string {

	group.mu.RLock()
	defer group.mu.RUnlock()

	snapShot := make([]string, 0)

	for u := range maps.Values(group.users) {
		snapShot = append(snapShot, u.Id())
	}

	return snapShot
}

// Gets list of users at this moment of calling this function
func (group *Group) GetUsersSnapshot() []*user.User {
	group.mu.RLock()
	defer group.mu.RUnlock()

	snapShot := make([]*user.User, 0)

	for u := range maps.Values(group.users) {
		snapShot = append(snapShot, u)
	}

	return snapShot
}

// Update the running game's stats
// If there is no game, it does nothing
// Returns an error if the game is not running
func (group *Group) UpdateStats(u *user.User, wpm float64, progress uint8) error {
	group.mu.Lock()
	defer group.mu.Unlock()

	if !group.isGameRunning {
		return fmt.Errorf("Game is not running!")
	}

	if p, ok := group.progress[u.Id()]; ok {
		p.Wpm = wpm
		p.Progress = progress
	}

	return nil
}

// Starts the game
func (group *Group) UserStartGame(u *user.User) error {
	group.mu.Lock()
	defer group.mu.Unlock()

	if *group.leaderId != u.Id() {
		return fmt.Errorf("Only the leader can start the game")
	}

	go func() {
		group.countDown()
		group.startGame()
	}()

	return nil
}

// As a lobby snapshot
func (group *Group) AsLobbySnapshot() models.LobbyInfo {
	group.mu.RLock()
	defer group.mu.RUnlock()

	players := make(map[string]models.PlayerInfo)

	for id, u := range group.users {
		if u == nil {
			continue
		}

		players[id] = models.PlayerInfo{
			IsLeader: group.leaderId != nil && id == *group.leaderId,
		}
	}

	lobby := models.LobbyInfo{
		LobbyId: group.id,
		Data:    group.data,
		Players: players,
	}

	return lobby
}

// Broadcast the given message to the given slice of users
// Return if at least one user got the message
func (group *Group) BroadcastToUserWithId(userIds []string, msg string) bool {
	group.mu.RLock()
	defer group.mu.RUnlock()

	count := 0

	for _, userId := range userIds {
		u := group.users[userId]

		if u != nil {
			u.SendMsg(msg)
			count += 1
		}
	}

	return count > 0
}

// Sends a message to every user of this group
func (group *Group) broadcast(msg string) {
	group.mu.RLock()
	defer group.mu.RUnlock()

	for _, u := range group.users {
		if u != nil {
			u.SendMsg(msg)
		}
	}
}

// avgWpm gets the average wpm of this group
// Used to match users in relatively equal brackets
func (group *Group) avgWpm() float64 {
	totalWpm := 0.0
	n := 0

	users := group.GetUsersSnapshot()

	for _, u := range users {
		if u != nil {
			totalWpm += u.AvgWpm()
			n += 1
		}
	}

	if n == 0 {
		return 0.0
	}

	return totalWpm / float64(n)
}

// Starts the game and broadcasts updates every 1 second
func (group *Group) startGame() {
	group.setGameRunning()
	defer group.endGameRunning()

	userIds := group.GetUserIdsSnapshot()
	group.initProgressForUsers(userIds)

	minWpm := 30
	nWords := len(strings.Split(group.data.Text, " "))

	ticker := time.Tick(time.Second * 1)
	timer := time.NewTimer(time.Second * 60 * time.Duration(minWpm) * time.Duration(nWords))

	for {
		select {
		case <-ticker:
			progress := group.getProgressSnapshot()
			progressBytes, err := json.Marshal(progress)

			if err != nil {
				log.Println(err)
				return
			}

			atLeastOneSend := group.BroadcastToUserWithId(userIds, "ProgressUpdate "+string(progressBytes))

			if !atLeastOneSend {
				return
			}

		case <-timer.C:
			return
		}
	}
}

// Starts the countdown of 10 seconds, allows for joins and exits
func (group *Group) countDown() {
	ticker := time.Tick(time.Second * 1)
	countdown := 10

	for _ = range ticker {
		group.broadcast(fmt.Sprintf("Countdown %v", countdown))
		countdown -= 1

		if countdown == 0 {
			return
		}
	}
}

// Gets a snapshot of the progress
func (group *Group) getProgressSnapshot() map[string]models.Progress {
	group.mu.RLock()
	defer group.mu.RUnlock()

	progress := make(map[string]models.Progress)

	for k, v := range group.progress {
		progress[k] = *v
	}

	return progress
}

// Sets isGameRunning to true
func (group *Group) setGameRunning() {
	group.mu.Lock()
	defer group.mu.Unlock()

	group.isGameRunning = true
}

// Sets isGameRunning to false
func (group *Group) endGameRunning() {
	group.mu.Lock()
	defer group.mu.Unlock()

	group.isGameRunning = false
}

// Sets a new leader
// Leader is nil if there are no more available users
func (group *Group) newLeader() {
	userIds := maps.Keys(group.users)
	next, _ := iter.Pull(userIds)

	nextId, ok := next()

	if ok {
		group.leaderId = &nextId
	} else {
		group.leaderId = nil
	}
}

// Initialize progress of the given users
func (group *Group) initProgressForUsers(userIds []string) {
	group.mu.Lock()
	defer group.mu.Unlock()

	for _, id := range userIds {
		group.progress[id] = &models.Progress{}
	}
}
