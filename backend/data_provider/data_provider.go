package data_provider

import (
	"encoding/json"
	"log"
	"os"
	"tui/backend/models"
)

type DataProvider struct {
	repository []models.Data
}

func NewDataProvider() (DataProvider, error) {
	filepath := "../assets/english.json"

	quotes_bytes, err := os.ReadFile(filepath)

	if err != nil {
		log.Printf("Could not load from %v: %v\n", filepath, err)
		return default_provider(), err
	}

	var repository []models.Data

	if err := json.Unmarshal(quotes_bytes, &repository); err != nil {
		log.Printf("Could no decode into Data: %v", err)
		return default_provider(), err

	}

	return DataProvider{repository}, nil
}

func default_provider() DataProvider {
	return DataProvider{repository: []models.Data{}}
}
