package group

import (
	"encoding/json"
	"fmt"
	"iter"
	"log"
	"maps"
	"math"
	"strings"
	"sync"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
	"tui/backend/services/data_provider"
)

type GameStatus int

const (
	Waiting GameStatus = iota
	CountDown
	Playing
	End
)

type Group struct {
	mu                sync.RWMutex
	id                string
	users             map[string]*user.User
	leaderId          *string
	data              models.Data
	dataProvider      *data_provider.DataProvider
	playerInfo        map[string]*models.PlayerInfo
	playerInfoVersion uint64
	status            GameStatus
	end               chan struct{}
}

func (group *Group) Id() string {
	return group.id
}

// Makes a new group with the given id and data
func NewGroup(id string, dataProvider *data_provider.DataProvider) *Group {
	data, _ := dataProvider.NewData()

	group := Group{
		id:           id,
		users:        make(map[string]*user.User),
		data:         data,
		dataProvider: dataProvider,
		playerInfo:   make(map[string]*models.PlayerInfo),
		status:       Waiting,
	}

	return &group
}

// Adds the given user to this group.
// Sets the given user's groupId to this group's id.
// There can be no duplicate users.
// If a game has already started, the user can still join the group, however, they won't be added to the playerInfo since they can't be part of the already running game. The user is added the the group.users so they can receive broadcast and spectate the already running game.
func (group *Group) AddUser(u *user.User) {
	group.mu.Lock()
	defer group.mu.Unlock()

	group.users[u.Id()] = u
	u.GroupId = &group.id

	if group.leaderId == nil {
		group.newLeader()
	}

	if group.status != Playing {
		group.playerInfo[u.Id()] = &models.PlayerInfo{
			IsLeader: *group.leaderId == u.Id(),
		}
	}

	group.playerInfoVersion += 1
}

// Removes the given user to this grouptitle
// Removes the given user's groupId
// Returns whether this group is empty
func (group *Group) RemoveUser(u *user.User) bool {
	group.mu.Lock()

	delete(group.users, u.Id())
	u.GroupId = nil

	userId := u.Id()
	if group.leaderId == nil || *group.leaderId == userId {
		group.newLeader()
	}

	delete(group.playerInfo, userId)

	group.playerInfoVersion += 1

	isEmpty := len(group.users) == 0
	shouldEndGame := isEmpty && group.status == Playing
	group.mu.Unlock()

	if shouldEndGame {
		group.endGameRunning()
	}

	return isEmpty
}

// Gets a snapshot of the group's user ids
func (group *Group) GetUserIdsSnapshot() []string {
	group.mu.RLock()
	defer group.mu.RUnlock()

	return group.GetUserIdsSnapshotLocked()
}

// Gets a snapshot of the group's user ids
func (group *Group) GetUserIdsSnapshotLocked() []string {
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
func (group *Group) UpdateStats(u *user.User, wpm float64, progressPercent uint8) error {
	group.mu.Lock()
	defer group.mu.Unlock()

	if group.status != Playing {
		return fmt.Errorf("Game is not running!")
	}

	if p, ok := group.playerInfo[u.Id()]; ok {
		p.Wpm = wpm
		p.ProgressPercent = progressPercent

		group.playerInfoVersion += 1
	}

	return nil
}

// Starts the game
func (group *Group) UserStartGame(u *user.User) error {
	if err := group.canUserStartGame(u); err != nil {
		return err
	}

	group.newGameIfAlreadyEnded()

	group.mu.Lock()
	group.status = CountDown
	group.mu.Unlock()

	go func() {
		group.countDown()
		group.startGame()
		group.endGame()
	}()

	return nil
}

// As a lobby snapshot
func (group *Group) GetLobbyInfo() models.LobbyInfo {
	group.mu.RLock()
	defer group.mu.RUnlock()

	lobby := models.LobbyInfo{
		LobbyId: group.id,
		Data:    group.data,
	}

	return lobby
}

// Send UpdatePlayers msg
// Returns whether at least one user was sent the message
func (group *Group) SendUpdatePlayers() bool {
	playerInfo := group.getPlayerInfoSnapshot()
	playerInfoBytes, err := json.Marshal(playerInfo)

	if err != nil {
		log.Println(err)
		return false
	}

	msg := fmt.Sprintf("UpdatePlayers %v", string(playerInfoBytes))

	return group.broadcast(msg)
}

// Broadcast the given message to the given slice of users
// Return if at least one user got the message
func (group *Group) broadcastToUserWithId(userIds []string, msg string) bool {
	group.mu.RLock()
	defer group.mu.RUnlock()

	atLeastOne := false

	for _, userId := range userIds {
		u := group.users[userId]

		if u != nil {
			u.SendMsg(msg)
			atLeastOne = true
		}
	}

	return atLeastOne
}

// Sends a message to every user of this group
// Returns if at least one user was send the given msg
func (group *Group) broadcast(msg string) bool {
	users := group.GetUsersSnapshot()

	atLeastOne := false

	for _, u := range users {
		if u != nil {
			atLeastOne = true
			u.SendMsg(msg)
		}
	}

	return atLeastOne
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

// Starts the game and broadcasts updates every 1 second
// Will set a max time based on the word length, will be at least 2 minutes
// Return if this method successfully started a game and ended
func (group *Group) startGame() {
	if !group.setGameRunning() {
		return
	}

	group.mu.RLock()
	end := group.end
	group.mu.RUnlock()

	minWpm := 30
	nWords := len(strings.Split(group.data.Text, " "))

	ticker := time.Tick(time.Second * 1)

	timer := time.NewTimer(time.Second * 60 * time.Duration(math.Max(float64(nWords)/float64(minWpm), 2)))

	for {
		select {
		case <-ticker:
			atLeastOneSend := group.SendUpdatePlayers()

			if !atLeastOneSend || group.isGameCompleted() {
				return
			}

		case <-timer.C:
			return
		case <-end:
			return
		}
	}
}

// Show the end game screen
func (group *Group) endGame() {
	if !group.endGameRunning() {
		return
	}

	playerInfo := group.getPlayerInfoSnapshot()
	PlayerInfoBytes, err := json.Marshal(playerInfo)

	if err != nil {
		log.Println(err)
		return
	}

	group.resetPlayerInfo()
	group.broadcast("EndGameResult " + string(PlayerInfoBytes))
}

// Gets a snapshot of the playerInfo
func (group *Group) getPlayerInfoSnapshot() models.PlayerInfoSnapshot {
	group.mu.RLock()
	defer group.mu.RUnlock()

	v := group.playerInfoVersion
	playerInfo := make(map[string]models.PlayerInfo)

	for k, v := range group.playerInfo {
		playerInfo[k] = *v
	}

	return models.PlayerInfoSnapshot{
		Version: v,
		Players: playerInfo,
	}
}

// Sets isGameRunning to true
// Returns if it was successful
func (group *Group) setGameRunning() bool {
	group.mu.Lock()
	defer group.mu.Unlock()

	if group.status != Playing {
		group.status = Playing
		group.end = make(chan struct{})
		return true
	}

	return false
}

// Sets isGameRunning to false
// Returns of it was successful
func (group *Group) endGameRunning() bool {
	group.mu.Lock()
	defer group.mu.Unlock()

	if group.status != Playing {
		return false
	}

	group.status = End

	close(group.end)
	group.end = nil

	return true
}

// Sets a new leader
// Leader is nil if there are no more available users
func (group *Group) newLeader() {
	userIds := maps.Keys(group.users)
	next, stop := iter.Pull(userIds)
	defer stop()

	nextId, ok := next()

	if ok {
		group.leaderId = &nextId

		if playerInfo, ok := group.playerInfo[nextId]; ok {
			playerInfo.IsLeader = true
		}
	} else {
		group.leaderId = nil
	}
}

// Checks if the running game is completed
// It's completed when every player has achieved 100%
func (group *Group) isGameCompleted() bool {
	group.mu.RLock()
	defer group.mu.RUnlock()

	for _, playerInfo := range group.playerInfo {
		if playerInfo.ProgressPercent < 100 {
			return false
		}
	}

	return true
}

// Resets the player info after game end
func (group *Group) resetPlayerInfo() {
	group.mu.Lock()
	defer group.mu.Unlock()

	for _, userId := range group.GetUserIdsSnapshotLocked() {
		group.playerInfo[userId] = &models.PlayerInfo{
			IsLeader: group.leaderId != nil && *group.leaderId == userId,
		}
	}

	group.playerInfoVersion += 1
}

// Called when a new game is played after a game has already ended
// Gets new data and tell the users about it
func (group *Group) newGameIfAlreadyEnded() {
	group.mu.Lock()

	if group.status == End {
		newData := group.data

		if !group.dataProvider.HasLessThan2Quotes() {
			for newData == group.data {
				newData, _ = group.dataProvider.NewData()
			}
		}

		group.data = newData

		group.mu.Unlock()

		newGame := models.NewGame{
			Data:        newData,
			PlayersInfo: group.getPlayerInfoSnapshot(),
		}

		msg, err := newGame.ToMsg()

		if err != nil {
			return
		}

		group.broadcast(msg)
		return
	}

	group.mu.Unlock()
}

// well can the user start the game?
func (group *Group) canUserStartGame(u *user.User) error {
	group.mu.RLock()
	defer group.mu.RUnlock()

	if group.leaderId == nil || *group.leaderId != u.Id() {
		return fmt.Errorf("Only the leader can start the game")
	}

	if group.status == Playing || group.status == CountDown {
		return fmt.Errorf("Lobby is busy, cannot start")
	}

	return nil
}
