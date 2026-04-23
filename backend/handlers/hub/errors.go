package hub

import "fmt"

type FunctionNotFoundError struct {
	Fn string
}

func (err FunctionNotFoundError) Error() string {
	return fmt.Sprintf("Function not found: %v", err.Fn)
}

type ErrorMessage struct {
	Msg string
}

func (err ErrorMessage) Error() string {
	return fmt.Sprintf("Error %s", err.Msg)
}
