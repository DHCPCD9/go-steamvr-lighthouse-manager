package main

import (
	"log"
	"strings"
	"sync/atomic"
	"time"
)

var (
	BS_POWERSTATE_SLEEP    = 0
	BS_POWERSTATE_STAND_BY = 2
	BS_POWERSTATE_AWAKE    = 9
	BS_POWERSTATE_AWAKE_2  = 11 // Some weird things is happening here
)

var LIGHTHOUSE_SERVICE_UUID = "00001523-1212-EFDE-1523-785FEABCD124"

var (
	LIGHTHOUSE_CHARACTERISTIC_MODE      = "00001524-1212-EFDE-1523-785FEABCD124"
	LIGHTHOUSE_CHARACTERISTIC_IDENTITFY = "00008421-1212-EFDE-1523-785FEABCD124"
	LIGHTHOUSE_CHARACTERISTIC_POWER     = "00001525-1212-EFDE-1523-785FEABCD124"
)

// BLE* interfaces abstract away the underlying Bluetooth LE stack so each
// platform can provide its own implementation:
//   - windows/darwin: a thin wrapper around tinygo.org/x/bluetooth (unchanged behavior)
//   - linux: a direct BlueZ D-Bus implementation (see bs_linux.go)
type BLEAdapter interface {
	Enable() error
	Scan(callback func(BLEAdapter, BLEScanResult)) error
	StopScan() error
	Connect(address string, timeout time.Duration) (BLEDevice, error)
}

type BLEScanResult interface {
	LocalName() string
	Address() string
}

type BLEDevice interface {
	Address() string
	Disconnect() error
	DiscoverServices() ([]BLEService, error)
}

type BLEService interface {
	UUID() string
	DiscoverCharacteristics() ([]BLECharacteristic, error)
}

type BLECharacteristic interface {
	UUID() string
	Read() ([]byte, error)
	Write(value []byte) (int, error)
	WriteWithoutResponse(value []byte) (int, error)
	EnableNotifications(callback func(buf []byte)) error
}

// optionalDisconnectWatcher lets a BLEDevice implementation (currently only the
// Linux/BlueZ backend) proactively report an out-of-band disconnect, e.g. the
// base station losing power or walking out of range, instead of only being
// noticed on the next failed read/write.
type optionalDisconnectWatcher interface {
	OnDisconnected(callback func())
}

type BaseStation interface {
	ScanCharacteristics() bool
	GetChannel() int
	SetChannel(channel int)
	GetPowerState() int
	SetPowerState(state byte)
	Identitfy()
	GetName() string
	SetName(string)
	GetVersion() int
	Disconnect()
	GetStatus() string
	GetId() string
	GetMAC() string
	IsOutdated() bool
}

type LighthouseV2 struct {
	modeCharacteristic       BLECharacteristic
	identifyCharacteristic   BLECharacteristic
	powerStateCharacteristic BLECharacteristic
	p                        BLEDevice
	adapter                  BLEAdapter
	service                  BLEService
	Name                     string
	Id                       string
	CachedPowerState         int
	CachedChannel            int
	ValidLighthouse          bool
	Status                   string
	mac                      string
	updateAvailable          bool
	reconnecting             int32
}

func PreloadBaseStation(config BaseStationConfiguration, wakeUp bool) BaseStation {
	lh := &LighthouseV2{
		Name:             config.Name,
		Id:               config.Id,
		Status:           "preloaded",
		CachedPowerState: -1,
		CachedChannel:    config.LastChannel,
		ValidLighthouse:  false,
		mac:              config.MacAddress,
	}

	go connectToPreloadedBaseStation(lh, config, wakeUp, 0)

	return lh
}

// connectToPreloadedBaseStation connects (or reconnects) to a base station
// that was already known from the saved configuration, retrying indefinitely
// (with a backoff) until it succeeds. It is shared across all platforms since
// address parsing/connection semantics are handled inside BLEAdapter.Connect.
func connectToPreloadedBaseStation(bs *LighthouseV2, config BaseStationConfiguration, wakeUp bool, attempt int) {
	var conn BLEDevice
	var err error

	for {
		conn, err = adapter.Connect(config.MacAddress, time.Second*10)
		if err == nil {
			break
		}

		log.Printf("Failed to connect to base station: %s %+v\n", config.Id, err)
		time.Sleep(retryBackoff(attempt))
		attempt++
	}

	bs.adapter = adapter
	bs.p = conn
	bs.mac = conn.Address()

	log.Printf("Connected to base station: %s, wake up: %+v\n", config.Id, wakeUp)

	go bs.PostInit(wakeUp)
}

func (lv *LighthouseV2) FindService() bool {
	services, err := lv.p.DiscoverServices()
	if err != nil {
		lv.Reconnect()
		log.Printf("Failed to find service: %+v\n", err)
		return false
	}

	var foundService BLEService
	for _, service := range services {
		uuid := strings.ToUpper(service.UUID())
		if uuid == LIGHTHOUSE_SERVICE_UUID {
			log.Printf("Found lighthouse service on base station %s\n", lv.Id)
			foundService = service
			break
		}
	}

	lv.service = foundService

	return true
}

func InitBaseStation(connection BLEDevice, adapter BLEAdapter, name string) (BaseStation, bool) {
	bs := &LighthouseV2{
		p:                        connection,
		adapter:                  adapter,
		service:                  nil,
		modeCharacteristic:       nil,
		identifyCharacteristic:   nil,
		powerStateCharacteristic: nil,
		Name:                     name,
		CachedPowerState:         -1,
		CachedChannel:            -1,
		Id:                       name,
		Status:                   "scanning",
		mac:                      connection.Address(),
	}

	go bs.PostInit(false)
	return bs, true
}

func (lighthouse *LighthouseV2) PostInit(wakeUp bool) {
	statusChannel := make(chan bool, 1)
	go lighthouse.InitStack(statusChannel)

	go func() {
		select {
		case status := <-statusChannel:
			if !status {
				lighthouse.Disconnect()
				go lighthouse.Reconnect()
				return
			}

			if wakeUp {
				lighthouse.SetPowerState(byte(0x01))
			}
		case <-time.After(time.Second * 3):
			go lighthouse.Reconnect()
			return
		}
	}()
}

func (lighthouse *LighthouseV2) InitStack(status chan bool) {

	if !lighthouse.FindService() {
		status <- false //Failed
	}

	//Adding deadline
	lighthouse.ValidLighthouse = lighthouse.ScanCharacteristics()

	if !lighthouse.ValidLighthouse {
		status <- false // Failed as well
	}
	go lighthouse.StartCaching()

	lighthouse.CachedPowerState = lighthouse.readPowerState()

	WEBSOCKET_BROADCAST.Broadcast(preparePacket("lighthouse.found", JsonBaseStation{
		Name:         lighthouse.GetName(),
		Channel:      lighthouse.GetChannel(),
		PowerState:   lighthouse.GetPowerState(),
		LastUpdated:  nil,
		Version:      lighthouse.GetVersion(),
		Status:       lighthouse.GetStatus(),
		ManagedFlags: 6,
		Id:           lighthouse.GetId(),
	}))
	//Everything is fine, telling in status that we don't need to wait anymore

	status <- true
}

func (lv *LighthouseV2) ScanCharacteristics() bool {
	if lv.service == nil {
		log.Printf("Lighthouse service on base station %s not found, reconnecting...\n", lv.Name)
		lv.Reconnect()
		return false
	}

	characteristics, err := lv.service.DiscoverCharacteristics()
	if err != nil {
		lv.p.Disconnect()
		return false
	}

	for _, char := range characteristics {
		uuid := char.UUID()

		switch strings.ToUpper(uuid) {
		case LIGHTHOUSE_CHARACTERISTIC_MODE:
			lv.modeCharacteristic = char
		case LIGHTHOUSE_CHARACTERISTIC_IDENTITFY:
			lv.identifyCharacteristic = char
		case LIGHTHOUSE_CHARACTERISTIC_POWER:
			lv.powerStateCharacteristic = char
		}
	}

	log.Printf("Finished finding characteristics on base station %s", lv.Name)
	log.Printf("Mode: %v, Power: %v, ID: %v",
		lv.modeCharacteristic != nil,
		lv.powerStateCharacteristic != nil,
		lv.identifyCharacteristic != nil)

	lv.ValidLighthouse = lv.modeCharacteristic != nil && lv.powerStateCharacteristic != nil

	if lv.ValidLighthouse {
		log.Println("Sending ready...")
		lv.Status = "ready"
		WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.status", "status", "ready"))
		WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.channel", "channel", lv.readChannel()))
		WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.power_state", "power_state", lv.readPowerState()))
	}

	lv.mac = lv.p.Address()
	go lv.StartCaching()
	return lv.modeCharacteristic != nil && lv.powerStateCharacteristic != nil
}

func (lv *LighthouseV2) GetChannel() int {

	return lv.CachedChannel
}

func (lv *LighthouseV2) SetChannel(channel int) {

	if !lv.ValidLighthouse {
		return
	}

	if lv.modeCharacteristic == nil {
		log.Printf("ModeCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.SetChannel(channel)
		return
	}

	_, err := lv.Write(lv.modeCharacteristic, []byte{byte(channel)})

	if err != nil {
		log.Printf("Failed to write bytes on lighthouse, reconnecting...; lighthouse=%s, err=%+v;\n", lv.Id, err)

		lv.Reconnect()
	}

	lv.CachedChannel = channel
}

func (lv *LighthouseV2) GetPowerState() int {
	return lv.CachedPowerState
}

func (lv *LighthouseV2) SetPowerState(state byte) {

	if !lv.ValidLighthouse {
		return
	}

	if lv.powerStateCharacteristic == nil {
		log.Printf("PowerStateCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.SetPowerState(state)
		return
	}

	// _, err := lv.powerStateCharacteristic.Write([]byte{state})
	_, err := lv.Write(lv.powerStateCharacteristic, []byte{state})

	if err != nil {
		log.Printf("Failed to write bytes on lighthouse, reconnecting...; lighthouse=%s, err=%+v;\n", lv.Id, err)

		lv.Reconnect()
	}
}

func (lv *LighthouseV2) Identitfy() {

	if !lv.ValidLighthouse {
		return
	}

	if lv.identifyCharacteristic == nil {
		log.Printf("IdentifyCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.Identitfy()
		return
	}

	// _, err := lv.identifyCharacteristic.Write([]byte{0x01})
	_, err := lv.Write(lv.identifyCharacteristic, []byte{0x01})

	if err != nil {
		log.Printf("Failed to write bytes on lighthouse, reconnecting...; lighthouse=%s, err=%+v;\n", lv.Id, err)

		lv.Reconnect()
	}
}

func (lv *LighthouseV2) GetName() string {

	return lv.Name
}

func (lv *LighthouseV2) Disconnect() {

	if lv.p == nil {
		return
	}
	err := lv.p.Disconnect()

	if err != nil {
		log.Println("Failed to disconnect.")
		log.Println(err)
		return
	}
	log.Println("Device has been disconnected")
}

func (lv *LighthouseV2) GetVersion() int {
	return 2 // stands for 2.0
}

func (lv *LighthouseV2) GetId() string {
	return lv.Id
}

func (lv *LighthouseV2) GetMAC() string {
	if lv.p != nil {
		return lv.p.Address()
	}

	return lv.mac
}

func (lv *LighthouseV2) GetStatus() string {
	return lv.Status
}

func (lv *LighthouseV2) SetName(name string) {
	lv.Name = name
}

func (lv *LighthouseV2) IsOutdated() bool {
	return lv.updateAvailable
}

func (lv *LighthouseV2) readPowerState() int {
	if lv.powerStateCharacteristic == nil {
		return -1
	}

	data, err := lv.powerStateCharacteristic.Read()

	if err != nil || len(data) == 0 {
		log.Printf("Failed to read state on %s: %+v\n", lv.Id, err)
		lv.Reconnect()
		return lv.readPowerState()
	}

	lv.CachedPowerState = int(data[0])
	return int(data[0])
}

func (lv *LighthouseV2) readChannel() int {
	if lv.modeCharacteristic == nil {
		return -1
	}

	data, err := lv.modeCharacteristic.Read()

	if err != nil || len(data) == 0 {
		log.Printf("Failed to read channel on %s: %+v\n", lv.Id, err)
		lv.Reconnect()
		return lv.readChannel()
	}

	lv.CachedChannel = int(data[0])

	if config != nil && config.KnownBaseStations[lv.Id] != nil && config.KnownBaseStations[lv.Id].LastChannel != lv.CachedChannel {
		config.KnownBaseStations[lv.Id].LastChannel = lv.CachedChannel
		config.Save()
	}
	return int(data[0])
}

// StartCaching enables notifications on the mode/power characteristics so
// channel/power-state changes (including ones made outside of this app) are
// reflected without polling. Shared across platforms since EnableNotifications
// already hides the tinygo-vs-BlueZ difference.
func (lv *LighthouseV2) StartCaching() {
	if lv.powerStateCharacteristic != nil {
		err := lv.powerStateCharacteristic.EnableNotifications(func(buf []byte) {
			if len(buf) == 0 {
				return
			}
			lv.CachedPowerState = int(buf[0])
			WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.power_state", "power_state", int(buf[0])))
			log.Printf("Power state on %s changed: %+v", lv.Id, buf)
		})

		if err != nil {
			log.Printf("Failed to receive notifications on power state, base station firmware probably outdated; lighthouse=%s; err=%+v", lv.Id, err)
			lv.updateAvailable = true
		}
	}

	if lv.modeCharacteristic != nil {
		err := lv.modeCharacteristic.EnableNotifications(func(buf []byte) {
			if len(buf) == 0 {
				return
			}
			lv.CachedChannel = int(buf[0])
			WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.channel", "channel", int(buf[0])))
		})

		if err != nil {
			log.Printf("Failed to receive notifications on power state, base station firmware probably outdated; lighthouse=%s; err=%+v", lv.Id, err)
			lv.updateAvailable = true
		}
	}

	// On platforms that can proactively detect a lost connection (Linux/BlueZ),
	// wire it up to the regular reconnect path instead of waiting for the next
	// failed read/write.
	if dw, ok := lv.p.(optionalDisconnectWatcher); ok {
		dw.OnDisconnected(func() {
			go lv.Reconnect()
		})
	}
}

// Reconnect tears down cached characteristics/service and reconnects to the
// base station by address, retrying with backoff until it succeeds. The
// reconnecting guard makes this safe to call concurrently (e.g. a failed
// write and a BlueZ disconnect signal racing) without stacking up duplicate
// reconnect attempts for the same base station - each base station has its
// own LighthouseV2/guard so this does not interfere with other base stations
// reconnecting at the same time.
func (lv *LighthouseV2) Reconnect() {
	if !atomic.CompareAndSwapInt32(&lv.reconnecting, 0, 1) {
		return
	}
	defer atomic.StoreInt32(&lv.reconnecting, 0)

	WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.status", "status", "preloaded"))

	lv.identifyCharacteristic = nil
	lv.modeCharacteristic = nil
	lv.powerStateCharacteristic = nil

	log.Printf("Reconnecting to %s...\n", lv.Id)

	attempt := 0
	for {
		conn, err := adapter.Connect(lv.mac, time.Second*10)
		if err != nil {
			log.Printf("Failed to reconnect to lighthouse %s: %+v\n", lv.Name, err)
			time.Sleep(retryBackoff(attempt))
			attempt++
			continue
		}

		lv.p = conn
		break
	}

	go lv.PostInit(false)
}

// retryBackoff gives connect/reconnect retry loops capped exponential
// backoff (1s, 2s, 4s, ... up to 30s) instead of hammering a base station
// that is simply out of range or turned off every second forever.
func retryBackoff(attempt int) time.Duration {
	const maxBackoff = 30 * time.Second
	if attempt < 0 || attempt > 20 {
		return maxBackoff
	}
	d := time.Second << uint(attempt)
	if d <= 0 || d > maxBackoff {
		return maxBackoff
	}
	return d
}
