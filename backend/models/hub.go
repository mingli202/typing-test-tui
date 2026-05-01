package models

import "encoding/json"

type Progress struct {
	// The current wpm of the user, calculated by the tui client
	Wpm float64
	// At which character the user is at
	ProgressPercent uint8
}

type LobbyInfo struct {
	LobbyId string
	Data    Data
}

type PlayerInfo struct {
	IsLeader bool
}

func (lobbyInfo LobbyInfo) ToMsg() (string, error) {
	lobbyInfoStr, err := json.Marshal(lobbyInfo)

	if err != nil {
		return "", err
	}

	return "LobbyInfo " + string(lobbyInfoStr), nil
}
