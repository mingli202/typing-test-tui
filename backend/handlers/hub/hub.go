package hub

import (
	"encoding/json"
	"fmt"
	"log"
	"maps"
	"math/rand/v2"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"
	"tui/backend/models"
	"tui/backend/services/data_provider"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{}

type User struct {
	conn       *websocket.Conn
	id         string
	group      *Group
	totalWpm   float64
	gamePlayed int
	name       string
}

func (user *User) avgWpm() float64 {
	return user.totalWpm / float64(user.gamePlayed)
}

type Group struct {
	id    string
	users map[string]*User
	data  models.Data
}

// Makes a new group with the given id and data
func newGroup(id string, data models.Data) Group {
	return Group{
		id:    id,
		users: make(map[string]*User),
		data:  data,
	}
}

// Adds the given user to this group
func (group *Group) addUser(user *User) {
	group.users[user.id] = user
	user.group = group
}

// Removes the given user to this group
// Returns whether this group is empty
func (group *Group) removeUser(user *User) bool {
	delete(group.users, user.id)
	user.group = nil

	return len(group.users) == 0
}

// avgWpm gets the average wpm of this group
// Used to match users in relatively equal brackets
func (group *Group) avgWpm() float64 {
	totalWpm := 0.0
	n := 0

	for user := range maps.Values(group.users) {
		if user != nil {
			totalWpm += user.avgWpm()
			n += 1
		}
	}

	if n == 0 {
		return 0.0
	}

	return totalWpm / float64(n)
}

// Sends a message to every user of this group
func (group *Group) broadcast(msg string) {
	for user := range maps.Values(group.users) {
		if user.conn != nil {
			user.conn.WriteMessage(websocket.TextMessage, []byte(msg))
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

type Hub struct {
	mu           sync.RWMutex
	groups       map[string]Group
	dataProvider data_provider.DataProvider
}

// Makes a new hub
func newHub(dataProvider data_provider.DataProvider) Hub {
	return Hub{
		groups:       make(map[string]Group),
		dataProvider: dataProvider,
	}
}

// Adds a user to its user repository
// and returns the newly added user
func (hub *Hub) newUser(conn *websocket.Conn) *User {
	user := User{
		conn:  conn,
		id:    uuid.NewString(),
		group: nil,
	}

	return &user
}

// Removes the given user from the user repository
// Closes the user's connection
// Removes the user from its group if there is one
func (hub *Hub) removeUser(user *User) {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	if user.conn != nil {
		user.conn.Close()
		user.conn = nil
	}
	hub.leave(user)
}

// Returns a new unique group Id
// Assumes the lock is already acquired
func (hub *Hub) newGroupId() string {
	id := newGroupId()
	_, ok := hub.groups[id]

	for ok {
		id = newGroupId()
		_, ok = hub.groups[id]
	}

	return id
}

// Makes a new group and adds the given user to it
// Returns the newly created group id
func (hub *Hub) handleNewGroup(user *User) string {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	id := hub.newGroupId()

	data, _ := hub.dataProvider.NewData()

	group := newGroup(id, data)
	hub.groups[group.id] = group

	hub.join(group.id, user)

	return id
}

// Appends the given conn to the group with the given id
// If the user is already in a group, they will be removed from it
// Return whether the conn was added to the group
func (hub *Hub) handleJoin(groupId string, user *User) bool {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	return hub.join(groupId, user)
}

// Helper method for Join.
// Does nothing if group with given groupId is not found
// Does nothing is user tries to join its own group
// Assumes the lock is already acquired
func (hub *Hub) join(groupId string, user *User) bool {
	group, ok := hub.groups[groupId]

	if ok {
		if user.group != nil && groupId == user.group.id {
			return true
		}

		hub.leave(user)
		group.addUser(user)
	}

	return ok
}

// User leaves its group if any
// Returns whether the remove was successful or not
func (hub *Hub) handleLeave(user *User) bool {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	return hub.leave(user)
}

// Helper method for leave
// Assumes the mutex is already acquired
// Deletes group if there is nobody left in the group
// Returns whether the leave was successful or not
func (hub *Hub) leave(user *User) bool {
	if user.group != nil {
		group := user.group

		isEmpty := group.removeUser(user)
		if isEmpty {
			delete(hub.groups, group.id)
		}

		return true

	}

	return false
}

// Handles random matchmaking
func (hub *Hub) handleMatch(user *User) {}

/*
Handles websocket message.
Expects shape to be <Function> <Payload>.
Maps the message function to its own function (the client "calls" a function on the hub).
Returns a response message and error.

All Functions:

- NewGroup -> <LobbyResponse>

- JoinGroup <Id> -> <LobbyResponse>

- LeaveGroup -> <DidSucceed>

- Match -> <LobbyResponse>

- Start -> Countdown
*/
func (hub *Hub) handleMessage(p []byte, user *User) (string, error) {
	msg := string(p)
	words := strings.Split(msg, " ")

	log.Println(msg)

	function := words[0]

	switch function {
	case "NewGroup":
		id := hub.handleNewGroup(user)
		return id, nil

	case "JoinGroup":
		if len(words) != 2 {
			return "", ErrorMessage{Msg: "Format must be JoinGroup <Id>"}
		}

		id := words[1]
		success := hub.handleJoin(id, user)
		return strconv.FormatBool(success), nil

	case "LeaveGroup":
		success := hub.handleLeave(user)
		return strconv.FormatBool(success), nil
	case "Match":
		return "", nil
	case "UpdateStats":
		return "", nil

	default:
		return "", FunctionNotFoundError{Fn: function}
	}
}

func (hub *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	user := hub.newUser(conn)

	log.Printf("New user connected: %v\n", user)

	defer func() {
		hub.removeUser(user)
		log.Printf("User disconnected: %v\n", user)
	}()

	for {
		messageType, p, err := conn.ReadMessage()

		if err != nil {
			log.Println(err)
			return
		}

		if messageType != websocket.TextMessage {
			continue
		}

		returnMessage, err := hub.handleMessage(p, user)

		if err != nil {
			returnMessage = ErrorMessage{Msg: err.Error()}.Error()
		}

		err = conn.WriteMessage(websocket.TextMessage, []byte(returnMessage))

		if err != nil {
			log.Println(err)
		}
	}
}

func (hub *Hub) String() string {
	return fmt.Sprintf("Hub {\n    groups: %+v\n}", hub.groups)
}

func Handler(dataProvider data_provider.DataProvider) http.Handler {
	hub := newHub(dataProvider)

	ticker := time.NewTicker(5 * time.Second)

	go func() {
		for {
			_, ok := <-ticker.C

			if !ok {
				break
			}

			log.Println(hub.String())
		}
	}()

	return &hub
}

func newGroupId() string {
	s := ""

	for i := 0; i < 6; i += 1 {
		randomChar := rand.IntN('z'-'a') + 'a'
		s = s + string(rune(randomChar))
	}

	return s
}
