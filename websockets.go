package main

import (
	"log"
	"net/http"
	"runtime"
	"time"

	"github.com/gorilla/websocket"
	"github.com/teivah/broadcast"
)

type BaseStationUpdate struct {
	Station  string      `json:"id"`
	Type     string      `json:"type"`
	NewValue interface{} `json:"field_value"`
}

var STEAMVR_AVAILABLE = broadcast.NewRelay[bool]()
var config *Configuration
var PREVIOUS_STEAMVR_VALUE = false
var STEAMVR_WATCHING = false
var WEBSOCKET_BROADCAST = broadcast.NewRelay[interface{}]()

func StartHttp() {
	http.HandleFunc("/api/lighthouse/websocket", handleLighthouseSocket)

	// defer BASESTATION_FOUND.Close()
	// defer BASESTATION_UPDATE.Close()
	// defer CONFIGURATION_UPDATE.Close()
	defer WEBSOCKET_BROADCAST.Close()

	log.Fatal(http.ListenAndServe(":15065", nil))

}

func waitForSteamVR() {
	go func() {
		if runtime.GOOS == "windows" {

			log.Println("Started waiting for steamvr")

			for {

				if config == nil {
					time.Sleep(time.Second)
					continue
				}

				if !config.IsSteamVRManaged {
					STEAMVR_WATCHING = false
					break
				}
				STEAMVR_WATCHING = true

				isSteamVRLaunched, err := isProcRunning("vrserver.exe")

				if err != nil {
					STEAMVR_WATCHING = false
					break
				}

				if !config.IsSteamVRManaged {
					STEAMVR_WATCHING = false
					break
				}

				if PREVIOUS_STEAMVR_VALUE != isSteamVRLaunched {
					PREVIOUS_STEAMVR_VALUE = isSteamVRLaunched

				}

				time.Sleep(time.Second * 3)
			}
		}

	}()
}

func handleLighthouseSocket(w http.ResponseWriter, r *http.Request) {
	var upgrader = websocket.Upgrader{
		ReadBufferSize:  1024,
		WriteBufferSize: 1024,
	}

	upgrader.CheckOrigin = func(r *http.Request) bool { return true }

	ws, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println(err)
	}

	go reader(ws)
}

func reader(conn *websocket.Conn) {

	//Sending all current base stations
	for _, v := range knownBaseStations.Items() {

		bs := *v

		configBs := config.KnownBaseStations[bs.GetId()]

		managedFlags := 6

		if configBs != nil {
			managedFlags = configBs.ManagedFlags
		}

		conn.WriteJSON(preparePacket("lighthouse.found", JsonBaseStation{
			Name:         bs.GetName(),
			Channel:      bs.GetChannel(),
			PowerState:   bs.GetPowerState(),
			LastUpdated:  nil,
			Version:      bs.GetVersion(),
			Status:       bs.GetStatus(),
			ManagedFlags: managedFlags,
			Id:           bs.GetId(),
		}))
	}

	for _, v := range config.Groups {
		conn.WriteJSON(preparePacket("groups.created", v))
	}

	conn.WriteJSON(preparePacket("config.configure", config))
	conn.WriteJSON(preparePacket("steamvr.status", map[string]interface{}{
		"status": PREVIOUS_STEAMVR_VALUE,
	}))

	go func() {
		for {
			listener := WEBSOCKET_BROADCAST.Listener(1)

			data := <-listener.Ch()

			err := conn.WriteJSON(data)

			if err != nil {
				break
			}
		}
	}()

}

func prepareIdWithFieldPacket(lighthouse string, packetType string, fieldName string, fieldValue interface{}) interface{} {
	return preparePacket(packetType, map[string]interface{}{
		"id":      lighthouse,
		fieldName: fieldValue,
	})
}

func preparePacket(packetType string, body interface{}) interface{} {

	return map[string]interface{}{
		"packet_type": packetType,
		"data":        body,
	}
}
