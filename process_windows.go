//go:build windows
// +build windows

package main

import (
	"bytes"
	"os/exec"
	"syscall"
)

func isProcRunning(names ...string) (bool, error) { // I don't know if there is any better method to do it
	if len(names) == 0 {
		return false, nil
	}

	cmd := exec.Command("tasklist.exe", "/fo", "csv", "/nh")
	cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
	out, err := cmd.Output()
	if err != nil {
		return false, err
	}

	for _, name := range names {
		if bytes.Contains(out, []byte(name)) {
			return true, nil
		}
	}
	return false, nil
}
