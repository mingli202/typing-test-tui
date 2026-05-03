package user

import (
	"sync"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

type User struct {
	mu          sync.Mutex
	conn        *websocket.Conn
	ch          chan []byte
	done        chan struct{}
	cleanupOnce sync.Once
	id          string
	totalWpm    float64
	gamePlayed  int
	GroupId     *string
}

func (u *User) Id() string {
	return u.id
}

func (user *User) AvgWpm() float64 {
	if user.gamePlayed == 0 {
		return 0.0
	}

	return user.totalWpm / float64(user.gamePlayed)
}

// Adds a user to its user repository
// and returns the newly added user
func NewUser(conn *websocket.Conn) User {
	return User{
		conn:    conn,
		done:    make(chan struct{}),
		id:      uuid.NewString(),
		GroupId: nil,
	}
}

// Sets the user's channel
// Used for debugging and testing
func (user *User) SetCh(ch chan []byte) {
	user.ch = ch
}

// Init the buffered channel to listen for write messages
func (user *User) InitWriteMessageCh() {
	user.mu.Lock()
	user.ch = make(chan []byte, 64)
	ch := user.ch
	user.mu.Unlock()

	go func() {
		for {
			select {
			case <-user.done:
				return
			case p := <-ch:
				if user.conn == nil {
					continue
				}

				if err := user.conn.WriteMessage(websocket.TextMessage, p); err != nil {
					user.mu.Lock()
					user.conn = nil
					user.mu.Unlock()
				}
			}
		}
	}()
}

// Helper method to send a string of message
func (user *User) SendMsg(msg string) {
	user.mu.Lock()
	ch := user.ch
	done := user.done
	user.mu.Unlock()

	if ch != nil {
		select {
		case ch <- []byte(msg):
		case <-done:
		}
	}
}

// Close websocket connection and closes the ch
func (user *User) Cleanup() {
	user.cleanupOnce.Do(func() {
		user.mu.Lock()
		conn := user.conn
		user.conn = nil
		user.ch = nil
		user.mu.Unlock()

		if conn != nil {
			conn.Close()
		}

		close(user.done)
	})
}
