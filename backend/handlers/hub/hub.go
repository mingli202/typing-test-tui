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
	"tui/backend/handlers/hub/group"
	"tui/backend/handlers/hub/user"
	"tui/backend/services/data_provider"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{}

type Hub struct {
	mu           sync.RWMutex
	groups       map[string]*group.Group
	dataProvider data_provider.DataProvider
}

// Makes a new hub
func newHub(dataProvider data_provider.DataProvider) Hub {
	return Hub{
		groups:       make(map[string]*group.Group),
		dataProvider: dataProvider,
	}
}

// Makes a new group and adds the given user to it
// Returns the newly created group id
func (hub *Hub) handleNewGroup(u *user.User) string {
	group := hub.newGroup()

	hub.handleJoin(group.Id(), u)

	return group.Id()
}

// User leaves its group if any
// If the group has no users, remove the group from the repo
// Returns whether the remove was successful or not
func (hub *Hub) handleLeave(u *user.User) bool {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	if groupId := u.GroupId; groupId != nil {
		group, ok := hub.groups[*groupId]

		if !ok {
			return false
		}

		isEmpty := group.RemoveUser(u)
		if isEmpty {
			delete(hub.groups, *groupId)
		}

		return true
	}

	return false
}

// Appends the given conn to the group with the given id
// If the user is already in a group, they will be removed from it
// Return whether the conn was added to the group
func (hub *Hub) handleJoin(groupId string, u *user.User) bool {
	group, ok := hub.getGroup(groupId)

	if !ok {
		return false
	}

	if u.GroupId != nil && *u.GroupId != groupId {
		hub.handleLeave(u)
	}

	group.AddUser(u)

	return true
}

// Removes the given user from the user repository
// Closes the user's connection
// Removes the user from its group if there is one
func (hub *Hub) removeUser(user *user.User) {
	hub.handleLeave(user)
	user.CloseConn()
}

func (hub *Hub) getGroup(id string) (*group.Group, bool) {
	hub.mu.RLock()
	defer hub.mu.RUnlock()

	group, ok := hub.groups[id]

	return group, ok
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

// Makes a new group in the hub
// Returns the newly created group
func (hub *Hub) newGroup() *group.Group {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	id := hub.newGroupId()

	data, _ := hub.dataProvider.NewData()

	group := group.NewGroup(id, data)
	hub.groups[group.Id()] = &group

	return &group
}

// Handles random matchmaking
func (hub *Hub) handleMatch(user *user.User) {}

/*
Handles websocket message.
Expects shape to be <Function> <Payload>.
Maps the message function to its own function (the client "calls" a function on the hub).
Returns a response message and error.

All Functions:

- NewGroup -> <LobbyResponse>

- JoinGroup <Id> -> <LobbyResponse>

- LeaveGroup -> <DidSucceed>

- Match -> <LobbyResponse> // TODO

- Start -> Countdown // TODO
*/
func (hub *Hub) handleMessage(p []byte, user *user.User) (string, error) {
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

	user := user.NewUser(conn)
	go user.InitWriteMessageCh()

	log.Printf("New user connected: %v\n", user)

	defer func() {
		hub.removeUser(&user)
		log.Printf("User disconnected: %v\n", user)
	}()

	// listen for incoming messages in current goroutine
	for {
		messageType, p, err := conn.ReadMessage()

		if err != nil {
			log.Println(err)
			return
		}

		if messageType != websocket.TextMessage {
			continue
		}

		returnMessage, err := hub.handleMessage(p, &user)

		if err != nil {
			returnMessage = ErrorMessage{Msg: err.Error()}.Error()
		}

		user.SendMsg(returnMessage)
	}
}

func (hub *Hub) String() string {
	return fmt.Sprintf("Hub {\n    groups: %#v\n}", hub.groups)
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
