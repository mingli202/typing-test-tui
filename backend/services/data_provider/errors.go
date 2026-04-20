package data_provider

type EmptyRepositoryError struct{}

func (m *EmptyRepositoryError) Error() string {
	return "Date provider repository is empty"
}
