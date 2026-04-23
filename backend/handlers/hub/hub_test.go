package hub

import (
	"encoding/json"
	"fmt"
	"testing"
	"tui/backend/models"
)

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

func TestNewUser(t *testing.T) {
	hub := newHub()

	user1 := hub.newUser(nil)

	if len(hub.groups) != 0 {
		t.Fatalf("How could a group been made?")
	}

	if len(hub.users) != 1 {
		t.Fatalf("Should have added a new user")
	}

	if user1.groupId != nil {
		t.Fatalf("User should not belong in any group for now")
	}
}

func TestRemoveUser(t *testing.T) {
	hub := newHub()

	user1 := hub.newUser(nil)

	hub.removeUser(user1)

	if len(hub.users) != 0 {
		t.Fatalf("Should have remove an user")
	}
}

func TestNewGroup(t *testing.T) {
	hub := newHub()

	user := hub.newUser(nil)

	groupId := hub.handleNewGroup(user)

	if len(hub.groups) != 1 {
		t.Fatalf("Should have added a group")
	}

	group, ok := hub.groups[groupId]

	if !ok {
		t.Fatalf("Why has the group not been added")
	}

	if len(group.users) != 1 {
		t.Fatalf("User not been added")
	}

	groupId = hub.handleNewGroup(user)

	if len(hub.groups) != 1 {
		t.Fatalf("Should have added a new group but old group is gone")
	}

	group2 := hub.groups[groupId]

	if group2.id == group.id {
		t.Fatalf("Impossible same group id")
	}

	if len(group2.users) != 1 {
		t.Fatalf("User should have been added to the new group")
	}
}

func TestJoin(t *testing.T) {
	hub := newHub()

	user1 := hub.newUser(nil)
	user2 := hub.newUser(nil)

	groupId1 := hub.handleNewGroup(user1)
	group1 := hub.groups[groupId1]

	// user 2 joins valid group
	ok := hub.handleJoin(groupId1, user2)

	if !ok {
		t.Fatalf("Join unsuccessful")
	}

	// user 1 joins invalid group
	ok = hub.handleJoin("ramdom groupId", user1)

	if ok {
		t.Fatalf("Group id not found, impossible")
	}

	if len(group1.users) != 2 {
		t.Fatalf("Group should still have 2 after invalid join")
	}

	// user1 makes another group
	groupId2 := hub.handleNewGroup(user1)
	group2 := hub.groups[groupId2]

	if len(group1.users) != 1 {
		t.Fatalf("User 1 should have left the first group")
	}

	hub.handleJoin(groupId2, user2)

	if len(group2.users) != 2 {
		t.Fatalf("User 2 should have joined the second group")
	}

	if len(group1.users) != 0 {
		t.Fatalf("Group1 should no longer have any users")
	}

	_, ok = hub.groups[group1.id]

	if ok {
		t.Fatalf("Group1 should have been deleted")
	}
}

func TestHandleMessageNewGroup(t *testing.T) {
	hub := newHub()

	user := hub.newUser(nil)

	msg := models.ReadMessage{
		Type:    "NewGroup",
		Payload: "",
	}

	msgBytes, err := json.Marshal(msg)

	if err != nil {
		t.Fatal(err)
	}

	resBytes, err := hub.handleMessage(msgBytes, user)

	if err != nil {
		t.Fatal(err)
	}

	resExpected := models.NewGroupResponse{}

	err = json.Unmarshal(resBytes, &resExpected)

	if err != nil {
		t.Fatal(err)
	}

	group, ok := hub.groups[resExpected.Id]

	if !ok {
		t.Fatal("Id returned an invalid group")
	}

	_, ok = group.users[user.id]

	if !ok {
		t.Fatal("Could not find user in returned group")
	}
}

func TestHandleMessageJoin(t *testing.T) {
	hub := newHub()

	user1 := hub.newUser(nil)

	groupId := hub.handleNewGroup(user1)

	user2 := hub.newUser(nil)

	msg := models.ReadMessage{
		Type:    "JoinGroup",
		Payload: fmt.Sprintf(`{"id": "%s"}`, groupId),
	}

	msgBytes, err := json.Marshal(msg)

	if err != nil {
		t.Fatal(err)
	}

	resBytes, err := hub.handleMessage(msgBytes, user2)

	if err != nil {
		t.Fatal(err)
	}

	resExpected := models.JoinResponse{}
	err = json.Unmarshal(resBytes, &resExpected)

	if err != nil {
		t.Fatal(err)
	}

	if resExpected.Success == false {
		t.Fatal("Unsuccessful join")
	}

	group := hub.groups[groupId]

	if len(group.users) != 2 {
		t.Fatal("Group does not have 2 users")
	}
}
