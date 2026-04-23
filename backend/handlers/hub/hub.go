package hub

import (
	"encoding/json"
	"log"
	"math/rand/v2"
	"net/http"
	"sync"
	"tui/backend/models"

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
// Assumes the lock is already acquired
func (hub *Hub) join(groupId string, user *User) bool {
	group, ok := hub.groups[groupId]

	if ok {
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

// Handles websocket message
// Maps the message function to its own function (the client "calls" a function on the hub)
// Returns a response message and error
func (hub *Hub) handleMessage(p []byte, user *User) ([]byte, error) {
	readMessage := models.ReadMessage{}

	err := json.Unmarshal(p, &readMessage)

	if err != nil {
		return []byte{}, err
	}

	switch readMessage.Type {
	case "NewGroup":
		id := hub.handleNewGroup(user)
		return json.Marshal(models.NewGroupResponse{Id: id})

	case "Join":
		joinGroup := models.JoinGroup{}
		err = json.Unmarshal([]byte(readMessage.Payload), &joinGroup)
		if err != nil {
			return []byte{}, err
		}

		success := hub.handleJoin(joinGroup.Id, user)
		return json.Marshal(models.JoinResponse{Success: success})

	case "Leave":
		exitGroup := models.LeaveGroup{}
		err = json.Unmarshal([]byte(readMessage.Payload), &exitGroup)

		if err != nil {
			return []byte{}, err
		}

		success := hub.handleLeave(user)

		return json.Marshal(models.JoinResponse{Success: success})

	default:
		return []byte{}, TypeNotFoundError{Type: readMessage.Type}
	}
}

func (hub *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	user := hub.newUser(conn)

	defer func() {
		hub.removeUser(user)
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
			errBytes, errErr := json.Marshal(err)

			if errErr == nil {
				err = conn.WriteMessage(websocket.TextMessage, errBytes)
			}

		} else {
			err = conn.WriteMessage(websocket.TextMessage, []byte(returnMessage))
		}

		if err != nil {
			log.Println(err)
		}
	}
}

func Handler() http.Handler {
	hub := newHub()

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
