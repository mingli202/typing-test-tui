package data_provider

import (
	"encoding/json"
	"log"
	"math/rand/v2"
	"os"
	"tui/backend/models"
)

type DataProvider struct {
	repository []models.Data
}

// Reads from ../assets/english.json and returns a new provider
// If err != nil, return the defaul provider (has no data)
func NewDataProvider() (DataProvider, error) {
	filepath := "../assets/english.json"

	quotesBytes, err := os.ReadFile(filepath)

	if err != nil {
		log.Printf("Could not load from %v: %v\n", filepath, err)
		return defaultProvider(), err
	}

	var repository []models.Data

	if err := json.Unmarshal(quotesBytes, &repository); err != nil {
		log.Printf("Could no decode into Data: %v", err)
		return defaultProvider(), err

	}

	return DataProvider{repository}, nil
}

// Returns a default provider that contains no data
func defaultProvider() DataProvider {
	return DataProvider{repository: []models.Data{}}
}

// Gets a random data from its repository of quotes
// If repository is empty, a default Data is returned
func (provider *DataProvider) NewData() (models.Data, error) {
	if len(provider.repository) == 0 {
		return models.Data{
			Text:   "No quotes found",
			Source: "No quotes found",
		}, &EmptyRepositoryError{}
	}

	randomIndex := rand.IntN(len(provider.repository))

	return provider.repository[randomIndex], nil
}
