package name_provider

import (
	"encoding/json"
	"fmt"
	"log"
	"math/rand/v2"
	"tui/backend/internal/assets"
	"tui/backend/internal/models"
)

type NameProvider struct {
	nouns      []string
	adjectives []string
}

// Makes a new name provider
func NewNameProvider() (NameProvider, error) {
	var repository models.NamesRepo

	if err := json.Unmarshal(assets.Names, &repository); err != nil {
		log.Printf("Could no decode into Names: %v", err)
		return defaultNameProvider(), nil
	}

	return NameProvider{
		nouns:      repository.Nouns,
		adjectives: repository.Adjectives,
	}, nil
}

// Gets a new name
func (provider *NameProvider) NewName() (string, error) {
	nounsLen := len(provider.nouns)
	adLen := len(provider.adjectives)

	if nounsLen == 0 || adLen == 0 {
		return "Handsome Guest", fmt.Errorf("Oops, no nouns or adjectives")
	}

	randomNounIndex := rand.IntN(nounsLen)
	randomAdIndex := rand.IntN(adLen)

	return provider.adjectives[randomAdIndex] + " " + provider.nouns[randomNounIndex], nil
}

// Returns a default provider in cases of errors
func defaultNameProvider() NameProvider {
	return NameProvider{
		nouns:      []string{"Guest"},
		adjectives: []string{"Handsome"},
	}
}
