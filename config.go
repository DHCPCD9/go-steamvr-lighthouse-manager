package main

import (
	_ "embed"
	"encoding/json"
	"log"
	"os"
	"path"
	"reflect"
	"slices"
	"strings"
	"time"

	"github.com/tiendc/go-deepcopy"
)

//go:embed steamvr/manifest.vrmanifest
var manifest string

//go:embed steamvr/com.github.dhcpcd9.base-station-manager.vrappconfig
var vrappconfig string

type BaseStationConfiguration struct {
	MacAddress  string    `json:"mac_address"`
	LastSeen    time.Time `json:"last_seen"`
	LastChannel int       `json:"channel"`
	Nickname    string    `json:"nickname"`
	Id          string    `json:"id"`
	Managed     bool      `json:"managed"`
	GroupNames  []string  `json:"groups"`
}

type Configuration struct {
	IsSteamVRManaged   bool                                 `json:"is_steamvr_managed"`
	IsSteamVRInstalled bool                                 `json:"is_steamvr_installed"`
	AllowTray          bool                                 `json:"allow_tray"`
	TrayNotified       bool                                 `json:"tray_notified"`
	KnownBaseStations  map[string]*BaseStationConfiguration `json:"known_base_stations"`
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

	if c.IsSteamVRInstalled {
		if c.IsSteamVRManaged {
			AddToStartup()
		} else {
			RemoveFromStartup()
		}
	}
}

func (c *Configuration) UpdateValue(jsonName string, value interface{}) {
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

	//Saving
	c.Save()
}

func (c *Configuration) SaveBaseStation(baseStation *BaseStation) {
	bs := *baseStation

	c.KnownBaseStations[bs.GetId()] = &BaseStationConfiguration{
		MacAddress:  bs.GetMAC(),
		LastSeen:    time.Now(),
		LastChannel: bs.GetChannel(),
		Nickname:    bs.GetId(),
		Id:          bs.GetId(),
		Managed:     true,
		GroupNames:  []string{},
	}

	c.Save()
}

func (c *Configuration) Save() {
	data, err := json.Marshal(c)

	if err != nil {
		panic("Failed to save config: " + err.Error())
	}

	c.IsSteamVRInstalled = GetSteamVRInstalled()
	if c.IsSteamVRInstalled {
		if c.IsSteamVRManaged {
			AddToStartup()
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
