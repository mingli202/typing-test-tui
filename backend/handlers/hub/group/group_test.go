package group

import (
	"encoding/json"
	"math/rand/v2"
	"slices"
	"strings"
	"sync"
	"testing"
	"time"
	"tui/backend/handlers/hub/user"
	"tui/backend/models"
	"tui/backend/services/data_provider"
)

var dataProvider, _ = data_provider.NewDataProvider()

func newGroup() *Group {
	group := NewGroup("asdf", &dataProvider)

	return group
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

	done := make(chan struct{})

	for i := 0; i < 10; i += 1 {
		go func() {
			select {
			case <-done:
				break
			default:
				random := rand.IntN(3)

				if random == 1 {
					u := users[rand.IntN(len(users))]
					gr.RemoveUser(u)
				} else {
					u := user.NewUser(nil)
					gr.AddUser(&u)
				}
			}
		}()
	}

	userCount := 0
	for _, u := range usersSnap {
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

func TestManyExitWhileGameNotStarted(t *testing.T) {
	// Arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.AddUser(&u3)

	time.Sleep(10 * time.Millisecond)

	done := make(chan struct{})
	timer := time.NewTimer(500 * time.Millisecond)

	// Act
	go func() {
		gr.RemoveUser(&u1)
		gr.RemoveUser(&u2)
		gr.RemoveUser(&u3)
		gr.RemoveUser(&u3)
		gr.RemoveUser(&u3)
		done <- struct{}{}
	}()

	// Assert
	select {
	case <-timer.C:
		t.Fatal("Remove user is taking too long")
	case <-done:
		return
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

	time.Sleep(10 * time.Millisecond)

	gr.AddUser(&u3)

	usersIds := gr.GetUserIdsSnapshot()

	if len(usersIds) != 3 {
		t.Fatal("Did not add user")
	}

	time.Sleep(10 * time.Millisecond)

	if gr.RemoveUser(&u1) || gr.RemoveUser(&u2) {
		t.Fatal("Group should not be empty")
	}

	if !gr.RemoveUser(&u3) {
		t.Fatal("Group should be empty now")
	}
}

func TestStartGame(t *testing.T) {
	// arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	go gr.startGame()

	time.Sleep(10 * time.Millisecond)

	// act
	gr.AddUser(&u3)

	// assert
	userIds := gr.GetUserIdsSnapshot()

	if len(userIds) != 3 {
		t.Fatal("Group can still add users")
	}

	if gr.status != Playing {
		t.Fatal("group is not in Playing Status")
	}

	if _, ok := gr.playerInfo[u3.Id()]; ok {
		t.Fatal("user3 should have not have been added to progress")
	}
}

func TestStartGameLeaveInTheMiddleChangeLeader(t *testing.T) {
	// arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.AddUser(&u3)

	go gr.startGame()

	time.Sleep(10 * time.Millisecond)

	// act
	gr.RemoveUser(&u1)

	// assert
	if !gr.playerInfo[u2.Id()].IsLeader && !gr.playerInfo[u3.Id()].IsLeader {
		t.Fatal("New leader not set")
	}
}

func TestStartGameInMiddleOfGame(t *testing.T) {
	// Arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.AddUser(&u3)

	go gr.startGame()

	time.Sleep(10 * time.Millisecond)

	// Act
	err := gr.UserStartGame(&u1)

	// Assert
	if err == nil {
		t.Fatal("How come no error when starting a game that has already started")
	}
}

// Issue: starting a game on an empty group should not panic.
// Regression expectation: method returns an error instead of dereferencing a nil leader.
func TestUserStartGameEmptyGroupDoesNotPanic(t *testing.T) {
	gr := newGroup()
	u := user.NewUser(nil)

	defer func() {
		if recovered := recover(); recovered != nil {
			t.Fatalf("UserStartGame panicked on empty group: %v", recovered)
		}
	}()

	err := gr.UserStartGame(&u)
	if err == nil {
		t.Fatal("expected error when starting game in empty group")
	}
}

func TestIsGameEndedWhenEveryoneLeft(t *testing.T) {
	// Arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.AddUser(&u3)

	go func() {
		gr.startGame()
		gr.endGame()
	}()

	time.Sleep(10 * time.Millisecond)

	// Act
	gr.RemoveUser(&u1)
	gr.RemoveUser(&u2)
	gr.RemoveUser(&u3)

	time.Sleep(10 * time.Millisecond)

	// Assert
	gr.mu.RLock()
	defer gr.mu.RUnlock()
	if gr.status != End {
		t.Fatal("Game not ended when everyone left")
	}
}

func TestNewGameAfterGameEnds(t *testing.T) {
	// Arrange
	ch1 := make(chan []byte)
	ch2 := make(chan []byte)
	ch3 := make(chan []byte)

	done := make(chan struct{})

	var msgMu sync.Mutex
	var msg1 string
	var msg2 string
	var msg3 string

	go func() {
		for {
			select {
			case <-done:
				return
			case p := <-ch1:
				msgMu.Lock()
				msg1 = string(p)
				msgMu.Unlock()
			case p := <-ch2:
				msgMu.Lock()
				msg2 = string(p)
				msgMu.Unlock()
			case p := <-ch3:
				msgMu.Lock()
				msg3 = string(p)
				msgMu.Unlock()
			}
		}
	}()

	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	u1.SetCh(ch1)
	u2.SetCh(ch2)
	u3.SetCh(ch3)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)
	gr.AddUser(&u3)

	go func() {
		gr.startGame()
		gr.endGame()
	}()

	time.Sleep(10 * time.Millisecond)

	gr.mu.RLock()
	gr.end <- struct{}{}
	gr.mu.RUnlock()
	time.Sleep(10 * time.Millisecond)

	gr.mu.RLock()
	initialData := gr.data
	gr.mu.RUnlock()
	// Act
	err := gr.UserStartGame(&u1)

	time.Sleep(10 * time.Millisecond)

	// Assert
	if err != nil {
		t.Fatal(err)
	}

	gr.mu.RLock()
	afterData := gr.data
	gr.mu.RUnlock()

	if initialData == afterData {
		t.Fatal("Did not get new data or data is the same")
	}

	assertNewData := func(msg string) {
		msgMu.Lock()
		defer msgMu.Unlock()
		words := strings.Split(msg, " ")
		cmd := words[0]
		rest := strings.Join(words[1:], " ")

		if cmd != "NewGame" {
			t.Fatalf("Expected NewGame, go %v", cmd)
		}

		var newGame models.NewGame
		err := json.Unmarshal([]byte(rest), &newGame)

		if err != nil {
			t.Fatal(err)
		}

		if newGame.Data != afterData {
			t.Fatal("Data received is different from internal state")
		}

		for _, playerInfo := range newGame.PlayersInfo.Players {
			if playerInfo.ProgressPercent != 0 {
				t.Fatal("ProgressPercent did not get reset")
			}
		}
	}

	assertNewData(msg1)
	assertNewData(msg2)
	assertNewData(msg3)

	done <- struct{}{}
}

func TestPlayerInfoUpdatedWithNewPlayerAfterGameEnds(t *testing.T) {
	// Arrange
	u1 := user.NewUser(nil)
	u2 := user.NewUser(nil)
	u3 := user.NewUser(nil)

	gr := newGroup()

	gr.AddUser(&u1)
	gr.AddUser(&u2)

	go func() {
		gr.startGame()
		gr.endGame()
	}()

	time.Sleep(10 * time.Millisecond)

	// Act
	// joins in middle of game
	gr.AddUser(&u3)

	userIds := gr.GetUserIdsSnapshot()
	if len(userIds) != 3 {
		t.Fatal("user3 did not join")
	}

	playerInfo := gr.getPlayerInfoSnapshot().Players
	if len(playerInfo) != 2 {
		t.Fatalf("playerinfo is not 2, got %v", len(playerInfo))
	}

	// ends the game
	gr.mu.RLock()
	gr.end <- struct{}{}
	gr.mu.RUnlock()

	time.Sleep(10 * time.Millisecond)

	// Assert
	playerInfo = gr.getPlayerInfoSnapshot().Players
	if len(playerInfo) != 3 {
		t.Fatalf("playerinfo is not 3, got %v", len(playerInfo))
	}
}

// Issue: short texts used to compute near-zero/zero game timeout and ended immediately.
// Regression expectation: even with a 1-word text, startGame should not end within 1 second.
func TestStartGameMinimumDurationForShortText(t *testing.T) {
	u := user.NewUser(nil)
	u.SetCh(make(chan []byte, 8))

	gr := newGroup()
	gr.data.Text = "short"
	gr.AddUser(&u)

	done := make(chan struct{})
	go func() {
		gr.startGame()
		close(done)
	}()

	deadline := time.Now().Add(2 * time.Second)
	for {
		gr.mu.RLock()
		status := gr.status
		endCh := gr.end
		gr.mu.RUnlock()

		if status == Playing && endCh != nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("game did not enter playing state in time")
		}
		time.Sleep(10 * time.Millisecond)
	}

	time.Sleep(1100 * time.Millisecond)
	select {
	case <-done:
		t.Fatal("startGame ended too early for short text")
	default:
	}

	gr.mu.RLock()
	endCh := gr.end
	gr.mu.RUnlock()
	endCh <- struct{}{}

	select {
	case <-done:
	case <-time.After(1 * time.Second):
		t.Fatal("startGame did not stop after end signal")
	}
}

// Issue: startGame ticker lifecycle must allow repeated start/stop runs to terminate cleanly.
func TestStartGameRepeatedRunsTerminateCleanly(t *testing.T) {
	for i := 0; i < 20; i++ {
		u := user.NewUser(nil)
		u.SetCh(make(chan []byte, 4))

		gr := newGroup()
		gr.AddUser(&u)

		done := make(chan struct{})
		go func() {
			gr.startGame()
			close(done)
		}()

		deadline := time.Now().Add(2 * time.Second)
		for {
			gr.mu.RLock()
			status := gr.status
			endCh := gr.end
			gr.mu.RUnlock()

			if status == Playing && endCh != nil {
				endCh <- struct{}{}
				break
			}
			if time.Now().After(deadline) {
				t.Fatal("game did not enter playing state in time")
			}
			time.Sleep(5 * time.Millisecond)
		}

		select {
		case <-done:
		case <-time.After(1 * time.Second):
			t.Fatal("startGame did not stop after end signal")
		}
	}
}
