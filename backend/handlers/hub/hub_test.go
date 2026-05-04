package hub

import (
	"encoding/json"
	"errors"
	"fmt"
	"maps"
	"net/http/httptest"
	"net/url"
	"slices"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
	"tui/backend/services/data_provider"

	"github.com/gorilla/websocket"
)

var dataProviderNoRef, _ = data_provider.NewDataProvider()
var dataProvider = &dataProviderNoRef

// A mock client
type MockClient struct {
	mu          sync.Mutex
	playersInfo models.PlayerInfoSnapshot
	lobbyInfo   models.LobbyInfo
	u           *user.User
	ch          chan models.Message
	wg          sync.WaitGroup
}

func newMockClient() *MockClient {
	u := user.NewUser(nil)

	ch := make(chan models.Message)

	u.SetCh(ch)

	mockClient := MockClient{
		u:  &u,
		ch: ch,
	}

	return &mockClient
}

func (mockClient *MockClient) close() {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	close(mockClient.ch)
}

func (mockClient *MockClient) listen(t *testing.T) {
	go func() {
		for msg := range mockClient.ch {
			str, errMsg := msg.ToMsg()

			if errMsg != nil {
				str, _ = models.ErrorMessage{Err: errMsg}.ToMsg()
			}

			mockClient.handleMsg(t, str)
		}
	}()
}

func (mockClient *MockClient) handleMsg(t *testing.T, msg string) {
	msgArr := strings.Split(msg, " ")

	if len(msg) < 1 {
		t.Fatalf("msg doesn't have cmd: %v", msg)
	}

	cmd := msgArr[0]

	switch cmd {
	case "PlayersInfo":
		if len(msg) < 2 {
			t.Fatalf("msg doesn't have payload: %v", msg)
		}

		playerInfoStr := strings.Join(msgArr[1:], " ")

		var playerInfo models.PlayerInfoSnapshot
		if err := json.Unmarshal([]byte(playerInfoStr), &playerInfo); err != nil {
			t.Fatalf("unmarshal error: %v", err)
		}

		mockClient.updatePlayers(playerInfo)
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

func (mockClient *MockClient) getPlayers() map[string]models.PlayerInfo {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	return maps.Clone(mockClient.playersInfo.Players)
}

func (mockClient *MockClient) getLobbyInfo() models.LobbyInfo {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	return mockClient.lobbyInfo
}

func (mockClient *MockClient) updatePlayers(playerInfo models.PlayerInfoSnapshot) {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	isSameLobby := playerInfo.LobbyId == mockClient.playersInfo.LobbyId
	isNewerPlayerInfoVersion := playerInfo.Version > mockClient.playersInfo.Version

	if !isSameLobby || isNewerPlayerInfoVersion {
		mockClient.playersInfo = playerInfo
	}

}

func TestMockClientUpdatePlayersIgnoresStaleAndDuplicateVersion(t *testing.T) {
	mockClient := newMockClient()

	freshPlayers := map[string]models.PlayerInfo{
		"u-new": {IsLeader: true, Wpm: 70, ProgressPercent: 50},
	}
	stalePlayers := map[string]models.PlayerInfo{
		"u-old": {IsLeader: false, Wpm: 10, ProgressPercent: 10},
	}
	duplicatePlayers := map[string]models.PlayerInfo{
		"u-dup": {IsLeader: false, Wpm: 99, ProgressPercent: 99},
	}

	mockClient.updatePlayers(models.PlayerInfoSnapshot{
		Version: 3,
		Players: freshPlayers,
	})
	mockClient.updatePlayers(models.PlayerInfoSnapshot{
		Version: 2,
		Players: stalePlayers,
	})
	mockClient.updatePlayers(models.PlayerInfoSnapshot{
		Version: 3,
		Players: duplicatePlayers,
	})

	got := mockClient.getPlayers()
	if !maps.Equal(got, freshPlayers) {
		t.Fatalf("stale/duplicate update should be ignored; got %+v want %+v", got, freshPlayers)
	}
}

func TestMockClientUpdatePlayersAppliesNewerEmptySnapshot(t *testing.T) {
	mockClient := newMockClient()

	mockClient.updatePlayers(models.PlayerInfoSnapshot{
		Version: 4,
		Players: map[string]models.PlayerInfo{
			"u1": {IsLeader: true, Wpm: 60, ProgressPercent: 80},
		},
	})

	mockClient.updatePlayers(models.PlayerInfoSnapshot{
		Version: 5,
		Players: map[string]models.PlayerInfo{},
	})

	got := mockClient.getPlayers()
	if len(got) != 0 {
		t.Fatalf("expected newer empty snapshot to clear players, got %+v", got)
	}
}

func TestMockClientHandleMsgOutOfOrderUpdatePlayers(t *testing.T) {
	mockClient := newMockClient()

	oldPayload, err := json.Marshal(models.PlayerInfoSnapshot{
		Version: 7,
		Players: map[string]models.PlayerInfo{
			"u-old": {IsLeader: false, Wpm: 40, ProgressPercent: 40},
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	newPayload, err := json.Marshal(models.PlayerInfoSnapshot{
		Version: 8,
		Players: map[string]models.PlayerInfo{
			"u-new": {IsLeader: true, Wpm: 85, ProgressPercent: 95},
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	mockClient.handleMsg(t, "PlayersInfo "+string(newPayload))
	mockClient.handleMsg(t, "PlayersInfo "+string(oldPayload))

	got := mockClient.getPlayers()
	want := map[string]models.PlayerInfo{
		"u-new": {IsLeader: true, Wpm: 85, ProgressPercent: 95},
	}

	if !maps.Equal(got, want) {
		t.Fatalf("out-of-order message handling failed: got %+v want %+v", got, want)
	}
}

func (mockClient *MockClient) updateLobbyInfo(lobbyInfo models.LobbyInfo) {
	mockClient.mu.Lock()
	defer mockClient.mu.Unlock()

	mockClient.lobbyInfo = lobbyInfo
}

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
		t.Fatal("Should have added a new group but old group is gone")
	}

	if u.GroupId == nil || *u.GroupId != lobby.LobbyId {
		t.Fatal("user groupid did not get set to the new group")
	}

	group2 := hub.groups[lobby.LobbyId]

	if group2.Id() == group.Id() {
		t.Fatalf("Impossible same group id")
	}

	if len(group2.GetUsersSnapshot()) != 1 {
		t.Fatalf("User should have been added to the new group")
	}
}

func TestHandleJoin(t *testing.T) {
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

	if err == nil {
		t.Fatal("Joining itself should be an error, make the client aware of it")
	}

	// user 2 joins valid group
	_, err = hub.handleJoin(groupId1, &user2)

	if err != nil {
		t.Fatalf("Join unsuccessful")
	}

	// user 1 joins invalid group
	_, err = hub.handleJoin("random groupId", &user1)

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

	if user1.GroupId == nil {
		t.Fatal("user groupid is gone")
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

	msgStr, errMsg := res.ToMsg()

	if errMsg != nil {
		t.Fatal(err)
	}

	lobbyStr := strings.Join(strings.Split(msgStr, " ")[1:], " ")

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

	if user.GroupId == nil || *user.GroupId != group.Id() {
		t.Fatal("user groupid did not get set")
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

	msgStr, errMsg := res.ToMsg()

	if errMsg != nil {
		t.Fatal(err)
	}

	lobbyStr := strings.Join(strings.Split(msgStr, " ")[1:], " ")

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

	msgStr, errMsg := res.ToMsg()

	if errMsg != nil {
		t.Fatal(err)
	}

	success, err := strconv.ParseBool(strings.Join(strings.Split(msgStr, " ")[1:], " "))

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

	msgStr, errMsg = res.ToMsg()

	if errMsg != nil {
		t.Fatal(err)
	}

	success, _ = strconv.ParseBool(strings.Join(strings.Split(msgStr, " ")[1:], " "))
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

	if hub.handleLeave(&user) == nil {
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

	success := hub.handleLeave(&user2) == nil

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

// Issue: UpdateStats accepts negative progress values, which are parsed then cast to uint8.
// Regression expectation: negative progress should be rejected at message validation time.
func TestHandleMessageUpdateStatsRejectsNegativeProgress(t *testing.T) {
	hub := newHub(dataProvider)
	u := user.NewUser(nil)

	_, err := hub.handleNewGroup(&u)
	if err != nil {
		t.Fatal(err)
	}

	_, err = hub.handleMessage([]byte("UpdateStats 70.0 -1"), &u)
	if err == nil {
		t.Fatal("expected validation error for negative progress")
	}

	if !strings.Contains(err.Error(), "<Progress>") {
		t.Fatalf("expected progress validation error, got: %v", err)
	}
}

// Issue: ServeHTTP builds ErrorMessage on handler error but did not send it to clients.
// Regression expectation: invalid commands should produce an "Error ..." websocket message.
func TestServeHTTPSendsErrorMessageOnInvalidCommand(t *testing.T) {
	h := Handler(dataProvider)
	srv := httptest.NewServer(h)
	defer srv.Close()

	u, err := url.Parse(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	u.Scheme = "ws"
	u.Path = "/"

	conn, _, err := websocket.DefaultDialer.Dial(u.String(), nil)
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()

	_, firstMsg, err := conn.ReadMessage()
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(string(firstMsg), "UserId ") {
		t.Fatalf("expected initial UserId message, got %q", string(firstMsg))
	}

	if err := conn.WriteMessage(websocket.TextMessage, []byte("JoinGroup")); err != nil {
		t.Fatal(err)
	}

	_, response, err := conn.ReadMessage()
	if err != nil {
		t.Fatal(err)
	}

	if !strings.HasPrefix(string(response), "Error ") {
		t.Fatalf("expected websocket error message, got %q", string(response))
	}
}

// Issue: stale/non-existent user.GroupId must not panic in getGroupOfUser.
// Expected behavior is a regular error that includes the missing group id.
func TestGetGroupOfUserMissingGroupDoesNotPanic(t *testing.T) {
	hub := newHub(dataProvider)
	u := user.NewUser(nil)
	missingGroupId := "missing-group-id"
	u.GroupId = &missingGroupId

	defer func() {
		if recovered := recover(); recovered != nil {
			t.Fatalf("getGroupOfUser should not panic for missing group, recovered: %v", recovered)
		}
	}()

	_, err := hub.getGroupOfUser(&u)
	if err == nil {
		t.Fatal("expected missing group error")
	}

	if !strings.Contains(err.Error(), missingGroupId) {
		t.Fatalf("expected error to include missing group id %q, got %q", missingGroupId, err.Error())
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

	if len(mockClient.getPlayers()) != 1 {
		t.Fatal("user1 did not get players notice")
	}

	if mockClient.u.GroupId == nil || *mockClient.u.GroupId != groupId {
		t.Fatal("actual user did not get its groupid set")
	}
}

func TestJoinGroupWithSync(t *testing.T) {
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
	groupId := mockClient1.getLobbyInfo().LobbyId

	mockClientMsg(t, &hub, mockClient2, "JoinGroup "+groupId)

	if len(mockClient1.getPlayers()) != 2 {
		t.Fatal("user1 did not receive player update")
	}
	if len(mockClient2.getPlayers()) != 2 {
		t.Fatal("user2 did not receive player update")
	}

	mockClientMsg(t, &hub, mockClient3, "JoinGroup "+groupId)

	if len(mockClient1.getPlayers()) != 3 {
		t.Fatal("user1 did not receive player update")
	}
	if len(mockClient2.getPlayers()) != 3 {
		t.Fatal("user2 did not receive player update")
	}
	if len(mockClient3.getPlayers()) != 3 {
		t.Fatal("user3 did not receive player update")
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

	groupId := mockClient1.getLobbyInfo().LobbyId

	mockClientMsg(t, &hub, mockClient2, "JoinGroup "+groupId)
	mockClientMsg(t, &hub, mockClient3, "JoinGroup "+groupId)

	// assert leader first
	if !mockClient2.getPlayers()[mockClient1.u.Id()].IsLeader {
		t.Fatal("Leader is not user1")
	}
	mockClientMsg(t, &hub, mockClient1, "LeaveGroup")

	if len(mockClient2.getPlayers()) != 2 {
		t.Fatal("user2 players did not get updated")
	}

	if len(mockClient3.getPlayers()) != 2 {
		t.Fatal("user3 players did not get updated")
	}

	if _, ok := mockClient3.getPlayers()[mockClient1.u.Id()]; ok {
		t.Fatal("user1 is still in players")
	}

	if !mockClient3.getPlayers()[mockClient2.u.Id()].IsLeader && !mockClient3.getPlayers()[mockClient3.u.Id()].IsLeader {
		t.Fatal("none of the two players are the leader")
	}
}

func TestStressTestSync(t *testing.T) {
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

	groupId := mockClient1.getLobbyInfo().LobbyId

	mockClientMsg(t, &hub, mockClient2, "JoinGroup "+groupId)

	const nClient = 10
	const nIterations = 100

	var wg sync.WaitGroup
	for i := 0; i < nClient; i++ {
		wg.Go(func() {
			mc := newMockClient()

			mc.listen(t)
			defer mc.close()

			for i := 0; i < nIterations; i++ {
				mockClientMsg(t, &hub, mc, "JoinGroup "+groupId)
				mockClientMsg(t, &hub, mc, "LeaveGroup")
			}
		})
	}

	mockClientMsg(t, &hub, mockClient3, "JoinGroup "+groupId)

	wg.Wait()

	// in the end, number of players should not have changed
	if len(mockClient1.getPlayers()) != 3 {
		t.Fatal("Number of players is not 3")
	}
	if len(mockClient2.getPlayers()) != 3 {
		t.Fatal("Number of players is not 3")
	}
	if len(mockClient3.getPlayers()) != 3 {
		t.Fatal("Number of players is not 3")
	}
}

func TestJoiningNewGroupWithLowerPlayerinfoVersion(t *testing.T) {
	// Arrange
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

	// Act
	mockClientMsg(t, &hub, mockClient1, "NewGroup")

	// Assert
	players := mockClient1.getPlayers()

	if len(players) != 1 {
		t.Fatal("There should have been only 1 player in this new group")
	}
}

func mockClientMsg(t *testing.T, hub *Hub, mockUser *MockClient, msg string) {
	u := mockUser.u

	res, err := hub.handleMessage([]byte(msg), u)

	if err != nil {
		u.SendMsg(models.ErrorMessage{Err: err})
	} else if res != nil {
		u.SendMsg(res)
	}

	time.Sleep(time.Millisecond * 10)
}
