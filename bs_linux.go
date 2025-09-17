//go:build unix
// +build unix

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

	if adapter == nil {
		log.Printf("No adapter found.")
		ADAPTER_NOT_FOUND = true
		WEBSOCKET_BROADCAST.Broadcast(preparePacket("adapter.status", map[string]interface{}{}))
		return
	}

	conn, err := adapter.Connect(bluetooth.Address{
		MACAddress: bluetooth.MACAddress{
			MAC: parsedMac,
		},
	}, bluetooth.ConnectionParams{
		Timeout: bluetooth.NewDuration(time.Second * 10),
	})
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
	}, bluetooth.ConnectionParams{
		MaxInterval: bluetooth.NewDuration(time.Millisecond * 30),
	})

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

func (lv *LighthouseV2) StartCaching() {
	// BlueZ doesn't allow to connect multiple devices at the same time.
}

func (lv *LighthouseV2) SetPowerState(state byte) {
	lv.Reconnect()

	if !lv.ValidLighthouse {
		return
	}

	if lv.powerStateCharacteristic == nil {
		log.Printf("PowerStateCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.SetPowerState(state)
		return
	}

	n, err := lv.powerStateCharacteristic.WriteWithoutResponse([]byte{state})

	if err != nil {
		log.Printf("Write error: %v", err)
	}

	if n > 0 {
		lv.CachedPowerState = int(state)
		WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.power_state", "power_state", int(state)))
	}
}

func (lv *LighthouseV2) Identitfy() {
	lv.Reconnect()

	if !lv.ValidLighthouse {
		return
	}

	if lv.identifyCharacteristic == nil {
		log.Printf("IdentifyCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.Identitfy()
		return
	}

	lv.identifyCharacteristic.WriteWithoutResponse([]byte{0x01})

}

func (lv *LighthouseV2) SetChannel(channel int) {
	lv.Reconnect()

	if !lv.ValidLighthouse {
		return
	}

	if lv.modeCharacteristic == nil {
		log.Printf("ModeCharacteristic on %s was nil, rescanning characteristics and trying again...\n", lv.Name)
		lv.ScanCharacteristics()
		lv.SetChannel(channel)
		return
	}

	_, err := lv.modeCharacteristic.WriteWithoutResponse([]byte{byte(channel)})
	if err != nil {
		log.Printf("Write error: %v", err)
	}

	lv.CachedChannel = channel
	WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.channel", "channel", channel))
}
