package main

import (
	"context"
	"log"
	"os"
	"path"
	"runtime"
	"slices"
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
var WAKE_UP_CHANNEL = make(chan interface{})
var knownBaseStations = cmap.New[*BaseStation]()

type JsonBaseStation struct {
	Name         string     `json:"name"`
	Channel      int        `json:"channel"`
	PowerState   int        `json:"power_state"`
	LastUpdated  *time.Time `json:"last_updated"`
	Version      int        `json:"version"`
	Status       string     `json:"status"`
	ManagedFlags int        `json:"managed_flags"`
	Id           string     `json:"id"`
}

//go:embed VERSION
var version string

// App struct
type App struct {
	ctx                   context.Context
	bluetoothInitFinished bool
}

func NewApp() *App {
	return &App{
		bluetoothInitFinished: false,
	}
}

func (a *App) UpdateConfigValue(name string, value interface{}) {
	config.UpdateValue(name, value)
}
func (a *App) startup(ctx context.Context) {

	if !strings.Contains(VERSION_FLAGS, "DEBUG") {
		f, err := os.OpenFile(path.Join(GetConfigFolder(), "log.txt"), os.O_RDWR|os.O_CREATE|os.O_TRUNC, 0666)
		if err != nil {
			log.Fatalf("error opening file: %v", err)
		}
		defer f.Close()

		log.SetOutput(f)

	}

	a.ctx = ctx

	go func() {
		for {
			<-WAKE_UP_CHANNEL

			running, _ := isProcRunning("vrserver.exe")

			if !running && config.IsSteamVRManaged {
				wruntime.WindowShow(a.ctx)
			}

			WEBSOCKET_BROADCAST.Broadcast(preparePacket("steamvr.status", map[string]interface{}{
				"status": running,
			}))
		}
	}()

	config = GetConfiguration()
	a.InitBluetooth()
	go a.preloadBaseStations()

	if config.IsSteamVRManaged && runtime.GOOS == "windows" {
		if running, _ := isProcRunning("vrserver.exe"); running {
			wruntime.WindowHide(a.ctx)
		}
	}

	go systray.Run(a.trayReady, TrayExit)

	go StartHttp()

}

func (a *App) CreateGroup(name string) string {
	if config.Groups[name] != nil {
		return "error: Already exists"
	}
	config.CreateGroup(name)

	newGroup := config.Groups[name]

	WEBSOCKET_BROADCAST.Broadcast(preparePacket("groups.created", newGroup))
	return "ok"
}

func (a *App) AddBaseStationToGroup(name string, station string) string {
	if config.Groups[name] == nil {
		return "error: Unknown group"
	}

	bs, found := knownBaseStations.Get(station)

	if !found {
		return "error: Unknown base station"
	}

	baseStation := *bs

	if slices.Contains(config.Groups[name].BaseStationIDs, baseStation.GetId()) {
		return "error: already in collection"
	}

	config.UpdateGroupValue(name, "base_stations", append(config.Groups[name].BaseStationIDs, baseStation.GetId()))
	WEBSOCKET_BROADCAST.Broadcast(preparePacket("groups.lighthouses.added", map[string]interface{}{
		"id":    baseStation.GetId(),
		"group": name,
	}))

	return "ok"
}

func (a *App) RenameGroup(name string, newName string) string {

	log.Println(name, newName)

	log.Println(config.Groups)
	if config.Groups[name] == nil {
		return "error: Unknown group"
	}

	group := *config.Groups[name]
	group.Name = newName
	config.Groups[newName] = &group

	delete(config.Groups, name)

	WEBSOCKET_BROADCAST.Broadcast(preparePacket("group.rename", map[string]interface{}{
		"old_name": name,
		"new_name": newName,
	}))

	config.Save()

	return "ok"
}

func (a *App) UpdateGroupManagedFlags(name string, managed_flags int) string {

	log.Println(config.Groups)
	if config.Groups[name] == nil {
		return "error: Unknown group"
	}

	config.Groups[name].ManagedFlags = managed_flags
	config.Save()

	WEBSOCKET_BROADCAST.Broadcast(preparePacket("group.update.flags", map[string]interface{}{
		"name":  name,
		"flags": managed_flags,
	}))

	return "ok"
}

func (a *App) preloadBaseStations() {

	steamVrRunning, _ := isProcRunning("vrserver.exe")
	for name, baseStation := range config.KnownBaseStations {
		log.Printf("Preload base station: %s %+v\n", name, baseStation)
		preloadedBaseStation := PreloadBaseStation(*baseStation, steamVrRunning && ((baseStation.ManagedFlags&2) > 0))
		knownBaseStations.Set(name, &preloadedBaseStation)
	}
}

func (a *App) ForgetBaseStation(name string) {
	station, found := knownBaseStations.Get(name)

	if !found {
		return
	}

	bs := *station
	bs.Disconnect()

	config.ForgetBaseStation(name)

	knownBaseStations.Remove(name)
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

func (a *App) GetFoundBaseStations() map[string]JsonBaseStation {
	var result = make(map[string]JsonBaseStation)
	for name, v := range knownBaseStations.Items() {

		if v == nil {
			log.Printf("Base station %s in nil\n", name)
			continue
		}

		bs := *v

		configBaseStation := config.KnownBaseStations[name]

		managed := 0

		if configBaseStation != nil {
			managed = configBaseStation.ManagedFlags
		}

		result[bs.GetId()] = JsonBaseStation{
			Name:         bs.GetName(),
			Channel:      bs.GetChannel(),
			PowerState:   bs.GetPowerState(),
			LastUpdated:  nil,
			Version:      bs.GetVersion(),
			Status:       bs.GetStatus(),
			ManagedFlags: managed,
			Id:           bs.GetId(),
		}

	}

	return result
}

func (a *App) UpdateBaseStationParam(name string, param string, value interface{}) {

	bs, found := knownBaseStations.Get(name)

	if !found {
		return
	}

	config.UpdateBaseStationValue(name, param, value)

	baseStation := *bs

	if param == "nickname" {
		baseStation.SetName(value.(string))
	}
}

func (a *App) GetConfiguration() *Configuration {
	return config
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
	config.IsSteamVRManaged = !config.IsSteamVRManaged

	config.Save()

	return config
}

func (a *App) ToggleTray() Configuration {
	config.AllowTray = !config.AllowTray

	config.Save()

	return *config
}

func (a *App) InitBluetooth() bool {

	if a.bluetoothInitFinished {
		return true
	}

	adapter.SetConnectHandler(func(device bluetooth.Device, connected bool) {
		log.Printf("Connection: %s; connected=%+v\n", device.Address, connected)
	})

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

	_, found := knownBaseStations.Get(sr.LocalName())

	// var baseStation BaseStation
	if !found {
		knownBaseStations.Set(sr.LocalName(), nil)
		//Base station not found, we need to store it in out config then

		conn, err := a.Connect(sr.Address, bluetooth.ConnectionParams{})

		if err != nil {
			log.Printf("Failed to connect to bluetooth device: %s\n", sr.LocalName())

			return
		}

		bs, ok := InitBaseStation(&conn, a, sr.LocalName())

		if !ok {
			knownBaseStations.Remove(sr.LocalName())
			return
		}
		defer conn.Disconnect()

		knownBaseStations.Set(sr.LocalName(), &bs)

		//Saving base station
		config.SaveBaseStation(&bs)

		log.Println("New base station discovered and saved")

		if config.IsSteamVRManaged && runtime.GOOS == "windows" {

			running, err := isProcRunning("vrserver.exe")

			if err != nil {
				//whoops
				return
			}
			//Powering on base station
			if running {
				bs.SetPowerState(0x01)
			}
		}

		return
	}
}

func (a *App) ChangeBaseStationPowerStatus(baseStationMac string, status string) string {
	baseStation, found := knownBaseStations.Get(baseStationMac)

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
	baseStation, found := knownBaseStations.Get(baseStationMac)

	if !found {
		return "Unknown base station"
	}

	for _, v := range knownBaseStations.Items() {
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
	baseStation, found := knownBaseStations.Get(baseStationMac)

	if !found {
		return "Unknown base station"
	}

	bs := *baseStation
	bs.Identitfy()

	return "ok"
}

func (a *App) Shutdown() {
	for _, bs := range knownBaseStations.Items() {

		if bs != nil {
			(*bs).Disconnect()
		}
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

	running, err := isProcRunning("vrserver.exe")

	if err != nil {
		log.Println("Failed to obtain process.")
		return false
	}

	return running
}

func (a *App) WakeUpAllBaseStations() {
	for _, c := range knownBaseStations.Items() {

		bs := *c
		if bs.GetPowerState() == BS_POWERSTATE_AWAKE {
			continue
		}
		bs.SetPowerState(0x01)
	}
}

func (a *App) SleepAllBaseStations() {
	for _, c := range knownBaseStations.Items() {
		bs := *c

		if bs.GetPowerState() != BS_POWERSTATE_AWAKE && bs.GetPowerState() != BS_POWERSTATE_AWAKE_2 {
			continue
		}

		bs.SetPowerState(0x01)
		bs.SetPowerState(0x00)
	}
}
