package hub

import "fmt"

type TypeNotFoundError struct {
	Type string
}

func (err TypeNotFoundError) Error() string {
	return fmt.Sprintf("Type not found: %v", err.Type)
}
