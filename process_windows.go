//go:build windows
// +build windows

package main

import (
	"fmt"

	"golang.org/x/sys/windows"
)

const processEntrySize = 568

func processID(name string) (uint32, error) {
	h, e := windows.CreateToolhelp32Snapshot(windows.TH32CS_SNAPPROCESS, 0)
	if e != nil {
		return 0, e
	}
	p := windows.ProcessEntry32{Size: processEntrySize}
	for {
		e := windows.Process32Next(h, &p)
		if e != nil {
			break
		}

		if windows.UTF16ToString(p.ExeFile[:]) == name {
			return p.ProcessID, nil
		}
	}
	return 0, fmt.Errorf("%q not found", name)
}

func isProcRunning(name string) (bool, error) { // I don't know if there is any better method to do it

	_, err := processID(name)

	if err != nil {
		return false, nil
	}

	return true, nil
}
