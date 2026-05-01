package group

import (
	"log"
	"math/rand/v2"
	"slices"
	"testing"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/services/data_provider"
)

var dataProvider, _ = data_provider.NewDataProvider()

func newGroup() *Group {
	data, _ := dataProvider.NewData()
	group := NewGroup("asdf", data)

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

	if gr.leaderId != nil {
		t.Fatal("leader id is not nil")
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
}

func TestGroupAddDuplicateUser(t *testing.T) {
	u := user.NewUser(nil)
	gr := newGroup()

	gr.AddUser(&u)
	gr.AddUser(&u)

	users := gr.GetUsersSnapshot()

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

func TestLeaderWhenAddUser(t *testing.T) {
	u := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u)

	if gr.leaderId == nil || *gr.leaderId != u.Id() {
		t.Fatal("group leader id is not set to the user id")
	}
}

func TestLeaderWhenAddMultipleUser(t *testing.T) {
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	if gr.leaderId == nil || *gr.leaderId != u1.Id() {
		t.Fatal("leader should have have stayed at the first user that joins")
	}
}

func TestLeaderWhenRemoveUser1(t *testing.T) {
	u := user.NewUser(nil)
	gr := newGroup()

	gr.AddUser(&u)
	gr.RemoveUser(&u)

	if gr.leaderId != nil {
		t.Fatal("there should be no more available leader")
	}
}

func TestLeaderWhenRemoveUser2(t *testing.T) {
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.RemoveUser(&u1)

	if *gr.leaderId != u2.Id() {
		t.Fatal("A new leader should be set")
	}
}

func TestCountDown(t *testing.T) {
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	go gr.countDown()

	time.Sleep(1 * time.Second)

	gr.AddUser(&u3)

	usersIds := gr.GetUserIdsSnapshot()

	if len(usersIds) != 3 {
		t.Fatal("Did not add user")
	}

	time.Sleep(1 * time.Second)

	if gr.RemoveUser(&u1) || gr.RemoveUser(&u2) {
		t.Fatal("Group should not be empty")
	}

	if !gr.RemoveUser(&u3) {
		t.Fatal("Group should be empty now")
	}
}

func TestStartGame(t *testing.T) {
	ch1 := make(chan []byte)
	ch2 := make(chan []byte)

	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	u1.SetCh(ch1)
	u2.SetCh(ch2)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	go gr.startGame()

	time.Sleep(1 * time.Millisecond)
	gr.AddUser(&u3)

	userIds := gr.GetUserIdsSnapshot()

	if len(userIds) != 3 {
		t.Fatal("Group can still add users")
	}

	if _, ok := gr.playerInfo[u3.Id()]; ok {
		t.Fatal("user3 should have not have been added to progress")
	}
}
