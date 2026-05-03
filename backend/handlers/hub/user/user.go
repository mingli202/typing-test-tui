package user

import (
	"sync"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

type User struct {
	mu         sync.Mutex
	conn       *websocket.Conn
	ch         chan []byte
	id         string
	totalWpm   float64
	gamePlayed int
	GroupId    *string
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
	user.mu.Unlock()

	go func() {
		for p := range user.ch {
			if user.conn == nil {
				continue
			}

			if err := user.conn.WriteMessage(websocket.TextMessage, p); err != nil {
				user.mu.Lock()
				user.conn = nil
				user.mu.Unlock()
			}
		}
	}()
}

// Helper method to send a string of message
func (user *User) SendMsg(msg string) {
	user.mu.Lock()
	defer user.mu.Unlock()

	if user.ch != nil {
		select {
		case user.ch <- []byte(msg):
		default:
		}
	}
}

// Close websocket connection and closes the ch
func (user *User) Cleanup() {
	user.mu.Lock()
	defer user.mu.Unlock()

	if user.conn != nil {
		user.conn.Close()
	}
	if user.ch != nil {
		close(user.ch)
		user.ch = nil
	}
}
