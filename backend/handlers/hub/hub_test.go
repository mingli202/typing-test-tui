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
	hub := New()

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
