//go:build darwin
// +build darwin

package main

import (
	"log"
	"time"

	"tinygo.org/x/bluetooth"
)

func connectToPreloadedBaseStation(bs *LighthouseV2, config BaseStationConfiguration, wakeUp bool, attemp int) {

	parsedUuid, err := bluetooth.ParseUUID(config.MacAddress)

	if err != nil {
		log.Printf("Failed to parse mac: %s for lighthouse %s (%+v)\n", config.MacAddress, config.Id, err)
		return
	}

	conn, err := adapter.Connect(bluetooth.Address{
		UUID: parsedUuid,
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

	go bs.PostInit(wakeUp)

	go bs.StartCaching()
}

func (lv *LighthouseV2) Reconnect() {
	WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.status", "status", "preloaded"))
	lv.identifyCharacteristic = nil
	lv.modeCharacteristic = nil
	lv.powerStateCharacteristic = nil

	log.Println("Reconnecting...")
	parsedUuid, err := bluetooth.ParseUUID(lv.mac)

	if err != nil {
		log.Printf("Failed to parse MAC: %+v\n", err)
		return
	}

	conn, err := adapter.Connect(bluetooth.Address{
		UUID: parsedUuid,
	}, bluetooth.ConnectionParams{})

	if err != nil {
		log.Printf("Failed to reconnect to lighthouse %s: %+v\n", lv.Name, err)
		time.Sleep(time.Second)
		lv.Reconnect()
		return
	}

	lv.p = &conn

	go lv.PostInit(false)
}

func (lv *LighthouseV2) StartCaching() {
	if lv.powerStateCharacteristic != nil {

		err := lv.powerStateCharacteristic.EnableNotifications(func(buf []byte) {
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
			lv.CachedChannel = int(buf[0])
			WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.channel", "channel", int(buf[0])))

		})

		if err != nil {
			log.Printf("Failed to receive notifications on power state, base station firmware probably outdated; lighthouse=%s; err=%+v", lv.Id, err)
			lv.updateAvailable = true
		}
	}

	// go func() {
	// 	//I really ran out of ideas how to do it better
	// 	var data []byte = make([]byte, 1)
	// 	var err error
	// 	for {

	// 		if lv.powerStateCharacteristic == nil {
	// 			time.Sleep(time.Second)
	// 			continue
	// 		}
	// 		_, err = lv.powerStateCharacteristic.Read(data)

	// 		if err != nil {
	// 			WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(lv.Id, "lighthouse.update.status", "status", "preloaded"))
	// 			lv.Reconnect()
	// 			break
	// 		}
	// 		time.Sleep(time.Second)
	// 	}
	// }()
}
