package name_provider

import "testing"

func TestNewNameProviderLoadsNames(t *testing.T) {
	provider, err := NewNameProvider()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(provider.nouns) == 0 {
		t.Fatal("expected nouns to be loaded")
	}
	if len(provider.adjectives) == 0 {
		t.Fatal("expected adjectives to be loaded")
	}

	if provider.nouns[0] != "Animal" {
		t.Fatalf("expected first noun to be %q, got %q", "Animal", provider.nouns[0])
	}
	if provider.adjectives[0] != "Little" {
		t.Fatalf("expected first adjective to be %q, got %q", "Little", provider.adjectives[0])
	}
}

func TestNewNameSingleEntryReturnsCombinedName(t *testing.T) {
	provider := NameProvider{
		nouns:      []string{"Guest"},
		adjectives: []string{"Handsome"},
	}

	name, err := provider.NewName()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if name != "Handsome Guest" {
		t.Fatalf("expected name %q, got %q", "Handsome Guest", name)
	}
}

func TestNewNameReturnsErrorWhenRepositoryIsEmpty(t *testing.T) {
	provider := NameProvider{}

	name, err := provider.NewName()
	if err == nil {
		t.Fatal("expected error when repository is empty")
	}

	if name != "Handsome Guest" {
		t.Fatalf("expected fallback name %q, got %q", "Handsome Guest", name)
	}
}

func TestDefaultNameProvider(t *testing.T) {
	provider := defaultNameProvider()

	if len(provider.nouns) != 1 || provider.nouns[0] != "Guest" {
		t.Fatalf("unexpected default nouns: %v", provider.nouns)
	}

	if len(provider.adjectives) != 1 || provider.adjectives[0] != "Handsome" {
		t.Fatalf("unexpected default adjectives: %v", provider.adjectives)
	}
}
