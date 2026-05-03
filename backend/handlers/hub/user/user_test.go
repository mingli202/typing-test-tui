package user_test

import (
	"sync"
	"testing"
	"tui/backend/handlers/hub/user"
)

func TestNewUser(t *testing.T) {
	user1 := user.NewUser(nil)

	if user1.GroupId != nil {
		t.Fatalf("User should not belong in any group for now")
	}
}

// Issue: sending on a cleaned-up user should not panic due to channel close races.
// If this test fails with a panic, the implementation is still vulnerable.
func TestSendMsgAfterCleanupDoesNotPanic(t *testing.T) {
	u := user.NewUser(nil)
	ch := make(chan []byte)
	u.SetCh(ch)
	u.Cleanup()

	defer func() {
		if recovered := recover(); recovered != nil {
			t.Fatalf("SendMsg panicked after Cleanup: %v", recovered)
		}
	}()

	u.SendMsg("hello")
}

// Issue: concurrent SendMsg/Cleanup should not panic from close/send races.
// Regression expectation: high-contention interleavings remain panic-free.
func TestSendMsgCleanupConcurrentDoesNotPanic(t *testing.T) {
	for i := 0; i < 500; i++ {
		u := user.NewUser(nil)
		u.SetCh(make(chan []byte, 1))

		var wg sync.WaitGroup
		panicCh := make(chan any, 2)

		wg.Add(2)
		go func() {
			defer wg.Done()
			defer func() {
				if r := recover(); r != nil {
					panicCh <- r
				}
			}()
			u.SendMsg("x")
		}()

		go func() {
			defer wg.Done()
			defer func() {
				if r := recover(); r != nil {
					panicCh <- r
				}
			}()
			u.Cleanup()
		}()

		wg.Wait()

		select {
		case p := <-panicCh:
			t.Fatalf("concurrent SendMsg/Cleanup panicked: %v", p)
		default:
		}
	}
}
