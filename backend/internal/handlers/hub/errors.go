package hub

import "fmt"

type FunctionNotFoundError struct {
	Fn string
}

func (err FunctionNotFoundError) Error() string {
	return fmt.Sprintf("Function not found: %v", err.Fn)
}
