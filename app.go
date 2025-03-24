package main

import (
	"context"
	"log"
	"os"
	"runtime"
	"strings"
	"time"

	_ "embed"

	"github.com/gen2brain/beeep"
	"github.com/getlantern/systray"
	cmap "github.com/orcaman/concurrent-map/v2"
	wruntime "github.com/wailsapp/wails/v2/pkg/runtime"
	"tinygo.org/x/bluetooth"
)

var adapter = bluetooth.DefaultAdapter

var baseStationsConnected = cmap.New[*BaseStation]()

type JsonBaseStations struct {
	Name        string    `json:"name"`
	Channel     int       `json:"channel"`
	PowerState  int       `json:"power_state"`
	LastUpdated time.Time `json:"last_updated"`
	Version     int       `json:"version"`
}

//go:embed VERSION
var version string

// App struct
type App struct {
	ctx                   context.Context
	bluetoothInitFinished bool
	config                *Configuration
}

func NewApp() *App {
	return &App{
		bluetoothInitFinished: false,
	}
}

func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	a.config = GetConfiguration()

	systray.Run(a.trayReady, TrayExit)

}

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

func (a *App) Notify(title string, text string) {
	beeep.Notify(title, text, "")
}
func (a *App) GetFoundBaseStations() map[string]JsonBaseStations {

	var result = make(map[string]JsonBaseStations)
	for _, v := range baseStationsConnected.Items() {

		if v == nil {
			continue
		}

		bs := *v
		result[bs.GetName()] = JsonBaseStations{
			Name:        bs.GetName(),
			Channel:     bs.GetChannel(),
			PowerState:  bs.GetPowerState(),
			LastUpdated: time.Now(),
			Version:     bs.GetVersion(),
		}

	}

	return result
}

func (a *App) GetConfiguration() *Configuration {
	return a.config
}

func (a *App) GetVersion() string {
	return version
}

func (a *App) ForceUpdate() {
	ForceUpdate()
}

func (a *App) IsUpdatingSupported() bool {
	return IsUpdatingSupported()
}

func (a *App) ToggleSteamVRManagement() *Configuration {
	a.config.IsSteamVRManaged = !a.config.IsSteamVRManaged

	a.config.Save()

	return a.config
}

func (a *App) ToggleTray() Configuration {
	a.config.AllowTray = !a.config.AllowTray

	a.config.Save()

	return *a.config
}

func (a *App) InitBluetooth() bool {

	if a.bluetoothInitFinished {
		return true
	}

	if err := adapter.Enable(); err != nil {
		return false
	}

	go adapter.Scan(func(adapter *bluetooth.Adapter, sr bluetooth.ScanResult) {
		go ScanCallback(a, adapter, sr)
	})

	a.bluetoothInitFinished = true
	return true
}

func ScanCallback(app *App, a *bluetooth.Adapter, sr bluetooth.ScanResult) {

	if !strings.HasPrefix(sr.LocalName(), "LHB-") {
		return
	}

	if baseStationsConnected.Has(sr.LocalName()) {
		return
	}

	log.Println("Possible V2 lighthouse found, trying to connect and discover it")

	baseStationsConnected.Set(sr.LocalName(), nil)
	conn, err := a.Connect(sr.Address, bluetooth.ConnectionParams{})

	if err != nil {
		log.Printf("Failed to connect to bluetooth device: %s\n", sr.LocalName())
		baseStationsConnected.Remove(sr.LocalName())

		return
	}

	bs := InitBaseStation(&conn, a, sr.LocalName())

	baseStationsConnected.Set(sr.LocalName(), &bs)
	defer conn.Disconnect()

	if app.config.IsSteamVRManaged && runtime.GOOS == "windows" {

		running, err := isProcRunning("vrserver")

		if err != nil {
			//whoops
			return
		}
		//Powering on base station
		if running {
			bs.SetPowerState(0x01)
		}
	}
}

func (a *App) ChangeBaseStationPowerStatus(baseStationMac string, status string) string {
	baseStation, found := baseStationsConnected.Get(baseStationMac)

	if !found {
		return "Unknown base station"
	}

	bs := *baseStation
	switch status {
	case "standingby":
		bs.SetPowerState(0x02)
	case "sleep":
		bs.SetPowerState(0x00)
	case "awake":
		bs.SetPowerState(0x01)
	default:
		return "unknown status"
	}

	return "ok"
}

func (a *App) ChangeBaseStationChannel(baseStationMac string, channel int) string {
	baseStation, found := baseStationsConnected.Get(baseStationMac)

	if !found {
		return "Unknown base station"
	}

	for _, v := range baseStationsConnected.Items() {
		bs := *v
		if bs.GetChannel() == channel {
			return "error: This channel conflicts with another base station"
		}
	}

	if channel < 1 || channel > 16 {
		return "error: Channel exceeds limit"
	}

	bs := *baseStation
	bs.SetChannel(channel)
	bs.SetPowerState(byte(BS_POWERSTATE_AWAKE))

	return "ok"
}

func (a *App) IdentitifyBaseStation(baseStationMac string) string {
	baseStation, found := baseStationsConnected.Get(baseStationMac)

	if !found {
		return "Unknown base station"
	}

	bs := *baseStation
	bs.Identitfy()

	return "ok"
}

func (a *App) Shutdown() {
	for _, bs := range baseStationsConnected.Items() {
		(*bs).Disconnect()
	}

	systray.Quit()
	os.Exit(0)
}

func (a *App) IsSteamVRConnectivityAvailable() bool {
	return runtime.GOOS == "windows" // No linux & macos support for now
}

func (a *App) IsSteamVRConnected() bool {
	if runtime.GOOS != "windows" {
		return false
	}

	running, err := isProcRunning("vrserver")

	if err != nil {
		log.Println("Failed to obtain process.")
		return false
	}

	return running
}

func (a *App) WakeUpAllBaseStations() {
	for _, c := range baseStationsConnected.Items() {

		bs := *c
		if bs.GetPowerState() == BS_POWERSTATE_AWAKE {
			continue
		}
		bs.SetPowerState(0x01)
	}
}

func (a *App) SleepAllBaseStations() {
	for _, c := range baseStationsConnected.Items() {
		bs := *c

		if bs.GetPowerState() != BS_POWERSTATE_AWAKE && bs.GetPowerState() != BS_POWERSTATE_AWAKE_2 {
			continue
		}

		bs.SetPowerState(0x01)
		bs.SetPowerState(0x00)
	}
}
