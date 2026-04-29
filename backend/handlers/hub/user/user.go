package user

import (
	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

type User struct {
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
	user := User{
		conn:    conn,
		ch:      make(chan []byte, 10),
		id:      uuid.NewString(),
		GroupId: nil,
	}

	return user
}

// Init the buffered channel to listen for write messages
func (user *User) InitWriteMessageCh() {
	for p := range user.ch {
		if user.conn == nil {
			continue
		}

		if err := user.conn.WriteMessage(websocket.TextMessage, p); err != nil {
			return
		}
	}
}

// Helper method to send a string of message
func (user *User) SendMsg(msg string) {
	user.ch <- []byte(msg)
}

// Close websocket connection and closes the ch
func (user *User) Cleanup() {
	if user.conn != nil {
		user.conn.Close()
	}
	close(user.ch)
}
