package models

type Message interface {
	ToMsg() (string, error)
}
