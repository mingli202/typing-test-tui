package models

import (
	"encoding/json"
	"fmt"
	"strconv"
)

type LobbyInfo struct {
	LobbyId string `json:"lobby_id"`
	Data    Data   `json:"data"`
}

type PlayerInfo struct {
	IsLeader bool `json:"is_leader"`
	// The current wpm of the user, calculated by the tui client
	Wpm float64 `json:"wpm"`
	// At which character the user is at
	ProgressPercent uint8 `json:"progress_percent"`
}

type PlayerInfoSnapshot struct {
	LobbyId string                `json:"lobby_id"`
	Version uint64                `json:"version"`
	Players map[string]PlayerInfo `json:"players"`
}

type NewGame struct {
	Data        Data               `json:"data"`
	PlayersInfo PlayerInfoSnapshot `json:"players_info"`
}

type EndGame struct {
	FinalPlayersInfo PlayerInfoSnapshot
}

type ErrorMessage struct {
	Err error
}

type UserIdMessage struct {
	UserId string
}

type LeaveGroupMessage struct {
	Success bool
}

type CountdownMessage struct {
	Countdown int
}

func (lobbyInfo LobbyInfo) ToMsg() (string, error) {
	lobbyInfoStr, err := json.Marshal(lobbyInfo)

	if err != nil {
		return "", err
	}

	return "LobbyInfo " + string(lobbyInfoStr), nil
}

func (newGame NewGame) ToMsg() (string, error) {
	p, err := json.Marshal(newGame)

	if err != nil {
		return "", err
	}

	return "NewGame " + string(p), nil
}

func (endGame EndGame) ToMsg() (string, error) {
	playerInfo, err := endGame.FinalPlayersInfo.ToMsg()

	if err != nil {
		return "", err
	}

	return "EndGame " + playerInfo, nil
}

func (err ErrorMessage) ToMsg() (string, error) {
	return fmt.Sprintf("Error %s", err.Err.Error()), nil
}

func (userId UserIdMessage) ToMsg() (string, error) {
	return "UserId " + userId.UserId, nil
}

func (leaveGroupMsg LeaveGroupMessage) ToMsg() (string, error) {
	return "LeaveGroup " + strconv.FormatBool(leaveGroupMsg.Success), nil
}

func (playerInfoSnapshot PlayerInfoSnapshot) ToMsg() (string, error) {
	playerInfoBytes, err := json.Marshal(playerInfoSnapshot)

	if err != nil {
		return "", err
	}

	return fmt.Sprintf("PlayersInfo %v", string(playerInfoBytes)), nil
}

func (countdown CountdownMessage) ToMsg() (string, error) {
	return "Countdown " + strconv.Itoa(countdown.Countdown), nil
}
