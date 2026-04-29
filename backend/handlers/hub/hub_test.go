package hub

import (
	"errors"
	"fmt"
	"net/http/httptest"
	"slices"
	"strconv"
	"strings"
	"sync"
	"testing"
	"tui/backend/handlers/hub/user"
	"tui/backend/services/data_provider"

	"github.com/gorilla/websocket"
)

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
	hubInstance := newHub(dataProvider)

	u := user.NewUser(nil)

	groupId := hubInstance.handleNewGroup(&u)

	if len(hubInstance.groups) != 1 {
		t.Fatalf("Should have added a group")
	}

	group, ok := hubInstance.groups[groupId]

	if !ok {
		t.Fatalf("Why has the group not been added")
	}

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatalf("User not been added")
	}

	groupId = hubInstance.handleNewGroup(&u)

	if len(hubInstance.groups) != 1 {
		t.Fatalf("Should have added a new group but old group is gone")
	}

	group2 := hubInstance.groups[groupId]

	if group2.Id() == group.Id() {
		t.Fatalf("Impossible same group id")
	}

	if len(group2.GetUsersSnapshot()) != 1 {
		t.Fatalf("User should have been added to the new group")
	}
}

func TestJoin(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)

	groupId1 := hubInstance.handleNewGroup(&user1)
	group1, ok := hubInstance.getGroup(groupId1)

	if !ok {
		t.Fatal("Where is the group??")
	}

	// user joins itself
	ok = hubInstance.handleJoin(groupId1, &user1)

	if !ok {
		t.Fatal("Technically the user can in fact join its own group")
	}

	// user 2 joins valid group
	ok = hubInstance.handleJoin(groupId1, &user2)

	if !ok {
		t.Fatalf("Join unsuccessful")
	}

	// user 1 joins invalid group
	ok = hubInstance.handleJoin("ramdom groupId", &user1)

	if ok {
		t.Fatalf("Group id not found, impossible")
	}

	if len(group1.GetUsersSnapshot()) != 2 {
		t.Fatalf("Group should still have 2 after invalid join")
	}

	// user1 makes another group
	groupId2 := hubInstance.handleNewGroup(&user1)
	group2 := hubInstance.groups[groupId2]

	if len(group1.GetUsersSnapshot()) != 1 {
		t.Fatalf("User 1 should have left the first group")
	}

	hubInstance.handleJoin(groupId2, &user2)

	if len(hubInstance.groups) != 1 {
		t.Fatal("Old group should have been removed")
	}

	if len(group2.GetUsersSnapshot()) != 2 {
		t.Fatalf("User 2 should have joined the second group")
	}

	if len(group1.GetUsersSnapshot()) != 0 {
		t.Fatalf("Group1 should no longer have any users")
	}

	_, ok = hubInstance.groups[group1.Id()]

	if ok {
		t.Fatalf("Group1 should have been deleted")
	}
}

func TestHandleMessageNewGroup(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user := user.NewUser(nil)

	msg := "NewGroup"

	res, err := hubInstance.handleMessage([]byte(msg), &user)

	if err != nil {
		t.Fatal(err)
	}

	id := res

	group, ok := hubInstance.groups[id]

	if !ok {
		t.Fatal("Id returned an invalid group")
	}

	if !slices.Contains(group.GetUsersSnapshot(), &user) {
		t.Fatal("Could not find user in returned group")
	}
}

func TestHandleMessageJoinGroup(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user1 := user.NewUser(nil)

	groupId := hubInstance.handleNewGroup(&user1)

	user2 := user.NewUser(nil)

	msg := "JoinGroup " + groupId

	res, err := hubInstance.handleMessage([]byte(msg), &user2)

	if err != nil {
		t.Fatal(err)
	}

	success, err := strconv.ParseBool(res)

	if err != nil {
		t.Fatal(err)
	}

	if success == false {
		t.Fatal("Unsuccessful join")
	}

	group := hubInstance.groups[groupId]

	if len(group.GetUsersSnapshot()) != 2 {
		t.Fatal("Group does not have 2 users")
	}
}

func TestHandleMessageLeaveGroup(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user1 := user.NewUser(nil)

	groupId := hubInstance.handleNewGroup(&user1)

	user2 := user.NewUser(nil)

	hubInstance.handleJoin(groupId, &user2)

	msg := "LeaveGroup"

	res, err := hubInstance.handleMessage([]byte(msg), &user2)

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

	group := hubInstance.groups[groupId]

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatal("Group does not have 1 user")
	}

	res, _ = hubInstance.handleMessage([]byte(msg), &user1)
	success, _ = strconv.ParseBool(res)
	if success == false {
		t.Fatal("Unsuccessful leave")
	}

	if len(group.GetUsersSnapshot()) != 0 {
		t.Fatal("Group does not have 0 user")
	}

	if _, ok := hubInstance.groups[groupId]; ok {
		t.Fatal("Group did not get removed")
	}
}

func TestRemoveUser(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)
	user3 := user.NewUser(nil)

	groupId1 := hubInstance.handleNewGroup(&user1)
	hubInstance.handleJoin(groupId1, &user2)

	hubInstance.removeUser(&user1)

	group1, ok := hubInstance.getGroup(groupId1)

	if !ok {
		t.Fatal("Where tf is the group??")
	}

	if len(group1.GetUsersSnapshot()) != 1 {
		t.Fatal("User1 did not get removed from its group")
	}

	hubInstance.removeUser(&user2)

	if _, ok = hubInstance.getGroup(groupId1); ok {
		t.Fatal("Group1 should have been removed")
	}

	hubInstance.removeUser(&user3) // don't crash pls
}

func TestHandleLeaveWithoutGroup(t *testing.T) {
	hubInstance := newHub(dataProvider)
	user := user.NewUser(nil)

	if hubInstance.handleLeave(&user) {
		t.Fatal("leave should fail for user that is not in a group")
	}
}

func TestLeaveHelperMatchesHandleLeaveBehavior(t *testing.T) {
	hubInstance := newHub(dataProvider)

	user1 := user.NewUser(nil)
	user2 := user.NewUser(nil)

	groupId := hubInstance.handleNewGroup(&user1)
	hubInstance.handleJoin(groupId, &user2)

	success := hubInstance.handleLeave(&user2)

	if !success {
		t.Fatal("leave helper should remove a user that belongs to a group")
	}

	group, ok := hubInstance.getGroup(groupId)
	if !ok {
		t.Fatal("group should still exist because user1 is still in it")
	}

	if len(group.GetUsersSnapshot()) != 1 {
		t.Fatal("group should keep a single user after helper leave")
	}
}

func TestHandleMessageJoinGroupBadFormat(t *testing.T) {
	hubInstance := newHub(dataProvider)
	user := user.NewUser(nil)

	cases := []string{
		"JoinGroup",
		"JoinGroup too many args",
	}

	for _, msg := range cases {
		_, err := hubInstance.handleMessage([]byte(msg), &user)
		if err == nil {
			t.Fatalf("expected error for %q", msg)
		}
	}
}

func TestHandleMessageUnknownFunction(t *testing.T) {
	hubInstance := newHub(dataProvider)
	user := user.NewUser(nil)

	_, err := hubInstance.handleMessage([]byte("DoesNotExist"), &user)
	if err == nil {
		t.Fatal("expected FunctionNotFoundError")
	}

	var fnErr FunctionNotFoundError
	if !errors.As(err, &fnErr) {
		t.Fatalf("expected FunctionNotFoundError, got %T", err)
	}
}

func TestHandleMessageEmptyInput(t *testing.T) {
	hubInstance := newHub(dataProvider)
	user := user.NewUser(nil)

	_, err := hubInstance.handleMessage([]byte(""), &user)
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
	hubInstance := newHub(dataProvider)

	anchor1 := user.NewUser(nil)
	groupId1 := hubInstance.handleNewGroup(&anchor1)

	anchor2 := user.NewUser(nil)
	groupId2 := hubInstance.handleNewGroup(&anchor2)

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

				ok := hubInstance.handleJoin(targetGroupID, u)
				if !ok {
					t.Errorf("join should succeed for valid group %s", targetGroupID)
					return
				}
			}
		}(us)
	}

	wg.Wait()

	group1, ok := hubInstance.getGroup(groupId1)
	if !ok {
		t.Fatal("group1 should still exist")
	}

	group2, ok := hubInstance.getGroup(groupId2)
	if !ok {
		t.Fatal("group2 should still exist")
	}

	totalUsers := len(group1.GetUsersSnapshot()) + len(group2.GetUsersSnapshot())
	if totalUsers != nUsers+2 {
		t.Fatalf("expected %d users across both groups (including anchors), got %d", nUsers+2, totalUsers)
	}
}

func TestLeaveGroupNoCrash(t *testing.T) {
	hub := newHub(dataProvider)

	server := httptest.NewServer(&hub)
	defer server.Close()

	conn1 := newTestConn(t, server)
	conn2 := newTestConn(t, server)
	conn3 := newTestConn(t, server)
	defer conn1.Close()
	defer conn2.Close()
	defer conn3.Close()

	if err := conn1.WriteMessage(websocket.TextMessage, []byte("NewGroup")); err != nil {
		t.Fatal(err)
	}

	msgType, res, err := conn1.ReadMessage()

	if msgType != websocket.TextMessage {
		t.Fatal("Response is not a text message")
	}
	if err != nil {
		t.Fatal(err)
	}

	groupId := string(res)

	if _, ok := hub.groups[groupId]; !ok {
		t.Fatal("NewGroup did not respond with groupid")
	}
}

// Gets a new connection to the test server
func newTestConn(t *testing.T, server *httptest.Server) *websocket.Conn {
	wsURL := "ws" + strings.TrimPrefix(server.URL, "http")

	conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)

	if err != nil {
		t.Fatal(err)
	}

	return conn
}
