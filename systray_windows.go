//go:build windows
// +build windows

package main

import (
	"log"

	"github.com/getlantern/systray"
	wruntime "github.com/wailsapp/wails/v2/pkg/runtime"
)

// I REALLY SHOULD NOT DO THAT
func (a *App) trayReady() {

	systray.SetIcon(icon)

	showWindowCh := systray.AddMenuItem("Show", "Show the window")

	systray.AddSeparator()
	wakeUpCh := systray.AddMenuItem("Wake up", "Wakes up all found Lighthouses.")
	sleepCh := systray.AddMenuItem("Sleep", "Puts all found Lighthouses in sleep mode.")
	systray.AddSeparator()

	quitCh := systray.AddMenuItem("Quit", "Quits programm fully")

	go func() {
		for {
			select {
			case <-showWindowCh.ClickedCh:
				wruntime.WindowShow(a.ctx)
				continue
			case <-wakeUpCh.ClickedCh:
				a.WakeUpAllBaseStations()
				continue
			case <-sleepCh.ClickedCh:
				a.SleepAllBaseStations()
				continue
			case <-quitCh.ClickedCh:
				a.Shutdown()
				continue
			}
		}
	}()

}

func TrayExit() {
	log.Println("Tray exit.")
}

func initializeSystray(a *App) {
	systray.Run(a.trayReady, TrayExit)
}

func shutdownSystray() {
	systray.Quit()
}
