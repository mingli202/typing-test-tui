package hub

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"maps"
	"slices"
	"strconv"
	"strings"
	"sync"
	"testing"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
	"tui/backend/services/data_provider"
)

// A mock user
type MockUser struct {
	mu        sync.Mutex
	players   map[string]models.PlayerInfo
	lobbyInfo models.LobbyInfo
	u         user.User
	ch        chan []byte
	wg        sync.WaitGroup
}

func newMockUser(t *testing.T) *MockUser {
	u := user.NewUser(nil)

	ch := make(chan []byte)

	u.SetCh(ch)

	mockUser := MockUser{
		u:  u,
		ch: ch,
	}

	return &mockUser
}

func (mockUser *MockUser) listenForMsg(t *testing.T) {
	mockUser.wg.Go(func() {
		log.Println("Waiting for msg")
		p := <-mockUser.ch
		msg := string(p)
		log.Println("msg received " + msg)

		mockUser.handleMsg(t, msg)
	})
}

func (mockUser *MockUser) handleMsg(t *testing.T, msg string) {
	msgArr := strings.Split(msg, " ")

	if len(msg) < 1 {
		t.Fatalf("msg doesn't have cmd: %v", msg)
	}

	cmd := msgArr[0]

	switch cmd {
	case "UpdatePlayers":
		if len(msg) < 2 {
			t.Fatalf("msg doesn't have payload: %v", msg)
		}

		playersStr := strings.Join(msgArr[1:], " ")

		var players map[string]models.PlayerInfo
		if err := json.Unmarshal([]byte(playersStr), &players); err != nil {
			t.Fatalf("unmarshal error: %v", err)
		}

		mockUser.updatePlayers(players)
	case "LobbyInfo":
		if len(msg) < 2 {
			t.Fatalf("msg doesn't have payload: %v", msg)
		}

		lobbyStr := strings.Join(msgArr[1:], " ")

		var lobbyInfo models.LobbyInfo
		if err := json.Unmarshal([]byte(lobbyStr), &lobbyInfo); err != nil {
			t.Fatalf("unmarshal error: %v", err)
		}

		mockUser.updateLobbyInfo(lobbyInfo)
	}
}

func (mockUser *MockUser) waitForMsg() {
	mockUser.wg.Wait()
}

func (mockUser *MockUser) getPlayers() map[string]models.PlayerInfo {
	mockUser.mu.Lock()
	defer mockUser.mu.Unlock()

	return maps.Clone(mockUser.players)
}

func (mockUser *MockUser) getLobbyInfo() models.LobbyInfo {
	mockUser.mu.Lock()
	defer mockUser.mu.Unlock()

	return mockUser.lobbyInfo
}

func (mockUser *MockUser) updatePlayers(players map[string]models.PlayerInfo) {
	mockUser.mu.Lock()
	defer mockUser.mu.Unlock()

	mockUser.players = players
}

func (mockUser *MockUser) updateLobbyInfo(lobbyInfo models.LobbyInfo) {
	mockUser.mu.Lock()
	defer mockUser.mu.Unlock()

	mockUser.lobbyInfo = lobbyInfo
}

var dataProvider, _ = data_provider.NewDataProvider()

func TestNewGroupId(t *testing.T) {
	groupId := newGroupId()
	anotherGroupId := newGroupId()

	if groupId == "" || anotherGroupId == "" {
		t.Fatalf("One of the group id was empty: groupId (%v) anotherGroupId (%v)", groupId, anotherGroupId)
	}

	if groupId == anotherGroupId {
		t.Fatalf("Got same random id twice in a row")
	}
	fmt.Printf("groupId: %+v\n", groupId)
	fmt.Printf("anotherGroupId: %+v\n", anotherGroupId)
}

func TestHandleNewGroup(t *testing.T) {
	hub := newHub(dataProvider)

	u := user.NewUser(nil)

	lobby, err := hub.handleNewGroup(&u)
	if err != nil {
		t.Fatal(err)
	}

	if len(hub.groups) != 1 {
		t.Fatalf("Should have added a group")
	}

	group, ok := hub.groups[lobby.LobbyId]

	if !ok {
		t.Fatalf("Why has the group not been added")
	}

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatalf("User not been added")
	}

	lobby, err = hub.handleNewGroup(&u)
	if err != nil {
		t.Fatal(err)
	}

	if len(hub.groups) != 1 {
		t.Fatalf("Should have added a new group but old group is gone")
	}

	group2 := hub.groups[lobby.LobbyId]

	if group2.Id() == group.Id() {
		t.Fatalf("Impossible same group id")
	}

	if len(group2.GetUsersSnapshot()) != 1 {
		t.Fatalf("User should have been added to the new group")
	}
}

func TestJoin(t *testing.T) {
	hub := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)

	lobby1, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId1 := lobby1.LobbyId
	group1, ok := hub.getGroup(groupId1)

	if !ok {
		t.Fatal("Where is the group??")
	}

	// user joins itself
	_, err = hub.handleJoin(groupId1, &user1)

	if err != nil {
		t.Fatal("Technically the user can in fact join its own group")
	}

	// user 2 joins valid group
	_, err = hub.handleJoin(groupId1, &user2)

	if err != nil {
		t.Fatalf("Join unsuccessful")
	}

	// user 1 joins invalid group
	_, err = hub.handleJoin("ramdom groupId", &user1)

	if err == nil {
		t.Fatalf("Group id not found, impossible")
	}

	if len(group1.GetUsersSnapshot()) != 2 {
		t.Fatalf("Group should still have 2 after invalid join")
	}

	// user1 makes another group
	lobby2, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId2 := lobby2.LobbyId
	group2 := hub.groups[groupId2]

	if len(group1.GetUsersSnapshot()) != 1 {
		t.Fatalf("User 1 should have left the first group")
	}

	_, err = hub.handleJoin(groupId2, &user2)
	if err != nil {
		t.Fatal(err)
	}

	if len(hub.groups) != 1 {
		t.Fatal("Old group should have been removed")
	}

	if len(group2.GetUsersSnapshot()) != 2 {
		t.Fatalf("User 2 should have joined the second group")
	}

	if len(group1.GetUsersSnapshot()) != 0 {
		t.Fatalf("Group1 should no longer have any users")
	}

	_, ok = hub.groups[group1.Id()]

	if ok {
		t.Fatalf("Group1 should have been deleted")
	}
}

func TestHandleMessageNewGroup(t *testing.T) {
	hub := newHub(dataProvider)

	user := user.NewUser(nil)

	msg := "NewGroup"

	res, err := hub.handleMessage([]byte(msg), &user)

	if err != nil {
		t.Fatal(err)
	}

	lobbyStr := strings.Join(strings.Split(res, " ")[1:], " ")

	var lobby models.LobbyInfo
	if err := json.Unmarshal([]byte(lobbyStr), &lobby); err != nil {
		t.Fatal(err)
	}
	id := lobby.LobbyId

	group, ok := hub.groups[id]

	if !ok {
		t.Fatal("Id returned an invalid group")
	}

	if !slices.Contains(group.GetUsersSnapshot(), &user) {
		t.Fatal("Could not find user in returned group")
	}
}

func TestHandleMessageJoinGroup(t *testing.T) {
	hub := newHub(dataProvider)

	user1 := user.NewUser(nil)

	lobby, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId := lobby.LobbyId

	user2 := user.NewUser(nil)

	msg := "JoinGroup " + groupId

	res, err := hub.handleMessage([]byte(msg), &user2)

	if err != nil {
		t.Fatal(err)
	}

	lobbyStr := strings.Join(strings.Split(res, " ")[1:], " ")

	var joinLobby models.LobbyInfo
	if err := json.Unmarshal([]byte(lobbyStr), &joinLobby); err != nil {
		t.Fatal(err)
	}

	if joinLobby.LobbyId != groupId {
		t.Fatal("join returned wrong lobby id")
	}

	group := hub.groups[groupId]

	if len(group.GetUsersSnapshot()) != 2 {
		t.Fatal("Group does not have 2 users")
	}
}

func TestHandleMessageLeaveGroup(t *testing.T) {
	hub := newHub(dataProvider)

	user1 := user.NewUser(nil)

	lobby, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId := lobby.LobbyId

	user2 := user.NewUser(nil)

	_, err = hub.handleJoin(groupId, &user2)
	if err != nil {
		t.Fatal(err)
	}

	msg := "LeaveGroup"

	res, err := hub.handleMessage([]byte(msg), &user2)

	if err != nil {
		t.Fatal(err)
	}

	success, err := strconv.ParseBool(res)

	if err != nil {
		t.Fatal(err)
	}

	if success == false {
		t.Fatal("Unsuccessful leave")
	}

	group := hub.groups[groupId]

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatal("Group does not have 1 user")
	}

	res, _ = hub.handleMessage([]byte(msg), &user1)
	success, _ = strconv.ParseBool(res)
	if success == false {
		t.Fatal("Unsuccessful leave")
	}

	if len(group.GetUsersSnapshot()) != 0 {
		t.Fatal("Group does not have 0 user")
	}

	if _, ok := hub.groups[groupId]; ok {
		t.Fatal("Group did not get removed")
	}
}

func TestRemoveUser(t *testing.T) {
	hub := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)
	user3 := user.NewUser(nil)

	lobby, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId1 := lobby.LobbyId
	_, err = hub.handleJoin(groupId1, &user2)
	if err != nil {
		t.Fatal(err)
	}

	hub.removeUser(&user1)

	group1, ok := hub.getGroup(groupId1)

	if !ok {
		t.Fatal("Where tf is the group??")
	}

	if len(group1.GetUsersSnapshot()) != 1 {
		t.Fatal("User1 did not get removed from its group")
	}

	hub.removeUser(&user2)

	if _, ok = hub.getGroup(groupId1); ok {
		t.Fatal("Group1 should have been removed")
	}

	hub.removeUser(&user3) // don't crash pls
}

func TestHandleLeaveWithoutGroup(t *testing.T) {
	hub := newHub(dataProvider)
	user := user.NewUser(nil)

	if hub.handleLeave(&user) {
		t.Fatal("leave should fail for user that is not in a group")
	}
}

func TestLeaveHelperMatchesHandleLeaveBehavior(t *testing.T) {
	hub := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)

	lobby, err := hub.handleNewGroup(&user1)
	if err != nil {
		t.Fatal(err)
	}
	groupId := lobby.LobbyId
	_, err = hub.handleJoin(groupId, &user2)
	if err != nil {
		t.Fatal(err)
	}

	success := hub.handleLeave(&user2)

	if !success {
		t.Fatal("leave helper should remove a user that belongs to a group")
	}

	group, ok := hub.getGroup(groupId)
	if !ok {
		t.Fatal("group should still exist because user1 is still in it")
	}

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatal("group should keep a single user after helper leave")
	}
}

func TestHandleMessageJoinGroupBadFormat(t *testing.T) {
	hub := newHub(dataProvider)
	user := user.NewUser(nil)

	cases := []string{
		"JoinGroup",
		"JoinGroup too many args",
	}

	for _, msg := range cases {
		_, err := hub.handleMessage([]byte(msg), &user)
		if err == nil {
			t.Fatalf("expected error for %q", msg)
		}
	}
}

func TestHandleMessageUnknownFunction(t *testing.T) {
	hub := newHub(dataProvider)
	user := user.NewUser(nil)

	_, err := hub.handleMessage([]byte("DoesNotExist"), &user)
	if err == nil {
		t.Fatal("expected FunctionNotFoundError")
	}

	var fnErr FunctionNotFoundError
	if !errors.As(err, &fnErr) {
		t.Fatalf("expected FunctionNotFoundError, got %T", err)
	}
}

func TestHandleMessageEmptyInput(t *testing.T) {
	hub := newHub(dataProvider)
	user := user.NewUser(nil)

	_, err := hub.handleMessage([]byte(""), &user)
	if err == nil {
		t.Fatal("expected error for empty message")
	}

	var fnErr FunctionNotFoundError
	if !errors.As(err, &fnErr) {
		t.Fatalf("expected FunctionNotFoundError, got %T", err)
	}

	if fnErr.Fn != "" {
		t.Fatalf("expected empty function name, got %q", fnErr.Fn)
	}
}

func TestConcurrentJoinStability(t *testing.T) {
	hub := newHub(dataProvider)

	anchor1 := user.NewUser(nil)
	lobby1, err := hub.handleNewGroup(&anchor1)
	if err != nil {
		t.Fatal(err)
	}
	groupId1 := lobby1.LobbyId

	anchor2 := user.NewUser(nil)
	lobby2, err := hub.handleNewGroup(&anchor2)
	if err != nil {
		t.Fatal(err)
	}
	groupId2 := lobby2.LobbyId

	const nUsers = 24
	const nIterations = 200

	users := make([]*user.User, 0, nUsers)
	for i := 0; i < nUsers; i++ {
		user := user.NewUser(nil)
		users = append(users, &user)
	}

	var wg sync.WaitGroup

	for _, us := range users {
		wg.Add(1)
		go func(u *user.User) {
			defer wg.Done()

			for i := 0; i < nIterations; i++ {
				targetGroupID := groupId1
				if i%2 == 1 {
					targetGroupID = groupId2
				}

				_, err := hub.handleJoin(targetGroupID, u)
				if err != nil {
					t.Errorf("join should succeed for valid group %s", targetGroupID)
					return
				}
			}
		}(us)
	}

	wg.Wait()

	group1, ok := hub.getGroup(groupId1)
	if !ok {
		t.Fatal("group1 should still exist")
	}

	group2, ok := hub.getGroup(groupId2)
	if !ok {
		t.Fatal("group2 should still exist")
	}

	totalUsers := len(group1.GetUsersSnapshot()) + len(group2.GetUsersSnapshot())
	if totalUsers != nUsers+2 {
		t.Fatalf("expected %d users across both groups (including anchors), got %d", nUsers+2, totalUsers)
	}
}

func TestNewGroupWithRes(t *testing.T) {
	hub := newHub(dataProvider)

	mockUser := newMockUser(t)

	mockUser.listenForMsg(t) // expects 1 msg

	mockClientMsg(t, &hub, mockUser, "NewGroup")

	// expects 1 msg
	mockUser.waitForMsg()

	groupId := mockUser.getLobbyInfo().LobbyId

	if _, ok := hub.groups[groupId]; !ok {
		t.Fatal("NewGroup did not respond with groupid")
	}
}

// func TestJoinGroupWithConn(t *testing.T) {
// 	hub := newHub(dataProvider)
//
// 	u1 := newMockUser(t, server)
// 	u2 := newMockUser(t, server)
// 	u3 := newMockUser(t, server)
//
// 	defer u1.cleanup()
// 	defer u2.cleanup()
// 	defer u3.cleanup()
//
// 	u1.sendMsg(t, "NewGroup")
//
// 	groupId := u1.getLobbyInfo().LobbyId
//
// 	// User2 join group
// 	u2.sendMsg(t, "JoinGroup "+groupId)
//
// 	if u2.getLobbyInfo().LobbyId != groupId {
// 		t.Fatal("User 2 should be able to join group")
// 	}
//
// 	// User1 should have received join notice
//
// 	if len(u1.getPlayers()) != 2 {
// 		t.Fatal("User1 did not receive join notice")
// 	}
//
// 	// User3 join group
// 	u3.sendMsg(t, "JoinGroup "+groupId)
//
// 	if u3.getLobbyInfo().LobbyId != groupId {
// 		t.Fatal("User 3 should be able to join group")
// 	}
//
// 	// User1 and user2 should also have received
// 	if len(u1.getPlayers()) != 3 {
// 		t.Fatal("User 2 should have recieved new lobby")
// 	}
//
// 	if len(u2.getPlayers()) != 3 {
// 		t.Fatal("User 2 should have recieved new lobby")
// 	}
//
// }

func mockClientMsg(t *testing.T, hub *Hub, mockUser *MockUser, msg string) {
	u := mockUser.u

	res, err := hub.handleMessage([]byte(msg), &u)

	if err != nil {
		u.SendMsg(err.Error())
	} else if res != "" {
		u.SendMsg(res)
	}
}
