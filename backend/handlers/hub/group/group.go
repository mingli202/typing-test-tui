package group

import (
	"encoding/json"
	"fmt"
	"log"
	"maps"
	"strings"
	"sync"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
)

type Group struct {
	mu    sync.RWMutex
	id    string
	users map[string]*user.User
	data  models.Data
}

func (group *Group) Id() string {
	return group.id
}

// Makes a new group with the given id and data
func NewGroup(id string, data models.Data) Group {
	return Group{
		id:    id,
		users: make(map[string]*user.User),
		data:  data,
	}
}

// Adds the given user to this group
func (group *Group) AddUser(u *user.User) {
	group.mu.Lock()
	defer group.mu.Unlock()

	group.users[u.Id()] = u
	u.GroupId = &group.id
}

// Removes the given user to this group
// Returns whether this group is empty
func (group *Group) RemoveUser(u *user.User) bool {
	group.mu.Lock()
	defer group.mu.Unlock()

	delete(group.users, u.Id())
	u.GroupId = nil

	return len(group.users) == 0
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

// Sends a message to every user of this group
func (group *Group) broadcast(msg string) {
	users := group.GetUsersSnapshot()

	for _, u := range users {
		if u != nil {
			u.SendMsg(msg)
		}
	}
}

// Starts the game and broadcasts updates every 1 second
func (group *Group) startGame() {
	minWpm := 30
	nWords := len(strings.Split(group.data.Text, " "))

	progress := make(map[string]models.Progress)

	for userId := range maps.Keys(group.users) {
		progress[userId] = models.Progress{
			Wpm:      0,
			Progress: 0,
		}
	}

	ticker := time.Tick(time.Second * 1)
	timer := time.NewTimer(time.Second * 60 * time.Duration(minWpm) * time.Duration(nWords))

	countdown := 10

	for {
		select {
		case <-ticker:
			if countdown == 0 {
				progressBytes, err := json.Marshal(maps.Keys(progress))

				if err != nil {
					log.Println(err)
					break
				}

				group.broadcast("ProgressUpdate " + string(progressBytes))
			} else {
				group.broadcast(fmt.Sprintf("Countdown %v", countdown))
				countdown -= 1
			}
		case <-timer.C:
			break
		}

	}
}
