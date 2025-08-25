//go:build !windows
// +build !windows

package main

func ForceUpdate() {
	return
}

func IsUpdatingSupported() bool {
	return false
}

func FindUpdates() *UpdateName {
	return nil
}
