package hub

import (
	"fmt"
	"log"
	"math/rand/v2"
	"net/http"
	"strconv"
	"strings"
	"sync"
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
	dataProvider *data_provider.DataProvider
}

// Makes a new hub
func newHub(dataProvider *data_provider.DataProvider) Hub {
	return Hub{
		groups:       make(map[string]*group.Group),
		dataProvider: dataProvider,
	}
}

// Makes a new group and adds the given user to it
// Returns the lobbyInfo on success
func (hub *Hub) handleNewGroup(u *user.User) (models.LobbyInfo, error) {
	group := hub.newGroup()

	lobbyInfo, err := hub.handleJoin(group.Id(), u)

	return lobbyInfo, err
}

// User leaves its group if any
// If the group has no users, remove the group from the repo
// Otherwise, notify the users that a new user has joined
// Returns whether the remove was successful or not
func (hub *Hub) handleLeave(u *user.User) error {
	if groupId := u.GroupId; groupId != nil {
		return hub.leaveAndNotify(*groupId, u)
	}

	return fmt.Errorf("User not in any group to leave")
}

// Appends the given conn to the group with the given id
// If the user is already in a group, they will be removed from it
// If successful, notify group that a new user has joined
// Return the lobbyInfo on succesful join
func (hub *Hub) handleJoin(groupId string, u *user.User) (models.LobbyInfo, error) {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	oldGroupId := u.GroupId
	isJoiningSameGroup := oldGroupId != nil && *oldGroupId == groupId

	// asserts the joining group is not the same as the one already present
	if isJoiningSameGroup {
		return models.LobbyInfo{}, fmt.Errorf("How can you join the same group?")
	}

	group, err := hub.canJoinGroupLocked(groupId, u)

	if err != nil {
		return models.LobbyInfo{}, err
	}

	// leaves old group knowing that it's a different group (if any)
	// but only if the joining was successful
	if oldGroupId != nil {
		if oldGroup, _ := hub.leaveLocked(*oldGroupId, u); oldGroup != nil {
			oldGroup.SendUpdatePlayers()
		}

	}

	group.AddUser(u)

	group.SendUpdatePlayers()

	return group.GetLobbyInfo(), nil
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
// Returns the error if any
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
func (hub *Hub) newGroupIdLocked() string {
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

	id := hub.newGroupIdLocked()

	group := group.NewGroup(id, hub.dataProvider)
	hub.groups[group.Id()] = group

	return group
}

// Gets the group of the user
// Returns an error if user is not in any group or the group dones't exist
func (hub *Hub) getGroupOfUser(u *user.User) (*group.Group, error) {
	if u.GroupId == nil {
		return nil, fmt.Errorf("User not in any group!")
	}

	group, ok := hub.getGroup(*u.GroupId)

	if !ok {
		return nil, fmt.Errorf("Could not find any group with group id %s", *u.GroupId)
	}

	return group, nil
}

// Returns the group to be joined if it is possible
func (hub *Hub) canJoinGroupLocked(groupId string, u *user.User) (*group.Group, error) {
	group, ok := hub.groups[groupId]

	if !ok {
		return nil, fmt.Errorf("Could not find group %v to join", groupId)
	}

	return group, nil
}

// Have the given user leaveLocked the given group
// Returns the group that got left and an error if the user failed to leaveLocked the group
func (hub *Hub) leaveLocked(groupId string, u *user.User) (*group.Group, error) {
	group, ok := hub.groups[groupId]

	if !ok {
		return nil, fmt.Errorf("Did not find group to leave")
	}

	isEmpty := group.RemoveUser(u)
	if isEmpty {
		delete(hub.groups, groupId)
	}

	return group, nil
}

// Another helper function that handles notifying the group
// Returns an error if the user failed to leave the group
func (hub *Hub) leaveAndNotify(groupId string, u *user.User) error {
	hub.mu.Lock()
	group, err := hub.leaveLocked(groupId, u)
	hub.mu.Unlock()

	if err != nil {
		return err
	}

	group.SendUpdatePlayers()

	return nil
}

// TODO: Handles random matchmaking
func (hub *Hub) handleMatch(u *user.User) {}

/*
Handles websocket message.
Expects shape to be <Function> <Payload>.
Maps the message function to its own function (the client "calls" a function on the hub).
Returns a response message and error.

All Functions:

- NewGroup -> <LobbyInfo>

- JoinGroup <Id> -> <LobbyInfo>

- LeaveGroup -> <DidSucceed>

- Match -> <LobbyResponse> // TODO

- Start -> Countdown -> StartGame

- UpdateStats <Wpm> <Progress>
*/
func (hub *Hub) handleMessage(p []byte, u *user.User) (models.Message, error) {
	msg := string(p)
	words := strings.Split(msg, " ")

	function := words[0]

	switch function {
	case "NewGroup":
		lobbyInfo, err := hub.handleNewGroup(u)

		return lobbyInfo, err

	case "JoinGroup":
		if len(words) != 2 {
			return nil, fmt.Errorf("Format must be JoinGroup <Id>")
		}

		id := words[1]
		lobbyInfo, err := hub.handleJoin(id, u)

		return lobbyInfo, err

	case "LeaveGroup":
		err := hub.handleLeave(u)
		return models.LeaveGroupMessage{Success: err == nil}, nil
	case "UpdateStats":
		if len(words) != 3 {
			return nil, fmt.Errorf("Format must be UpdateStates <Wpm> <Progress>")
		}

		wpmStr := words[1]
		progressStr := words[2]

		wpm, err := strconv.ParseFloat(wpmStr, 64)

		if err != nil || wpm < 0 {
			return nil, fmt.Errorf("<Wpm> must be a positive float")
		}

		progress, err := strconv.ParseInt(progressStr, 10, 8)

		if err != nil || progress < 0 || progress > 100 {
			return nil, fmt.Errorf("<Progress> must be a positive int between 0 and 100")
		}

		err = hub.handleUpdateStats(u, wpm, uint8(progress))

		return nil, err

	case "StartGame":
		err := hub.handleStartGame(u)

		return nil, err

	default:
		return nil, FunctionNotFoundError{Fn: function}
	}
}

func (hub *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	user := user.NewUser(conn)
	user.InitWriteMessageCh()

	log.Printf("New user connected: %v\n", user.Id())

	defer func() {
		hub.removeUser(&user)
		log.Printf("User disconnected: %v\n", user.Id())
	}()

	// sends the user id to identify itself
	user.SendMsg(models.UserIdMessage{UserId: user.Id()})

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
			user.SendMsg(models.ErrorMessage{Err: err})
		} else if returnMessage != nil {
			user.SendMsg(returnMessage)
		}

	}
}

/* Stringer */
func (hub *Hub) String() string {
	hub.mu.RLock()
	defer hub.mu.RUnlock()

	return fmt.Sprintf("Hub {\n    groups: %#v\n}", hub.groups)
}

func Handler(dataProvider *data_provider.DataProvider) http.Handler {
	hub := newHub(dataProvider)

	return &hub
}

// Gets a random 6 lowercase alphabetical letters
func newGroupId() string {
	s := ""

	for i := 0; i < 6; i += 1 {
		randomChar := rand.IntN(26) + 'a'
		s = s + string(rune(randomChar))
	}

	return s
}
