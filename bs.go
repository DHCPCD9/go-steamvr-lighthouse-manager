package main

import (
	"log"
	"strings"

	"tinygo.org/x/bluetooth"
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
	modeCharacteristic       *bluetooth.DeviceCharacteristic
	identifyCharacteristic   *bluetooth.DeviceCharacteristic
	powerStateCharacteristic *bluetooth.DeviceCharacteristic
	p                        *bluetooth.Device
	adapter                  *bluetooth.Adapter
	service                  *bluetooth.DeviceService
	Name                     string
	Id                       string
	CachedPowerState         int
	CachedChannel            int
	ValidLighthouse          bool
	Status                   string
	mac                      string
	updateAvailable          bool
}

func PreloadBaseStation(config BaseStationConfiguration, wakeUp bool) BaseStation {
	lh := &LighthouseV2{
		Name:             config.Name,
		Id:               config.Id,
		Status:           "preloaded",
		CachedPowerState: -1,
		CachedChannel:    config.LastChannel,
		ValidLighthouse:  false,
	}

	go connectToPreloadedBaseStation(lh, config, wakeUp, 0)

	return lh
}

func (lv *LighthouseV2) FindService() {
	services, err := lv.p.DiscoverServices(nil)
	if err != nil {
		lv.Reconnect()
		log.Printf("Failed to find service: %+v\n", err)
		return
	}

	var foundService *bluetooth.DeviceService
	for i := range services {
		service := &services[i]
		uuid := strings.ToUpper(service.UUID().String())
		if uuid == LIGHTHOUSE_SERVICE_UUID {
			log.Printf("Found lighthouse service on base station %s\n", lv.Id)
			foundService = service
			break
		}
	}

	lv.service = foundService
}

func InitBaseStation(connection *bluetooth.Device, adapter *bluetooth.Adapter, name string) (BaseStation, bool) {
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
	}

	bs.FindService()
	bs.ValidLighthouse = bs.ScanCharacteristics()

	go bs.StartCaching()

	bs.CachedPowerState = bs.readPowerState()

	WEBSOCKET_BROADCAST.Broadcast(preparePacket("lighthouse.found", JsonBaseStation{
		Name:         bs.GetName(),
		Channel:      bs.GetChannel(),
		PowerState:   bs.GetPowerState(),
		LastUpdated:  nil,
		Version:      bs.GetVersion(),
		Status:       bs.GetStatus(),
		ManagedFlags: 6,
		Id:           bs.GetId(),
	}))

	return bs, true
}

func (lv *LighthouseV2) ScanCharacteristics() bool {
	if lv.service == nil {
		log.Printf("Lighthouse service on base station %s not found, reconnecting...\n", lv.Name)
		lv.Reconnect()
		return false
	}

	characteristics, err := lv.service.DiscoverCharacteristics(nil)
	if err != nil {
		lv.p.Disconnect()
		return false
	}

	for i := range characteristics {
		char := &characteristics[i]
		uuid := char.UUID().String()

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

	lv.mac = lv.p.Address.String()
	go lv.StartCaching()
	return lv.modeCharacteristic != nil && lv.powerStateCharacteristic != nil
}

func (lv *LighthouseV2) GetChannel() int {

	return lv.CachedChannel
}

func (lv *LighthouseV2) GetPowerState() int {
	return lv.CachedPowerState
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
		return lv.p.Address.String()
	}

	return ""
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

	var data []byte = make([]byte, 1)
	_, err := lv.powerStateCharacteristic.Read(data)

	if err != nil {
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

	var data []byte = make([]byte, 1)
	_, err := lv.modeCharacteristic.Read(data)

	if err != nil {
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
