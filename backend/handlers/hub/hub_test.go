package hub

import (
	"fmt"
	"testing"
)

func TestNewGroupId(t *testing.T) {
	groupId := newGroupId()
	anotherGroupId := newGroupId()

	if groupId == "" || anotherGroupId == "" {
		t.Errorf("One of the group id was empty: groupId (%v) anotherGroupId (%v)", groupId, anotherGroupId)
	}

	if groupId == anotherGroupId {
		t.Errorf("Got same random id twice in a row")
	}
	fmt.Printf("groupId: %+v\n", groupId)
	fmt.Printf("anotherGroupId: %+v\n", anotherGroupId)
}

func TestNewUser(t *testing.T) {
	hub := NewHub()

	user1 := hub.NewUser(nil)

	if len(hub.groups) != 0 {
		t.Errorf("How could a group been made?")
	}

	if len(hub.users) != 1 {
		t.Errorf("Should have added a new user")
	}

	if user1.groupId != nil {
		t.Errorf("User should not belong in any group for now")
	}
}

func TestRemoveUser(t *testing.T) {
	hub := NewHub()

	user1 := hub.NewUser(nil)

	hub.RemoveUser(user1)

	if len(hub.users) != 0 {
		t.Errorf("Should have remove an user")
	}
}

func TestNewGroup(t *testing.T) {
	hub := NewHub()

	user := hub.NewUser(nil)

	groupId := hub.NewGroup(user)

	if len(hub.groups) != 1 {
		t.Errorf("Should have added a group")
	}

	group, ok := hub.groups[groupId]

	if !ok {
		t.Errorf("Why has the group not been added")
	}

	if len(group.users) != 1 {
		t.Errorf("User not been added")
	}

	groupId = hub.NewGroup(user)

	if len(hub.groups) != 1 {
		t.Errorf("Should have added a new group but old group is gone")
	}

	group2 := hub.groups[groupId]

	if group2.id == group.id {
		t.Errorf("Impossible same group id")
	}

	if len(group2.users) != 1 {
		t.Errorf("User should have been added to the new group")
	}
}
