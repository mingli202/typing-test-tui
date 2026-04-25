package models

type Progress struct {
	UserId string
	// The current wpm of the user, calculated by the tui client
	Wpm float64
	// At which character the user is at
	Progress uint8
}

type LobbyResponse struct {
	LobbyId  string
	data     Data
	Progress []Progress
}
