package main

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path"
	"reflect"
	"slices"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/tiendc/go-deepcopy"
)

//go:embed steamvr/manifest.vrmanifest
var manifest string

//go:embed steamvr/com.github.dhcpcd9.base-station-manager.vrappconfig
var vrappconfig string

type BaseStationConfiguration struct {
	MacAddress   string    `json:"mac_address"`
	LastSeen     time.Time `json:"last_seen"`
	LastChannel  int       `json:"channel"`
	Nickname     string    `json:"nickname"`
	Id           string    `json:"id"`
	ManagedFlags int       `json:"managed_flags"`
}

type Group struct {
	Name           string   `json:"name"`
	ManagedFlags   int      `json:"managed_flags"`
	BaseStationIDs []string `json:"base_stations"`
}

type Configuration struct {
	IsSteamVRManaged   bool                                 `json:"is_steamvr_managed"`
	IsSteamVRInstalled bool                                 `json:"is_steamvr_installed"`
	AllowTray          bool                                 `json:"allow_tray"`
	TrayNotified       bool                                 `json:"tray_notified"`
	KnownBaseStations  map[string]*BaseStationConfiguration `json:"known_base_stations"`
	Groups             map[string]*Group                    `json:"groups"`
	VersionBranch      string                               `json:"branch"`
}

type AppConfig struct {
	ManifestPaths []string `json:"manifest_paths"`
}

type VRAppConfig struct {
	Autolaunch     bool   `json:"autolaunch"`
	LastLaunchTime string `json:"last_launch_time"`
}

func GetConfiguration() *Configuration {
	appdataFolder, err := os.UserConfigDir()
	config := Configuration{IsSteamVRManaged: true, AllowTray: true, KnownBaseStations: make(map[string]*BaseStationConfiguration), TrayNotified: false}

	if err != nil {
		panic("Failed to find user config dir.")
	}

	if _, err := os.Stat(path.Join(appdataFolder, "Alumi", "Base Station Manager")); os.IsNotExist((err)) {
		os.MkdirAll(path.Join(appdataFolder, "Alumi", "Base Station Manager"), 0700)
	}

	if _, err := os.Stat(path.Join(appdataFolder, "Alumi", "Base Station Manager", "config.json")); os.IsNotExist(err) {
		config.Save()
	}

	//Loading config
	config.Load()

	config.IsSteamVRInstalled = GetSteamVRInstalled()
	return &config
}

func (c *Configuration) Load() {

	configPath := c.GetConfigPath()

	configBytes, err := os.ReadFile(configPath)

	if err != nil {
		panic("Failed to read config: " + err.Error())
	}

	var config Configuration
	err = json.Unmarshal(configBytes, &config)

	if err != nil {
		//Recreating it...
		c.Save()
		c.Load()
		return
	}

	deepcopy.Copy(c, config)

	//I don't know why deepcopy didn't copied it, but okay

	c.KnownBaseStations = config.KnownBaseStations

	if c.KnownBaseStations == nil {
		c.KnownBaseStations = make(map[string]*BaseStationConfiguration) // Works in case if user updated from old version
	}

	if c.Groups == nil {
		c.Groups = make(map[string]*Group)
	}

	if c.IsSteamVRInstalled {
		if c.IsSteamVRManaged {
			AddToStartup()
			if !STEAMVR_WATCHING {
				go waitForSteamVR()
			}
		} else {
			RemoveFromStartup()
		}
	}

	if c.VersionBranch == "" {
		c.VersionBranch = "main"
	}

	c.Save()
}

func (c *Configuration) UpdateValue(jsonName string, value interface{}) {
	UpdateValueOfInterface(c, jsonName, value)
	c.Save()
}

func (c *Configuration) UpdateBaseStationValue(baseStation string, jsonName string, value interface{}) {

	if c.KnownBaseStations[baseStation] == nil {
		return
	}
	UpdateValueOfInterface(c.KnownBaseStations[baseStation], jsonName, value)
	WEBSOCKET_BROADCAST.Broadcast(prepareIdWithFieldPacket(baseStation, fmt.Sprintf("lighthouse.update.%s", jsonName), jsonName, value))
	c.Save()
}

func (c *Configuration) UpdateGroup(groupName string, jsonName string, value interface{}) {

	if c.Groups[groupName] == nil {
		return
	}
	UpdateValueOfInterface(c.Groups[groupName], jsonName, value)
	c.Save()
}

func (c *Configuration) SaveBaseStation(baseStation *BaseStation) {
	bs := *baseStation

	c.KnownBaseStations[bs.GetId()] = &BaseStationConfiguration{
		MacAddress:   bs.GetMAC(),
		LastSeen:     time.Now(),
		LastChannel:  bs.GetChannel(),
		Nickname:     bs.GetId(),
		Id:           bs.GetId(),
		ManagedFlags: 6,
	}

	c.Save()
}

func (c *Configuration) ForgetBaseStation(name string) {
	delete(c.KnownBaseStations, name)
	c.Save()
}

func (c *Configuration) CreateGroup(name string) (string, *Group) {
	id := uuid.NewString()

	c.Groups[id] = &Group{
		Name:           name,
		ManagedFlags:   6,
		BaseStationIDs: []string{},
	}

	c.Save()

	return id, c.Groups[id]
}

func (c *Configuration) DeleteGroup(uid string) {
	delete(c.Groups, uid)
	c.Save()
}

func (c *Configuration) UpdateGroupValue(group, name string, value interface{}) {
	// c.Groups[name] = &Group{
	// 	Name:           name,
	// 	ManagedFlags:   6,
	// 	BaseStationIDs: []string{},
	// }

	if c.Groups[group] == nil {
		log.Printf("No group with name %s found; name=%+v; value=%+v", group, name, value)
		return
	}

	UpdateValueOfInterface(c.Groups[group], name, value)

	c.Save()
}

func (c *Configuration) Save() {
	WEBSOCKET_BROADCAST.Broadcast(preparePacket("client.configure", *c))

	data, err := json.Marshal(c)

	if err != nil {
		panic("Failed to save config: " + err.Error())
	}

	c.IsSteamVRInstalled = GetSteamVRInstalled()
	if c.IsSteamVRInstalled {
		if c.IsSteamVRManaged {
			AddToStartup()
			go waitForSteamVR()

		} else {
			RemoveFromStartup()
		}
	} else {
		c.IsSteamVRManaged = false
	}

	os.WriteFile(c.GetConfigPath(), data, 0644)

}

func (c *Configuration) GetConfigPath() string {
	return path.Join(GetConfigFolder(), "config.json")
}

func GetConfigFolder() string {
	appdataFolder, err := os.UserConfigDir()

	if err != nil {
		panic("Failed to find user config dir.")
	}

	if _, err = os.Stat(path.Join(appdataFolder, "Alumi", "Base Station Manager")); os.IsNotExist(err) {
		os.MkdirAll(path.Join(appdataFolder, "Alumi", "Base Station Manager"), 0700)
	}

	return path.Join(appdataFolder, "Alumi", "Base Station Manager")
}

func AddToStartup() {
	executable, _ := os.Executable()
	exPath := GetConfigFolder()

	//Should be good
	_ = os.WriteFile(path.Join(exPath, "manifest.vrmanifest"), []byte(strings.ReplaceAll(manifest, "%EXECUTABLE%", strings.ReplaceAll(executable, "\\", "\\\\"))), 0644) // windows thing

	//Appending it to appconfig.json

	fp := os.ExpandEnv("${ProgramFiles(x86)}\\Steam\\config\\appconfig.json")

	fileBytes, err := os.ReadFile(fp)

	if err != nil {
		panic("Failed to read appconfig.")
	}

	var appconfig AppConfig
	err = json.Unmarshal(fileBytes, &appconfig)

	if err != nil {
		panic("Failed to parse appconfig.")
	}

	if !slices.Contains(appconfig.ManifestPaths, path.Join(exPath, "manifest.vrmanifest")) {
		appconfig.ManifestPaths = append(appconfig.ManifestPaths, path.Join(exPath, "manifest.vrmanifest"))
	}

	//Writing file
	newConfigBytes, err := json.Marshal(appconfig)

	if err != nil {
		panic("Failed to marshal new appconfig.")
	}

	err = os.WriteFile(fp, newConfigBytes, os.FileMode(os.O_WRONLY))

	if err != nil {
		panic("Failed to write new appconfig.")
	}

	fp = os.ExpandEnv("${ProgramFiles(x86)}\\Steam\\config\\vrappconfig")

	_ = os.WriteFile(path.Join(fp, "com.github.dhcpcd9.base-station-manager.vrappconfig"), []byte(vrappconfig), 0644)
}

func RemoveFromStartup() {
	fp := os.ExpandEnv("${ProgramFiles(x86)}\\Steam\\config\\vrappconfig\\com.github.dhcpcd9.base-station-manager.vrappconfig")

	if err := os.Remove(fp); err != nil {
		log.Println("Failed to write config.")
	}
}

func GetSteamVRInstalled() bool {
	fp := os.ExpandEnv("${ProgramFiles(x86)}\\Steam\\config\\appconfig.json")

	_, err := os.ReadFile(fp)

	return err == nil
}

func UpdateValueOfInterface(c interface{}, jsonName string, value interface{}) {
	structValue := reflect.ValueOf(c).Elem()
	structType := structValue.Type()

	for i := 0; i < structType.NumField(); i++ {
		field := structType.Field(i)
		tag := field.Tag.Get("json")
		tagParts := strings.Split(tag, ",")
		jsonTagName := tagParts[0]

		if jsonTagName == jsonName {
			fieldValue := structValue.Field(i)
			log.Println("Found field:", field.Name, "value:", fieldValue.Interface())

			if fieldValue.CanSet() {
				val := reflect.ValueOf(value)
				if val.Type().ConvertibleTo(fieldValue.Type()) {
					fieldValue.Set(val.Convert(fieldValue.Type()))
				} else {
					log.Printf("Type mismatch: %s is %s, not %s\n", jsonName, val.Type(), fieldValue.Type())
				}
			}
			return
		}
	}
	log.Printf("JSON field name %s not found in struct\n", jsonName)
}
