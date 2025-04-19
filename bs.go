package main

import (
	"log"
	"strings"
	"time"

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
		Name:             config.Nickname,
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

func connectToPreloadedBaseStation(bs *LighthouseV2, config BaseStationConfiguration, wakeUp bool, attemp int) {

	parsedMac, err := bluetooth.ParseMAC(config.MacAddress)

	if err != nil {
		log.Printf("Failed to parse mac: %s for lighthouse %s (%+v)\n", config.MacAddress, config.Id, err)
		return
	}

	conn, err := adapter.Connect(bluetooth.Address{
		MACAddress: bluetooth.MACAddress{
			MAC: parsedMac,
		},
	}, bluetooth.ConnectionParams{})

	if err != nil {
		log.Printf("Failed to connect to base station: %s %+v", config.Id, err)

		time.Sleep(time.Second)
		connectToPreloadedBaseStation(bs, config, wakeUp, attemp+1)
		return
	}

	bs.adapter = adapter
	bs.p = &conn

	log.Printf("Connected to base station: %s, wake up: %+v\n", config.Id, wakeUp)

	bs.FindService()
	bs.ScanCharacteristics()

	if wakeUp {
		bs.SetPowerState(byte(0x01))
	}

	go bs.StartCaching()
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

func (lv *LighthouseV2) StartCaching() {
	if lv.powerStateCharacteristic != nil {

		err := lv.powerStateCharacteristic.EnableNotificationsWithMode(bluetooth.NotificationModeNotify, func(buf []byte) {
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
		err := lv.modeCharacteristic.EnableNotificationsWithMode(bluetooth.NotificationModeNotify, func(buf []byte) {
			lv.CachedChannel = int(buf[0])
			WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.channel", "channel", int(buf[0])))

		})

		if err != nil {
			log.Printf("Failed to receive notifications on power state, base station firmware probably outdated; lighthouse=%s; err=%+v", lv.Id, err)
			lv.updateAvailable = true
		}
	}

	go func() {
		//I really ran out of ideas how to do it better

		for {

			if lv.powerStateCharacteristic == nil {
				time.Sleep(time.Second)
				continue
			}
			var data []byte = make([]byte, 1)
			_, err := lv.powerStateCharacteristic.Read(data)

			if err != nil {
				WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.status", "status", "preloaded"))
				lv.Reconnect()
				break
			}
			time.Sleep(time.Second)
		}
	}()
}

func (lv *LighthouseV2) Reconnect() {
	lv.identifyCharacteristic = nil
	lv.modeCharacteristic = nil
	lv.powerStateCharacteristic = nil

	log.Println("Reconnecting...")
	parsedMac, err := bluetooth.ParseMAC(lv.mac)

	if err != nil {
		log.Printf("Failed to parse MAC: %+v\n", err)
		return
	}

	conn, err := adapter.Connect(bluetooth.Address{
		MACAddress: bluetooth.MACAddress{
			MAC: parsedMac,
		},
	}, bluetooth.ConnectionParams{})

	if err != nil {
		log.Printf("Failed to reconnect to lighthouse %s: %+v\n", lv.Name, err)
		time.Sleep(time.Second)
		lv.Reconnect()
		return
	}
	lv.p = &conn

	lv.FindService()
	lv.ScanCharacteristics()
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

	_, err := lv.modeCharacteristic.Write([]byte{byte(channel)})
	if err != nil {
		log.Printf("Write error: %v", err)
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

	lv.powerStateCharacteristic.Write([]byte{state})
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

	lv.identifyCharacteristic.Write([]byte{0x01})
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
		return lv.p.Address.MAC.String()
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
