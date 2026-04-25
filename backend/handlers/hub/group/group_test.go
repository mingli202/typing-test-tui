package group_test

import (
	"log"
	"math/rand/v2"
	"slices"
	"testing"
	"time"
	"tui/backend/handlers/hub/group"
	"tui/backend/handlers/hub/user"
	"tui/backend/services/data_provider"
)

var dataProvider, _ = data_provider.NewDataProvider()

func newGroup() *group.Group {
	data, _ := dataProvider.NewData()
	group := group.NewGroup("asdf", data)

	return &group
}

func TestNewGroup(t *testing.T) {
	gr := newGroup()

	users := gr.GetUsersSnapshot()

	if users == nil {
		t.Fatal("group.users should not be nil")
	}

	if len(users) != 0 {
		t.Fatal("There should be no users")
	}
}

func TestGroupAddUser(t *testing.T) {
	u := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u)

	users := gr.GetUsersSnapshot()

	if len(users) != 1 || !slices.Contains(users, &u) {
		t.Fatal("It should have added the added user")
	}

	if *u.GroupId != gr.Id() {
		t.Fatal("user group should have been set to the group's id")
	}

	gr.AddUser(&u)

	users = gr.GetUsersSnapshot()

	if len(users) != 1 {
		t.Fatal("Duplicate user tf")
	}
}

func TestRemoverUser(t *testing.T) {
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	isEmpty := gr.RemoveUser(&u2)

	if isEmpty {
		t.Fatal("Group is still not empty")
	}

	users := gr.GetUsersSnapshot()

	if len(users) != 1 {
		t.Fatal("User did not get removed")
	}

	if slices.Contains(users, &u2) {
		t.Fatal("Group removed the wrong user vro")
	}

	if !slices.Contains(users, &u1) {
		t.Fatal("Where tf is the first user")
	}

	isEmpty = gr.RemoveUser(&u1)

	if !isEmpty {
		t.Fatal("There should be no more users in the group")
	}
}

func TestGetUsersSnapshot(t *testing.T) {
	gr := newGroup()

	users := make([]*user.User, 0, 10)

	for i := 0; i < 10; i += 1 {
		u := user.NewUser(nil)
		users = append(users, &u)

		gr.AddUser(&u)
	}

	if len(users) != 10 {
		t.Fatalf("Where are all my users? %v", len(users))
	}

	usersSnap := gr.GetUsersSnapshot()

	userCount := 0

	done := make(chan struct{})

	go func() {
		ticker := time.Tick(time.Millisecond * 8)

		for {
			select {
			case <-done:
				break
			case _ = <-ticker:
				random := rand.IntN(3)

				if random == 1 {
					u := users[rand.IntN(len(users))]
					gr.RemoveUser(u)
					log.Printf("Removing an user %v\n", u.Id())
				} else {
					u := user.NewUser(nil)
					gr.AddUser(&u)
					log.Printf("Adding an user %v\n", u.Id())
				}
			}
		}
	}()

	for _, u := range usersSnap {
		time.Sleep(time.Millisecond * 10)
		if !slices.Contains(users, u) {
			t.Fatal("Who is this user?")
		}
		userCount += 1
	}

	done <- struct{}{}

	if userCount != len(users) {
		t.Fatal("Total user count not equal to original")
	}
}
