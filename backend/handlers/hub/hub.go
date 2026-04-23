package hub

import (
	"fmt"
	"log"
	"math/rand/v2"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{}

type User struct {
	conn    *websocket.Conn
	id      string
	groupId *string
}

type Group struct {
	id    string
	users map[string]bool
}

// Makes a new group with the given id
func newGroup(id string) Group {
	return Group{
		id:    id,
		users: make(map[string]bool),
	}
}

// Adds the given user to this group
func (group *Group) addUser(user *User) {
	group.users[user.id] = true
	user.groupId = &group.id
}

// Removes the given user to this group
// Returns whether this group is empty
func (group *Group) removeUser(user *User) bool {
	delete(group.users, user.id)
	user.groupId = nil

	return len(group.users) == 0
}

type Hub struct {
	mu     sync.Mutex
	groups map[string]Group
	users  map[string]User
}

// Makes a new hub
func newHub() Hub {
	return Hub{
		groups: make(map[string]Group),
		users:  make(map[string]User),
	}
}

// Adds a user to its user repository
// and returns the newly added user
func (hub *Hub) newUser(conn *websocket.Conn) *User {
	user := User{
		conn:    conn,
		id:      uuid.NewString(),
		groupId: nil,
	}

	hub.mu.Lock()
	defer hub.mu.Unlock()

	_, ok := hub.users[user.id]

	for ok {
		user.id = uuid.NewString()
		_, ok = hub.users[user.id]
	}

	hub.users[user.id] = user

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
	}
	hub.leave(user)
	delete(hub.users, user.id)
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
	group := newGroup(id)
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
		if user.groupId != nil && groupId == *user.groupId {
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
	if user.groupId != nil {
		id := *user.groupId

		group, ok := hub.groups[id]

		if ok {
			isEmpty := group.removeUser(user)
			if isEmpty {
				delete(hub.groups, group.id)
			}
		}

		return ok
	}

	return false
}

/*
Handles websocket message.
Expects shape to be <Function> <Payload>.
Maps the message function to its own function (the client "calls" a function on the hub).
Returns a response message and error.

All Functions:

- NewGroup -> <NewlyJoinedGroupId>

- JoinGroup <Id> -> <DidSucceed>

- LeaveGroup -> <DidSucceed>
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
	return fmt.Sprintf("Hub {\n    groups: %+v\n    user: %+v\n}", hub.groups, hub.users)
}

func Handler() http.Handler {
	hub := newHub()

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
