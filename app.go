package main

import (
	"context"
	"log"
	"os"
	"runtime"
	"slices"
	"strings"

	_ "embed"

	"tinygo.org/x/bluetooth"
)

var adapter = bluetooth.DefaultAdapter
var knownDeviceNames []string = []string{}
var baseStationsConnected map[string]*BaseStation = make(map[string]*BaseStation)
var config Configuration = GetConfiguration()

//go:embed VERSION
var version string

// App struct
type App struct {
	ctx                   context.Context
	bluetoothInitFinished bool
}

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{
		bluetoothInitFinished: false,
	}
}

// startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
}

func (a *App) GetFoundBaseStations() map[string]*BaseStation {
	return baseStationsConnected
}

func (a *App) GetConfiguration() Configuration {
	return config
}

func (a *App) GetVersion() string {
	return version
}

func (a *App) ForceUpdate() {
	ForceUpdate()
}

func (a *App) ToggleSteamVRManagement() Configuration {
	config.IsSteamVRManaged = !config.IsSteamVRManaged

	config.Save()

	return config
}

func (a *App) InitBluetooth() bool {

	if a.bluetoothInitFinished {
		return true
	}

	if err := adapter.Enable(); err != nil {
		return false
	}

	go adapter.Scan(func(a *bluetooth.Adapter, sr bluetooth.ScanResult) {
		go ScanCallback(a, sr)
	})

	a.bluetoothInitFinished = true
	return true
}

func ScanCallback(a *bluetooth.Adapter, sr bluetooth.ScanResult) {

	if slices.Contains(knownDeviceNames, sr.LocalName()) {
		return
	}

	knownDeviceNames = append(knownDeviceNames, sr.LocalName())

	if !strings.HasPrefix(sr.LocalName(), "LHB-") {
		return
	}

	log.Println("Possible V2 lighthouse found, trying to connect and discover it")

	conn, err := a.Connect(sr.Address, bluetooth.ConnectionParams{})

	if err != nil {
		log.Printf("Failed to connect to bluetooth device: %s\n", sr.LocalName())
		return
	}

	bs := InitBaseStation(&conn, a)

	if !bs.IsValidLighthouse {
		return
	}

	bs.ScanCharacteristics()

	bs.Name = sr.LocalName()
	bs.Channel = int(bs.GetChannel())
	bs.PowerState = int(bs.GetPowerState())

	baseStationsConnected[sr.LocalName()] = bs
	defer conn.Disconnect()

	if config.IsSteamVRManaged && runtime.GOOS == "windows" {

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
	bs := baseStationsConnected[baseStationMac]

	if bs == nil {
		return "Unknown base station"
	}

	switch status {
	case "standingby":
		bs.SetPowerState(0x02)
		baseStationsConnected[bs.Name].PowerState = 0x02
	case "sleep":
		bs.SetPowerState(0x00)
		baseStationsConnected[bs.Name].PowerState = 0x00
	case "awake":
		bs.SetPowerState(0x01)
		baseStationsConnected[bs.Name].PowerState = 0x01
	default:
		return "unknown status"
	}

	return "ok"
}

func (a *App) ChangeBaseStationChannel(baseStationMac string, channel int) string {
	bs := baseStationsConnected[baseStationMac]

	if bs == nil {
		return "Unknown base station"
	}

	for _, v := range baseStationsConnected {
		if v.Channel == channel {
			return "error: This channel conflicts with another base station"
		}
	}

	if channel < 1 || channel > 16 {
		return "error: Channel exceeds limit"
	}

	bs.SetChannel(channel)
	bs.Channel = channel

	return "ok"
}

func (a *App) IdentitifyBaseStation(baseStationMac string) string {
	bs := baseStationsConnected[baseStationMac]

	if bs == nil {
		return "Unknown base station"
	}

	bs.Identitfy()

	return "ok"
}

func (a *App) Shutdown() {
	for _, bs := range baseStationsConnected {
		bs.p.Disconnect()
	}

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
	for _, c := range baseStationsConnected {

		if c.PowerState == BS_POWERSTATE_AWAKE {
			continue
		}
		c.SetPowerState(0x01)
		c.PowerState = 0x01
	}
}

func (a *App) SleepAllBaseStations() {
	for _, c := range baseStationsConnected {
		if c.PowerState != BS_POWERSTATE_AWAKE {
			continue
		}

		c.SetPowerState(0x01)
		c.SetPowerState(0x00)
		c.PowerState = 0x00
	}
}
