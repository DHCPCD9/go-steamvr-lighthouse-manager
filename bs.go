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
)

type BaseStation struct {
	modeCharacteristic       *bluetooth.DeviceCharacteristic
	identifyCharacteristic   *bluetooth.DeviceCharacteristic
	powerStateCharacteristic *bluetooth.DeviceCharacteristic
	p                        *bluetooth.Device
	adapter                  *bluetooth.Adapter
	service                  *bluetooth.DeviceService
	Name                     string

	PowerState        int
	Channel           int
	IsValidLighthouse bool
	Address           string
}

func InitBaseStation(connection *bluetooth.Device, adapter *bluetooth.Adapter) *BaseStation {
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

	bs := &BaseStation{
		p:                        connection,
		adapter:                  adapter,
		service:                  foundService,
		modeCharacteristic:       nil,
		identifyCharacteristic:   nil,
		powerStateCharacteristic: nil,
		PowerState:               0,
		Channel:                  0,
		IsValidLighthouse:        true,
	}

	return bs
}

func (bs *BaseStation) ScanCharacteristics() {
	if bs.service == nil {
		log.Println("Service not found")
		return
	}

	characteristics, err := bs.service.DiscoverCharacteristics(nil)
	if err != nil {
		bs.p.Disconnect()
	}

	for i := range characteristics {
		char := &characteristics[i]
		uuid := char.UUID().String()

		switch strings.ToUpper(uuid) {
		case "00001524-1212-EFDE-1523-785FEABCD124":
			bs.modeCharacteristic = char
		case "00008421-1212-EFDE-1523-785FEABCD124":
			bs.identifyCharacteristic = char
		case "00001525-1212-EFDE-1523-785FEABCD124":
			bs.powerStateCharacteristic = char
		}

	}

	log.Println("Finished finding characteristics")
	log.Printf("Mode: %v, Power: %v, ID: %v", bs.modeCharacteristic != nil, bs.powerStateCharacteristic != nil, bs.identifyCharacteristic != nil)
}

func (bs *BaseStation) GetChannel() int {

	if bs.modeCharacteristic == nil {
		log.Println("ModeCharacteristic is nil")
		return -1
	}

	data := make([]byte, 2) // Буфер достаточного размера
	_, err := bs.modeCharacteristic.Read(data)
	if err != nil {
		log.Printf("Read error: %v", err)
		return -1
	}

	return int(data[0])
}

func (bs *BaseStation) SetChannel(channel int) {

	if bs.modeCharacteristic == nil {
		log.Println("ModeCharacteristic is nil")
		return
	}

	_, err := bs.modeCharacteristic.Write([]byte{byte(channel)})
	if err != nil {
		log.Printf("Read error: %v", err)
	}

}

func (bs *BaseStation) GetPowerState() int16 {

	if bs.powerStateCharacteristic == nil {
		log.Println("PowerStateCharacteristic is nil")
		return -1
	}

	data := make([]byte, 2)
	_, err := bs.powerStateCharacteristic.Read(data)
	if err != nil {
		log.Printf("Read error: %v", err)
		return -1
	}

	return int16(data[0])
}

func (bs *BaseStation) SetPowerState(state byte) {

	if bs.powerStateCharacteristic == nil {
		log.Println("PowerStateCharacteristic is nil")
		return
	}

	bs.powerStateCharacteristic.Write([]byte{state})
	bs.PowerState = int(state)
}

func (bs *BaseStation) Identitfy() {

	if bs.identifyCharacteristic == nil {
		log.Println("identifyCharacteristic is nil")
		return
	}

	bs.identifyCharacteristic.Write([]byte{0x01})
}
