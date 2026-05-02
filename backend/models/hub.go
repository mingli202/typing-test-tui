package models

import "encoding/json"

type LobbyInfo struct {
	LobbyId string
	Data    Data
}

type PlayerInfo struct {
	IsLeader bool
	// The current wpm of the user, calculated by the tui client
	Wpm float64
	// At which character the user is at
	ProgressPercent uint8
}

func (lobbyInfo LobbyInfo) ToMsg() (string, error) {
	lobbyInfoStr, err := json.Marshal(lobbyInfo)

	if err != nil {
		return "", err
	}

	return "LobbyInfo " + string(lobbyInfoStr), nil
}
