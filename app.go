package main

import (
	"context"
	"log"
	"slices"
	"strings"

	"tinygo.org/x/bluetooth"
)

var adapter = bluetooth.DefaultAdapter
var knownDeviceNames []string = []string{}
var baseStationsConnected []BaseStation = []BaseStation{}

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

func (a *App) GetFoundBaseStations() []BaseStation {
	return baseStationsConnected
}

func (a *App) InitBluetooth() bool {

	if a.bluetoothInitFinished {
		return true
	}

	if err := adapter.Enable(); err != nil {
		return false
	}

	adapter.Scan(ScanCallback)

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

	baseStationsConnected = append(baseStationsConnected, *bs)
	defer conn.Disconnect()
}

func GetBaseStation(name string) *BaseStation {
	for _, baseStation := range baseStationsConnected {
		if baseStation.Name == name {
			return &baseStation
		}
	}
	return nil
}

func (a *App) ChangeBaseStationPowerStatus(baseStationMac string, status string) string {
	bs := GetBaseStation(baseStationMac)

	if bs == nil {
		return "Unknown base station"
	}

	switch status {
	case "standingby":
		bs.SetPowerState(0x02)
		bs.PowerState = BS_POWERSTATE_STAND_BY
	case "sleep":
		bs.SetPowerState(0x01)
		bs.SetPowerState(0x00)
		bs.PowerState = BS_POWERSTATE_SLEEP
	case "awake":
		bs.SetPowerState(0x01)
		bs.PowerState = BS_POWERSTATE_AWAKE
	default:
		return "unknown status"
	}

	return "ok"
}

func (a *App) ChangeBaseStationChannel(baseStationMac string, channel int) string {
	bs := GetBaseStation(baseStationMac)

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

	return "ok"
}
