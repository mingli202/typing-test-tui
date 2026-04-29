package models

type Progress struct {
	// The current wpm of the user, calculated by the tui client
	Wpm float64
	// At which character the user is at
	Progress uint8
}

type LobbyInfo struct {
	LobbyId string
	Data    Data
	Players map[string]PlayerInfo
}

type PlayerInfo struct {
	IsLeader bool
}
