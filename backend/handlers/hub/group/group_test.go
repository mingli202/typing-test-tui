package group_test

import (
	"testing"
	"tui/backend/handlers/hub/group"
	"tui/backend/services/data_provider"
)

var dataProvider, _ = data_provider.NewDataProvider()

func TestNewGroup(t *testing.T) {
	data, _ := dataProvider.NewData()
	group := group.NewGroup("asdf", data)

	users := group.GetUsersSnapshot()

	if users == nil || len(users) != 1 {
		t.Error("group.users should not be nil")
	}
}

func TestGroupAddUser(t *testing.T) {
}
