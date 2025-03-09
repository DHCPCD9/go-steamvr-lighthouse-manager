//go:build !windows
// +build !windows

package main

func isProcRunning(names ...string) (bool, error) {
	return false, nil
}
