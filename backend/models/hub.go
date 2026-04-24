package models

type Progress struct {
	Wpm      float64
	Progress uint8
}

type LobbyResponse struct {
	LobbyId  string
	data     Data
	Progress []Progress
}
