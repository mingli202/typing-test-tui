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
	"tui/backend/models"
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
func (hub *Hub) handleNewGroup(u *user.User) (models.LobbyInfo, error) {
	group := hub.newGroup()

	lobbyInfo, err := hub.handleJoin(group.Id(), u)

	return lobbyInfo, err
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
func (hub *Hub) handleJoin(groupId string, u *user.User) (models.LobbyInfo, error) {
	group, ok := hub.getGroup(groupId)

	if !ok {
		return models.LobbyInfo{}, fmt.Errorf("Could not find group to join")
	}

	if u.GroupId != nil && *u.GroupId != groupId {
		hub.handleLeave(u)
	}

	group.AddUser(u)

	lobbyInfo := group.GetLobbyInfo()

	return lobbyInfo, nil
}

// Handles the updating of stats
// Does nothing if the user is not in a group
// Does nothing if the user's group can't be found
// Return the error if any
func (hub *Hub) handleUpdateStats(u *user.User, wpm float64, progress uint8) error {
	userGroup, err := hub.getGroupOfUser(u)

	if err != nil {
		return err
	}

	return userGroup.UpdateStats(u, wpm, progress)
}

// Handles starting a game
func (hub *Hub) handleStartGame(u *user.User) error {
	userGroup, err := hub.getGroupOfUser(u)

	if err != nil {
		return err
	}

	return userGroup.UserStartGame(u)
}

// Removes the given user from the user repository
// Closes the user's connection
// Removes the user from its group if there is one
func (hub *Hub) removeUser(user *user.User) {
	hub.handleLeave(user)
	user.Cleanup()
}

// Get the group associated with the given id
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

// Gets the group of the user
// Returns an error if user is not in any group or the group dones't exist
func (hub *Hub) getGroupOfUser(u *user.User) (*group.Group, error) {
	if u.GroupId == nil {
		return nil, fmt.Errorf("User not in any group!")
	}

	group, ok := hub.getGroup(*u.GroupId)

	if !ok {
		return nil, fmt.Errorf("Could not find any group with group id %s", group.Id())
	}

	return group, nil
}

// TODO: Handles random matchmaking
func (hub *Hub) handleMatch(u *user.User) {}

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

- UpdateStats <Wpm> <Progress>
*/
func (hub *Hub) handleMessage(p []byte, u *user.User) (string, error) {
	msg := string(p)
	words := strings.Split(msg, " ")

	log.Println(msg)

	function := words[0]

	switch function {
	case "NewGroup":
		lobbyInfo, err := hub.handleNewGroup(u)

		if err != nil {
			return "", err
		}

		str, err := lobbyInfo.ToMsg()
		if err != nil {
			return "", err
		}

		return string(str), nil

	case "JoinGroup":
		if len(words) != 2 {
			return "", fmt.Errorf("Format must be JoinGroup <Id>")
		}

		id := words[1]
		lobbyInfo, err := hub.handleJoin(id, u)
		if err != nil {
			return "", err
		}

		str, marshalErr := lobbyInfo.ToMsg()
		if marshalErr != nil {
			return "", err
		}

		return str, nil

	case "LeaveGroup":
		success := hub.handleLeave(u)
		return strconv.FormatBool(success), nil
	case "Match":
		return "", nil
	case "UpdateStats":
		if len(words) != 3 {
			return "", fmt.Errorf("Format must be UpdateStates <Wpm> <Progress>")
		}

		wpmStr := words[1]
		progressStr := words[2]

		wpm, err := strconv.ParseFloat(wpmStr, 64)

		if err != nil {
			return "", fmt.Errorf("<Wpm> must be a float")
		}

		progress, err := strconv.ParseInt(progressStr, 10, 8)

		if err != nil {
			return "", fmt.Errorf("<Progress> must be an int")
		}

		err = hub.handleUpdateStats(u, wpm, uint8(progress))

		return "", err

	case "StartGame":
		err := hub.handleStartGame(u)

		return "", err

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

// Gets a random 6 lowercase alphabetical letters
func newGroupId() string {
	s := ""

	for i := 0; i < 6; i += 1 {
		randomChar := rand.IntN('z'-'a') + 'a'
		s = s + string(rune(randomChar))
	}

	return s
}
