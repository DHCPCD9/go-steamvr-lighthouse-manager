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
)

type BaseStation interface {
	ScanCharacteristics()
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
		if uuid == "00001523-1212-EFDE-1523-785FEABCD124" {
			log.Println("Found service")
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

	bs.ScanCharacteristics()

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

		log.Println("Force cache reset.")
	}
}
func (lv *LighthouseV2) ScanCharacteristics() {
	if lv.service == nil {
		log.Println("Service not found")
		return
	}

	characteristics, err := lv.service.DiscoverCharacteristics(nil)
	if err != nil {
		lv.p.Disconnect()
	}

	for i := range characteristics {
		char := &characteristics[i]
		uuid := char.UUID().String()

		switch strings.ToUpper(uuid) {
		case "00001524-1212-EFDE-1523-785FEABCD124":
			lv.modeCharacteristic = char
		case "00008421-1212-EFDE-1523-785FEABCD124":
			lv.identifyCharacteristic = char
		case "00001525-1212-EFDE-1523-785FEABCD124":
			lv.powerStateCharacteristic = char
		}
	}

	log.Println("Finished finding characteristics")
	log.Printf("Mode: %v, Power: %v, ID: %v",
		lv.modeCharacteristic != nil,
		lv.powerStateCharacteristic != nil,
		lv.identifyCharacteristic != nil)
}

func (lv *LighthouseV2) readChannel() int {

	if lv.modeCharacteristic == nil {
		log.Println("ModeCharacteristic is nil")
		return -1
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
	return lv.CachedChannel
}

func (lv *LighthouseV2) SetChannel(channel int) {
	if lv.modeCharacteristic == nil {
		log.Println("ModeCharacteristic is nil")
		return
	}

	_, err := lv.modeCharacteristic.Write([]byte{byte(channel)})
	if err != nil {
		log.Printf("Write error: %v", err)
	}

	lv.CachedChannel = channel
}

func (lv *LighthouseV2) readPowerState() int {
	if lv.powerStateCharacteristic == nil {
		log.Println("PowerStateCharacteristic is nil")
		return -1
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
	return lv.CachedPowerState
}

func (lv *LighthouseV2) SetPowerState(state byte) {
	if lv.powerStateCharacteristic == nil {
		log.Println("PowerStateCharacteristic is nil")
		return
	}

	lv.powerStateCharacteristic.Write([]byte{state})
	lv.CachedPowerState = int(state)
}

func (lv *LighthouseV2) Identitfy() {
	if lv.identifyCharacteristic == nil {
		log.Println("identifyCharacteristic is nil")
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
