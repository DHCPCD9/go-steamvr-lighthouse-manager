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

func FindUpdates() *UpdateName {
	log.Println("Checking for updates...")
	releases, _, err := client.Repositories.ListReleases(context.Background(), "DHCPCD9", "go-steamvr-lighthouse-manager", &github.ListOptions{})
	if err != nil {
		log.Println("Failed to check updates: " + err.Error())
		return nil
	}

	if config == nil {
		log.Println("Failed to check updates: config is nil")
		return nil
	}

	for _, v := range releases {
		if strings.HasSuffix(*v.TagName, fmt.Sprintf("-%s", config.VersionBranch)) {
			if withReplacedSuffix, _ := strings.CutSuffix(*v.TagName, fmt.Sprintf("-%s", config.VersionBranch)); withReplacedSuffix != BINARY_VERSION {
				log.Printf("Update available: %s (current: %s)\n", *v.TagName, BINARY_VERSION)
				return &UpdateName{
					Available: true,
					Version:   *v.TagName,
					Branch:    config.VersionBranch,
				}
			}
			break
		}
	}

	log.Println("No updates found")

	return &UpdateName{
		Available: false,
		Version:   BINARY_VERSION,
		Branch:    config.VersionBranch,
	}

}

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
		if strings.HasSuffix(*v.TagName, fmt.Sprintf("-%s", config.VersionBranch)) {
			UpdateWithRelease(*v.TagName)
			return
		}
	}

	log.Printf("Failed to find updates with branch %s\n", config.VersionBranch)

	//Finding the installer

}

func UpdateWithRelease(tag string) {

	withReplacedSuffix, _ := strings.CutSuffix(tag, fmt.Sprintf("-%s", config.VersionBranch))

	if withReplacedSuffix == BINARY_VERSION {
		return
	}

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
