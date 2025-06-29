//go:build windows
// +build windows

package main

import (
	"context"
	_ "embed"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path"
	"strings"
	"time"

	"github.com/google/go-github/v69/github"
	"golang.org/x/sys/windows/registry"
)

var FLAGS_NO_UPDATE string = "NO_UPDATES"
var client = github.NewClient(nil)

func ForceUpdate() {

	releases, _, err := client.Repositories.ListReleases(context.Background(), "DHCPCD9", "go-steamvr-lighthouse-manager", &github.ListOptions{})

	if err != nil {
		log.Println("Failed to check updates: " + err.Error())
		return
	}

	if config == nil {
		log.Println("Failed to update: config is nil")
		return
	}

	for _, v := range releases {
		if strings.HasSuffix(*v.Name, fmt.Sprintf("-%s", config.VersionBranch)) {
			UpdateWithRelease(*v.TagName)
			return
		}
	}

	log.Println("Failed to find updates")

	//Finding the installer

}

func UpdateWithRelease(tag string) {

	if strings.Contains(VERSION_FLAGS, "DEBUG") {
		return
	}

	release, _, err := client.Repositories.GetReleaseByTag(context.Background(), "DHCPCD9", "go-steamvr-lighthouse-manager", tag)

	if err != nil {
		log.Printf("Failed to update: %s\n", err.Error())
		return
	}

	for _, v := range release.Assets {
		if strings.HasSuffix(*v.Name, "installer.exe") {
			//Downloading the release
			response, err := http.Get(*v.BrowserDownloadURL)

			if err != nil {
				log.Println("Failed to download latest release")
				return
			}

			bodyBytes, _ := io.ReadAll(response.Body)

			os.WriteFile(path.Join(GetConfigFolder(), "installer.exe"), bodyBytes, 0644)

			cmd := exec.Command("cmd", "/k", strings.ReplaceAll(path.Join(GetConfigFolder(), "installer.exe"), "/", "\\"))
			go cmd.Output()

			time.Sleep(time.Second)
			os.Exit(0)

		}
	}
}

func IsUpdatingSupported() bool {

	//probing regedit
	_, err := registry.OpenKey(registry.LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Alumi Inc.Base Station Manager", registry.QUERY_VALUE)

	return err == nil && !strings.Contains(FLAGS_NO_UPDATE, VERSION_FLAGS)
}
