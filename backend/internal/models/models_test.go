package models

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

func TestLobbyInfoToMsg(t *testing.T) {
	lobbyInfo := LobbyInfo{
		LobbyId: "asdf",
		Data: Data{
			Text:   "qwer",
			Source: "zxcv",
		},
	}

	msg := mustMsg(t, lobbyInfo)
	assertMsg(t, msg, `LobbyInfo {"lobby_id":"asdf","data":{"text":"qwer","source":"zxcv"}}`)

	var payload LobbyInfo
	unmarshalPayload(t, msg, "LobbyInfo", &payload)

	if payload != lobbyInfo {
		t.Fatalf("expected payload %+v, got %+v", lobbyInfo, payload)
	}
}

func TestNewGameToMsg(t *testing.T) {
	newGame := NewGame{
		Data: Data{
			Text:   "qwer",
			Source: "zxcv",
		},
		PlayersInfo: testPlayersInfoSnapshot(),
	}

	msg := mustMsg(t, newGame)
	assertMsg(t, msg, `NewGame {"data":{"text":"qwer","source":"zxcv"},"players_info":{"lobby_id":"asdf","version":1,"players":{"player-1":{"name":"","is_leader":true,"wpm":42.5,"progress_percent":75},"player-2":{"name":"","is_leader":false,"wpm":10.1,"progress_percent":100}}}}`)

	var payload NewGame
	unmarshalPayload(t, msg, "NewGame", &payload)
	assertPlayersInfoSnapshot(t, payload.PlayersInfo, newGame.PlayersInfo)
}

func TestEndGameToMsg(t *testing.T) {
	endGame := EndGame{
		FinalPlayersInfo: testPlayersInfoSnapshot(),
	}

	msg := mustMsg(t, endGame)
	assertMsg(t, msg, `EndGame {"lobby_id":"asdf","version":1,"players":{"player-1":{"name":"","is_leader":true,"wpm":42.5,"progress_percent":75},"player-2":{"name":"","is_leader":false,"wpm":10.1,"progress_percent":100}}}`)

	var payload PlayersInfoSnapshot
	unmarshalPayload(t, msg, "EndGame", &payload)
	assertPlayersInfoSnapshot(t, payload, endGame.FinalPlayersInfo)
}

func TestErrorMessageToMsg(t *testing.T) {
	msg := mustMsg(t, ErrorMessage{Err: errors.New("something went wrong")})

	assertMsg(t, msg, "Error something went wrong")
}

func TestUserIdMessageToMsg(t *testing.T) {
	msg := mustMsg(t, UserIdMessage{UserId: "user-123"})

	assertMsg(t, msg, "UserId user-123")
}

func TestLeaveGroupMessageToMsg(t *testing.T) {
	tests := []struct {
		name     string
		message  LeaveGroupMessage
		expected string
	}{
		{
			name:     "succeeded",
			message:  LeaveGroupMessage{DidSucceed: true},
			expected: "LeaveGroup true",
		},
		{
			name:     "failed",
			message:  LeaveGroupMessage{DidSucceed: false},
			expected: "LeaveGroup false",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := mustMsg(t, tt.message)

			assertMsg(t, msg, tt.expected)
		})
	}
}

func TestPlayersInfoSnapshotToMsg(t *testing.T) {
	playersInfo := testPlayersInfoSnapshot()

	msg := mustMsg(t, playersInfo)
	assertMsg(t, msg, `PlayersInfo {"lobby_id":"asdf","version":1,"players":{"player-1":{"name":"","is_leader":true,"wpm":42.5,"progress_percent":75},"player-2":{"name":"","is_leader":false,"wpm":10.1,"progress_percent":100}}}`)

	var payload PlayersInfoSnapshot
	unmarshalPayload(t, msg, "PlayersInfo", &payload)
	assertPlayersInfoSnapshot(t, payload, playersInfo)
}

func TestCountdownMessageToMsg(t *testing.T) {
	msg := mustMsg(t, CountdownMessage{Countdown: 3})

	assertMsg(t, msg, "Countdown 3")
}

func mustMsg(t *testing.T, message Message) string {
	t.Helper()

	msg, err := message.ToMsg()
	if err != nil {
		t.Fatal(err)
	}

	return msg
}

func assertMsg(t *testing.T, actual, expected string) {
	t.Helper()

	if actual != expected {
		t.Fatalf("expected %q, got %q", expected, actual)
	}
}

func unmarshalPayload(t *testing.T, msg, command string, target any) {
	t.Helper()

	prefix := command + " "
	if !strings.HasPrefix(msg, prefix) {
		t.Fatalf("expected %q prefix, got %q", prefix, msg)
	}

	if err := json.Unmarshal([]byte(strings.TrimPrefix(msg, prefix)), target); err != nil {
		t.Fatalf("expected valid %s JSON payload: %v", command, err)
	}
}

func testPlayersInfoSnapshot() PlayersInfoSnapshot {
	return PlayersInfoSnapshot{
		LobbyId: "asdf",
		Version: 1,
		Players: map[string]PlayerInfo{
			"player-1": {
				IsLeader:        true,
				Wpm:             42.5,
				ProgressPercent: 75,
			},
			"player-2": {
				IsLeader:        false,
				Wpm:             10.1,
				ProgressPercent: 100,
			},
		},
	}
}

func assertPlayersInfoSnapshot(t *testing.T, actual, expected PlayersInfoSnapshot) {
	t.Helper()

	if actual.Version != expected.Version {
		t.Fatalf("expected version %d, got %d", expected.Version, actual.Version)
	}
	if actual.LobbyId != expected.LobbyId {
		t.Fatalf("expected lobby id %q, got %q", expected.LobbyId, actual.LobbyId)
	}

	if len(actual.Players) != len(expected.Players) {
		t.Fatalf("expected %d players, got %d", len(expected.Players), len(actual.Players))
	}

	for userId, expectedPlayer := range expected.Players {
		actualPlayer, ok := actual.Players[userId]
		if !ok {
			t.Fatalf("expected player %q to be present", userId)
		}

		if actualPlayer != expectedPlayer {
			t.Fatalf("expected player %q to be %+v, got %+v", userId, expectedPlayer, actualPlayer)
		}
	}
}
