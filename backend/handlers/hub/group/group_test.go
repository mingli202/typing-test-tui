package group_test

import (
	"testing"
)

func TestNewGroup(t *testing.T) {
	data, _ := dataProvider.NewData()
	group := newGroup("asdf", data)

	if group.users == nil {
		t.Error("group.user should not be nil")
	}
}

func TestGroupAddUser(t *testing.T) {
}
