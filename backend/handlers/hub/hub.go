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
func NewGroup(id string) Group {
	return Group{
		id:    id,
		users: make(map[string]bool),
	}
}

// Adds the given user to this group
func (group *Group) AddUser(user *User) {
	group.users[user.id] = true
	user.groupId = &group.id
}

// Removes the given user to this group
// Returns whether this group is empty
func (group *Group) RemoveUser(user *User) bool {
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
func NewHub() Hub {
	return Hub{
		groups: make(map[string]Group),
		users:  make(map[string]User),
	}
}

// Adds a user to its user repository
// and returns the newly added user
func (hub *Hub) NewUser(conn *websocket.Conn) *User {
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

// Removes the given user
func (hub *Hub) RemoveUser(user *User) {
	hub.mu.Lock()
	defer hub.mu.Unlock()

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
func (hub *Hub) NewGroup(user *User) string {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	id := hub.newGroupId()
	group := NewGroup(id)
	hub.groups[group.id] = group

	hub.join(group.id, user)

	return id
}

// Appends the given conn to the group with the given id
// If the user is already in a group, they will be removed from it
// Return whether the conn was added to the group
func (hub *Hub) Join(groupId string, user *User) bool {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	return hub.join(groupId, user)
}

// Helper method for Join.
// Assumes the lock is already acquired
func (hub *Hub) join(groupId string, user *User) bool {
	group, ok := hub.groups[groupId]

	if ok {
		if user.groupId != nil {
			hub.exit(*user.groupId, user)
		}

		group.AddUser(user)
	}

	return ok
}

// Removes the given user from the repository (e.g. when the user disconnects)
// Returns whether the remove was successful or not
func (hub *Hub) Exit(user *User) bool {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	delete(hub.users, user.id)

	return hub.exit(user)
}

// Helper method for Exit
// Assumes the mutex is already locked
// Returns whether the remove was successful or not
func (hub *Hub) exit(user *User) bool {
	if user.groupId != nil {
		id := *user.groupId

		group, ok := hub.groups[id]

		if ok {
			group.RemoveUser(user)
		}

		return ok
	}

	return false
}

// Handles websocket message
// Maps the message function to its own function (the client "calls" a function on the hub)
func (hub *Hub) HandleMessage(p []byte, user *User) ([]byte, error) {
	readMessage := models.ReadMessage{}

	err := json.Unmarshal(p, &readMessage)

	if err != nil {
		return []byte{}, err
	}

	switch readMessage.Type {
	case "NewGroup":
		id := hub.NewGroup(user)
		return json.Marshal(models.NewGroupResponse{Id: id})
	case "Join":
		joinGroup := models.JoinGroup{}
		err = json.Unmarshal([]byte(readMessage.Payload), &joinGroup)
		if err != nil {
			return []byte{}, err
		}

		success := hub.Join(joinGroup.Id, user)
		return json.Marshal(models.JoinResponse{Success: success})

	case "Exit":
		exitGroup := models.ExitGroup{}
		err = json.Unmarshal([]byte(readMessage.Payload), &exitGroup)

		if err != nil {
			return []byte{}, err
		}

		success := hub.Exit(exitGroup.Id, user)

		return json.Marshal(models.JoinResponse{Success: success})
	default:
		return []byte{}, TypeNotFoundError{}
	}
}

func (hub *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	user := hub.NewUser(conn)

	defer func() {
		hub.RemoveUser(user)
		conn.Close()
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

		returnMessage, err := hub.HandleMessage(p, user)

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
	hub := Hub{}

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
