package data_provider

import (
	"testing"
	"tui/backend/models"
)

func TestNewProvider(t *testing.T) {
	data_provider, err := NewDataProvider()

	if err != nil {
		t.Errorf("Encounted error: %v", err)
	}

	if len(data_provider.repository) == 0 {
		t.Errorf("unable to load repository")
	}

	first_data := models.Data{
		Text:   "You have the power to heal your life, and you need to know that.",
		Source: "Meditations to Heal Your Life",
	}

	repo_first := data_provider.repository[0]

	if first_data != repo_first {
		t.Errorf("Expected %v; got %v", repo_first, first_data)
	}
}

func TestNewQuote(t *testing.T) {
	data_provider, err := NewDataProvider()

	if err != nil {
		t.Error(err)
	}

	data, err := data_provider.NewData()

	if err != nil {
		t.Error(err)
	}

	t.Log(data)
}
