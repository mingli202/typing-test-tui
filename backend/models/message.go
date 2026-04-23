package models

type ReadMessage struct {
	Type    string
	Payload string
}

type JoinGroup struct {
	Id string
}

type LeaveGroup struct {
	Id string
}

type WriteMessage struct {
	Type    string
	Payload string
}

type NewGroupResponse struct {
	Id string
}

type JoinResponseGroup struct {
	Success bool
}

type LeaveGroupResponse struct {
	Success bool
}
