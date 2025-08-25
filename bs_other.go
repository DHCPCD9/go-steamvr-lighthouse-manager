//go:build !darwin
// +build !darwin

package main

import (
	"log"
	"time"

	"tinygo.org/x/bluetooth"
)

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

	defer conn.Disconnect()

	bs.p = &conn

	log.Printf("Connected to base station: %s, wake up: %+v\n", config.Id, wakeUp)

	bs.FindService()
	bs.ScanCharacteristics()

	if wakeUp {
		bs.SetPowerState(byte(0x01))
	}

	go bs.StartCaching()
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

	defer conn.Disconnect()
	lv.p = &conn

	lv.FindService()
	lv.ScanCharacteristics()
}
