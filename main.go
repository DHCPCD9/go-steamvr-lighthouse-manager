package main

import (
	"embed"
	"log"
	"strings"

	"net/http"
	_ "net/http/pprof"

	"github.com/wailsapp/wails/v2/pkg/application"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/options/windows"
)

//go:embed all:frontend/dist
var assets embed.FS

//go:embed build/windows/icon.ico
var icon []byte

func main() {
	// Create an instance of the app structure
	app := NewApp()

	// Create application with options
	mainApp := application.NewWithOptions(&options.App{
		Title:         "Base Station Manager By Alumi",
		Width:         700,
		Height:        445,
		DisableResize: true,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		SingleInstanceLock: &options.SingleInstanceLock{
			UniqueId: "2defdeea-4350-43ad-beed-bc65f1b7cd69",
			OnSecondInstanceLaunch: func(secondInstanceData options.SecondInstanceData) {
				WAKE_UP_CHANNEL <- 1
			},
		},
		Frameless:        true,
		BackgroundColour: &options.RGBA{R: 18, G: 18, B: 18, A: 1},
		OnStartup:        app.startup,
		Windows: &windows.Options{
			IsZoomControlEnabled: false,
		},
		Bind: []interface{}{
			app,
		},
	})

	if strings.Contains(VERSION_FLAGS, "MEMORY_PROFILING") {
		go func() {
			log.Println(http.ListenAndServe(":6060", nil))
		}()
	}

	err := mainApp.Run()

	if err != nil {
		log.Fatalln("Error:", err.Error())
	}

}
