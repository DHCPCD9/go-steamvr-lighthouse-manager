// +build !windows
package main

import (
)

func isProcRunning(names ...string) (bool, error) {
	return false, nil
}