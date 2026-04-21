package hub

import (
	"log"
	"math/rand/v2"
	"net/http"
	"sync"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{}

type Hub struct {
	mu     sync.Mutex
	groups map[string][]*websocket.Conn
}

// Makes a new group with the given conn
// Returns the newly created group id
func (hub *Hub) NewGroup(conn *websocket.Conn) string {
	hub.mu.Lock()
	defer hub.mu.Unlock()

	id := newGroupId()
	_, ok := hub.groups[id]

	for ok {
		id = newGroupId()
		_, ok = hub.groups[id]
	}

	hub.groups[id] = make([]*websocket.Conn, 1)
	hub.groups[id] = append(hub.groups[id], conn)

	return id
}

func (hub *Hub) Join(id string, conn *websocket.Conn) {
}

func (hub *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	defer func() {
		conn.Close()
	}()

	for {
		_, p, err := conn.ReadMessage()

		log.Println(string(p))

		if err != nil {
			log.Println(err)
			return
		}
	}
}

func Handler() http.Handler {
	hub := Hub{}

	return &hub
}

func newGroupId() string {
	s := ""

	for i := 0; i < 6; i += 1 {
		randomChar := rand.IntN('z'-'a') + 'a'
		s = s + string(rune(randomChar))
	}

	return s
}
