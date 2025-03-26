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
	GetVersion() int
	Disconnect()
}

type LighthouseV2 struct {
	modeCharacteristic       *bluetooth.DeviceCharacteristic
	identifyCharacteristic   *bluetooth.DeviceCharacteristic
	powerStateCharacteristic *bluetooth.DeviceCharacteristic
	p                        *bluetooth.Device
	adapter                  *bluetooth.Adapter
	service                  *bluetooth.DeviceService
	Name                     string
	CachedPowerState         int
	CachedChannel            int
	CacheTimer               *time.Ticker
	ValidLighthouse          bool
}

func InitBaseStation(connection *bluetooth.Device, adapter *bluetooth.Adapter, name string) BaseStation {
	services, err := connection.DiscoverServices(nil)
	if err != nil {
		connection.Disconnect()
		panic("Failed to discover services: " + err.Error())
	}

	var foundService *bluetooth.DeviceService
	for i := range services {
		service := &services[i]
		uuid := strings.ToUpper(service.UUID().String())
		if uuid == LIGHTHOUSE_SERVICE_UUID {
			log.Printf("Found lighthouse  service on base station %s\n", name)
			foundService = service
			break
		}
	}

	bs := &LighthouseV2{
		p:                        connection,
		adapter:                  adapter,
		service:                  foundService,
		modeCharacteristic:       nil,
		identifyCharacteristic:   nil,
		powerStateCharacteristic: nil,
		Name:                     name,
		CachedPowerState:         -1,
		CachedChannel:            -1,
		CacheTimer:               time.NewTicker(time.Second * 10),
	}

	bs.ValidLighthouse = bs.ScanCharacteristics()

	bs.CachedPowerState = bs.readPowerState()
	bs.CachedChannel = bs.readChannel()

	go bs.StartCaching()
	return bs
}

func (lv *LighthouseV2) StartCaching() {

	for {
		<-lv.CacheTimer.C
		lv.CachedPowerState = lv.readPowerState()
		lv.CachedChannel = lv.readChannel()

		// log.Printf("Force cache reset. - %s\n", lv.Name)
		// We do not need to log it, because of how SSD work.
	}
}
func (lv *LighthouseV2) ScanCharacteristics() bool {
	if lv.service == nil {
		log.Printf("Lighthouse service on base station %s not found\n", lv.Name)
		return false
	}

	characteristics, err := lv.service.DiscoverCharacteristics(nil)
	if err != nil {
		lv.p.Disconnect()
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
	return lv.modeCharacteristic != nil && lv.powerStateCharacteristic != nil
}

func (lv *LighthouseV2) readChannel() int {

	if !lv.ValidLighthouse {
		return -1
	}

	if lv.modeCharacteristic == nil {
		log.Printf("ModeCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		return lv.readChannel()
	}

	data := make([]byte, 2)
	_, err := lv.modeCharacteristic.Read(data)
	if err != nil {
		log.Printf("Read error: %v", err)
		return -1
	}

	return int(data[0])
}

func (lv *LighthouseV2) GetChannel() int {

	if lv.CachedChannel == -1 {
		return lv.readChannel()
	}
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

func (lv *LighthouseV2) readPowerState() int {
	if !lv.ValidLighthouse {
		return -1
	}

	if lv.powerStateCharacteristic == nil {
		log.Printf("PowerStateCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		return lv.readPowerState()
	}

	data := make([]byte, 2)
	_, err := lv.powerStateCharacteristic.Read(data)
	if err != nil {
		log.Printf("Read error: %v", err)
		return -1
	}

	return int(data[0])
}

func (lv *LighthouseV2) GetPowerState() int {

	if lv.CachedPowerState == -1 {
		return lv.readPowerState()
	}
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
	lv.CachedPowerState = int(state)
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
	lv.p.Disconnect()
}

func (lv *LighthouseV2) GetVersion() int {
	return 2 // stands for 2.0
}
