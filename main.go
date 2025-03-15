package main

import (
	"embed"

	"github.com/wailsapp/wails/v2/pkg/application"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
)

//go:embed all:frontend/dist
var assets embed.FS

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
		},
		Frameless:        true,
		BackgroundColour: &options.RGBA{R: 18, G: 18, B: 18, A: 1},
		OnStartup:        app.startup,
		Bind: []interface{}{
			app,
		},
	})

	err := mainApp.Run()

	if err != nil {
		println("Error:", err.Error())
	}
}
