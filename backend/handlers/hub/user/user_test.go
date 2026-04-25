package user_test

import (
	"testing"
	"tui/backend/handlers/hub/user"
)

func TestNewUser(t *testing.T) {
	user1 := user.NewUser(nil)

	if user1.GroupId != nil {
		t.Fatalf("User should not belong in any group for now")
	}
}
