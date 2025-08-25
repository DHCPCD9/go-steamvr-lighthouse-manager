package main

import (
	_ "embed"
)

//go:embed VERSION_FLAGS
var VERSION_FLAGS string

type UpdateName struct {
	Available bool   `json:"available"`
	Version   string `json:"version"`
	Branch    string `json:"branch"`
}
