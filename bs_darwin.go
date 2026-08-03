//go:build darwin
// +build darwin

package main

import (
	"time"

	"tinygo.org/x/bluetooth"
)

// macOS keeps using tinygo.org/x/bluetooth (CoreBluetooth backend), unchanged
// - including addressing devices by CoreBluetooth UUID instead of MAC. This
// file only adapts it to the platform-agnostic BLE* interfaces from bs.go.

func NewPlatformAdapter() BLEAdapter {
	return &tinygoAdapter{adapter: bluetooth.DefaultAdapter}
}

type tinygoAdapter struct {
	adapter *bluetooth.Adapter
}

func (a *tinygoAdapter) Enable() error {
	return a.adapter.Enable()
}

func (a *tinygoAdapter) Scan(callback func(BLEAdapter, BLEScanResult)) error {
	return a.adapter.Scan(func(_ *bluetooth.Adapter, sr bluetooth.ScanResult) {
		callback(a, &tinygoScanResult{sr: sr})
	})
}

func (a *tinygoAdapter) StopScan() error {
	return a.adapter.StopScan()
}

func (a *tinygoAdapter) Connect(address string, timeout time.Duration) (BLEDevice, error) {
	parsedUuid, err := bluetooth.ParseUUID(address)
	if err != nil {
		return nil, err
	}

	conn, err := a.adapter.Connect(bluetooth.Address{
		UUID: parsedUuid,
	}, bluetooth.ConnectionParams{})

	if err != nil {
		return nil, err
	}

	return &tinygoDevice{device: conn}, nil
}

type tinygoScanResult struct {
	sr bluetooth.ScanResult
}

func (r *tinygoScanResult) LocalName() string { return r.sr.LocalName() }
func (r *tinygoScanResult) Address() string   { return r.sr.Address.String() }

type tinygoDevice struct {
	device bluetooth.Device
}

func (d *tinygoDevice) Address() string  { return d.device.Address.String() }
func (d *tinygoDevice) Disconnect() error { return d.device.Disconnect() }

func (d *tinygoDevice) DiscoverServices() ([]BLEService, error) {
	services, err := d.device.DiscoverServices(nil)
	if err != nil {
		return nil, err
	}

	result := make([]BLEService, len(services))
	for i := range services {
		result[i] = &tinygoService{service: services[i]}
	}
	return result, nil
}

type tinygoService struct {
	service bluetooth.DeviceService
}

func (s *tinygoService) UUID() string { return s.service.UUID().String() }

func (s *tinygoService) DiscoverCharacteristics() ([]BLECharacteristic, error) {
	chars, err := s.service.DiscoverCharacteristics(nil)
	if err != nil {
		return nil, err
	}

	result := make([]BLECharacteristic, len(chars))
	for i := range chars {
		result[i] = &tinygoCharacteristic{char: chars[i]}
	}
	return result, nil
}

type tinygoCharacteristic struct {
	char bluetooth.DeviceCharacteristic
}

func (c *tinygoCharacteristic) UUID() string { return c.char.UUID().String() }

func (c *tinygoCharacteristic) Read() ([]byte, error) {
	data := make([]byte, 1)
	n, err := c.char.Read(data)
	if err != nil {
		return nil, err
	}
	return data[:n], nil
}

func (c *tinygoCharacteristic) Write(value []byte) (int, error) {
	return c.char.Write(value)
}

func (c *tinygoCharacteristic) WriteWithoutResponse(value []byte) (int, error) {
	return c.char.WriteWithoutResponse(value)
}

func (c *tinygoCharacteristic) EnableNotifications(callback func(buf []byte)) error {
	return c.char.EnableNotifications(callback)
}

func (lighthouse *LighthouseV2) Write(characteristic BLECharacteristic, value []byte) (int, error) {
	return characteristic.Write(value)
}
