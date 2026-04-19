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

// Returns a default provider that contains no data
func default_provider() DataProvider {
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

	random_index := rand.IntN(len(provider.repository))

	return provider.repository[random_index], nil
}
