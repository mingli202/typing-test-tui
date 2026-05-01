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
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
	"tui/backend/services/data_provider"
)

// A mock client
type MockClient struct {
	mu        sync.Mutex
	players   map[string]models.PlayerInfo
	lobbyInfo models.LobbyInfo
	u         user.User
	ch        chan []byte
	wg        sync.WaitGroup
}

func newMockClient() *MockClient {
	u := user.NewUser(nil)

	ch := make(chan []byte)

	u.SetCh(ch)

	mockClient := MockClient{
		u:  u,
		ch: ch,
	}

	return &mockClient
}

func (mockClient *MockClient) close() {
	close(mockClient.ch)
}

func (mockClient *MockClient) listen(t *testing.T) {
	go func() {
		for p := range mockClient.ch {
			msg := string(p)
			log.Println("msg received " + msg)

			mockClient.handleMsg(t, msg)
		}
	}()
}

func (mockClient *MockClient) listenForMsg(t *testing.T) {
	mockClient.wg.Go(func() {
		log.Println("Waiting for msg")
		p := <-mockClient.ch
		msg := string(p)
		log.Println("msg received " + msg)

		mockClient.handleMsg(t, msg)
	})
}

func (mockClient *MockClient) listenForMsgN(t *testing.T, n int) {
	mockClient.wg.Go(func() {
		for i := 0; i < n; i += 1 {
			log.Println("Waiting for msg")
			p := <-mockClient.ch
			msg := string(p)
			log.Println("msg received " + msg)

			mockClient.handleMsg(t, msg)
		}
	})
}

func (mockClient *MockClient) handleMsg(t *testing.T, msg string) {
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

		mockClient.updatePlayers(players)
	case "LobbyInfo":
		if len(msg) < 2 {
			t.Fatalf("msg doesn't have payload: %v", msg)
		}

		lobbyStr := strings.Join(msgArr[1:], " ")

		var lobbyInfo models.LobbyInfo
		if err := json.Unmarshal([]byte(lobbyStr), &lobbyInfo); err != nil {
			t.Fatalf("unmarshal error: %v", err)
		}

		mockClient.updateLobbyInfo(lobbyInfo)
	}
}

func (mockClient *MockClient) waitForMsg() {
	mockClient.wg.Wait()
}

func (mockClient *MockClient) getPlayers() map[string]models.PlayerInfo {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	return maps.Clone(mockClient.players)
}

func (mockClient *MockClient) getLobbyInfo() models.LobbyInfo {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	return mockClient.lobbyInfo
}

func (mockClient *MockClient) updatePlayers(players map[string]models.PlayerInfo) {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	mockClient.players = players
}

func (mockClient *MockClient) updateLobbyInfo(lobbyInfo models.LobbyInfo) {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	mockClient.lobbyInfo = lobbyInfo
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

func TestNewGroupWithSync(t *testing.T) {
	hub := newHub(dataProvider)

	mockClient := newMockClient()

	mockClient.listen(t)
	defer mockClient.close()

	mockClientMsg(t, &hub, mockClient, "NewGroup")

	groupId := mockClient.getLobbyInfo().LobbyId

	if _, ok := hub.groups[groupId]; !ok {
		t.Fatal("NewGroup did not respond with groupid")
	}

	if len(mockClient.players) != 1 {
		t.Fatal("user1 did not get players notice")
	}
}

func TestJoinGroupWithSync(t *testing.T) {
	hub := newHub(dataProvider)
	mockClient1 := newMockClient()
	mockClient2 := newMockClient()

	mockClient1.listen(t)
	mockClient2.listen(t)
	defer mockClient1.close()
	defer mockClient2.close()

	mockClientMsg(t, &hub, mockClient1, "NewGroup")
	groupId := mockClient1.getLobbyInfo().LobbyId

	mockClientMsg(t, &hub, mockClient2, "JoinGroup "+groupId)

	if len(mockClient1.players) != 2 {
		t.Fatal("user1 did not receive player update")
	}
	if len(mockClient2.players) != 2 {
		t.Fatal("user1 did not receive player update")
	}
}

func TestLeaveGroupWithSync(t *testing.T) {
	hub := newHub(dataProvider)

	mockClient1 := newMockClient()
	mockClient2 := newMockClient()
	mockClient3 := newMockClient()

	mockClient1.listen(t)
	mockClient2.listen(t)
	mockClient3.listen(t)
	defer mockClient1.close()
	defer mockClient2.close()
	defer mockClient3.close()

	mockClientMsg(t, &hub, mockClient1, "NewGroup")

	groupId := mockClient1.lobbyInfo.LobbyId

	mockClientMsg(t, &hub, mockClient2, "JoinGroup "+groupId)
	mockClientMsg(t, &hub, mockClient3, "JoinGroup "+groupId)

	// assert leader first
	if !mockClient2.players[mockClient1.u.Id()].IsLeader {
		t.Fatal("Leader is not user1")
	}
	mockClientMsg(t, &hub, mockClient1, "LeaveGroup")

	if len(mockClient2.players) != 2 {
		t.Fatal("user2 players did not get updated")
	}

	if len(mockClient3.players) != 2 {
		t.Fatal("user3 players did not get updated")
	}
}

func mockClientMsg(t *testing.T, hub *Hub, mockUser *MockClient, msg string) {
	u := mockUser.u

	res, err := hub.handleMessage([]byte(msg), &u)

	if err != nil {
		u.SendMsg(err.Error())
	} else if res != "" {
		u.SendMsg(res)
	}

	time.Sleep(time.Millisecond * 10)
}
