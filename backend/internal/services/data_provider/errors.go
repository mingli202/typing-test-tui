package data_provider

type EmptyRepositoryError struct{}

func (m *EmptyRepositoryError) Error() string {
	return "Provider repository is empty"
}
