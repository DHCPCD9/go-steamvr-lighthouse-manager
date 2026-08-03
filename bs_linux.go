//go:build linux
// +build linux

package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"regexp"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/godbus/dbus/v5"
)

// Linux talks to BlueZ directly over D-Bus (the system bus) instead of going
// through tinygo.org/x/bluetooth. This avoids that library's cgo-free but
// fairly bare-bones Linux backend and lets us handle BlueZ's real object
// model: adapters, devices and GATT services/characteristics are all
// separate D-Bus objects, discovered/kept up to date via the
// org.freedesktop.DBus.ObjectManager interface and PropertiesChanged signals.
// Everything here is safe to use concurrently for any number of base
// stations at once - there is a single shared bus connection and object
// cache, per-address connect locking, and per-object-path notification
// routing so base stations never interfere with each other.

const (
	bluezDest = "org.bluez"

	dbusObjMgrIface = "org.freedesktop.DBus.ObjectManager"
	dbusPropsIface  = "org.freedesktop.DBus.Properties"

	adapter1Iface     = "org.bluez.Adapter1"
	device1Iface      = "org.bluez.Device1"
	gattService1Iface = "org.bluez.GattService1"
	gattChar1Iface    = "org.bluez.GattCharacteristic1"

	propertiesChangedSignal = dbusPropsIface + ".PropertiesChanged"
	interfacesAddedSignal   = dbusObjMgrIface + ".InterfacesAdded"
	interfacesRemovedSignal = dbusObjMgrIface + ".InterfacesRemoved"
)

var macAddressPattern = regexp.MustCompile(`^[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){5}$`)

func NewPlatformAdapter() BLEAdapter {
	return &dbusAdapter{}
}

// Linux GATT writes to the lighthouse characteristics have always been done
// without waiting for a response (matches prior behavior of this backend).
func (lighthouse *LighthouseV2) Write(characteristic BLECharacteristic, value []byte) (int, error) {
	return characteristic.WriteWithoutResponse(value)
}

type managedObjects map[dbus.ObjectPath]map[string]map[string]dbus.Variant

type propSubscriber func(iface string, changed map[string]dbus.Variant)

type propEntry struct {
	token uint64
	fn    propSubscriber
}

type addedSubscriber func(path dbus.ObjectPath, ifaces map[string]map[string]dbus.Variant)

// anyPropsSubscriber is notified of every PropertiesChanged signal, regardless
// of path - unlike propSubscriber (single subscriber per exact path, used for
// device-disconnect/characteristic-notify plumbing), this is used during
// scanning: BlueZ frequently reports a device for the first time (via
// InterfacesAdded) before its advertised name has been parsed out of a later
// advertisement report, so we also need to watch for the Name/Alias arriving
// slightly afterwards via PropertiesChanged instead of only catching the
// device once at first sight.
type anyPropsSubscriber func(path dbus.ObjectPath, iface string, changed map[string]dbus.Variant)

type bluezManager struct {
	mu          sync.Mutex
	conn        *dbus.Conn
	adapterPath dbus.ObjectPath

	scanMu          sync.Mutex
	scanUnsubscribe func()

	connectMu    sync.Mutex
	connectLocks map[string]*sync.Mutex

	subTokens uint64

	objMu   sync.RWMutex
	objects managedObjects

	propMu   sync.RWMutex
	propSubs map[dbus.ObjectPath]propEntry

	addedMu   sync.RWMutex
	addedSubs map[uint64]addedSubscriber

	anyPropMu   sync.RWMutex
	anyPropSubs map[uint64]anyPropsSubscriber
}

var bluez = &bluezManager{
	connectLocks: map[string]*sync.Mutex{},
	objects:      managedObjects{},
	propSubs:     map[dbus.ObjectPath]propEntry{},
	addedSubs:    map[uint64]addedSubscriber{},
	anyPropSubs:  map[uint64]anyPropsSubscriber{},
}

func (m *bluezManager) ensureConnected() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.conn != nil {
		return nil
	}

	conn, err := dbus.SystemBus()
	if err != nil {
		return fmt.Errorf("bluez: failed to connect to the system D-Bus bus: %w", err)
	}

	if err := conn.AddMatchSignal(dbus.WithMatchInterface(dbusObjMgrIface)); err != nil {
		return fmt.Errorf("bluez: failed to subscribe to object manager signals: %w", err)
	}

	if err := conn.AddMatchSignal(
		dbus.WithMatchInterface(dbusPropsIface),
		dbus.WithMatchMember("PropertiesChanged"),
	); err != nil {
		return fmt.Errorf("bluez: failed to subscribe to property change signals: %w", err)
	}

	sigCh := make(chan *dbus.Signal, 256)
	conn.Signal(sigCh)
	go m.dispatchLoop(sigCh)

	var objects managedObjects
	bluezObj := conn.Object(bluezDest, dbus.ObjectPath("/"))
	if err := bluezObj.Call(dbusObjMgrIface+".GetManagedObjects", 0).Store(&objects); err != nil {
		return fmt.Errorf("bluez: failed to enumerate objects (is bluetoothd running?): %w", err)
	}

	log.Printf("bluez: enumerated %d object(s) from bluetoothd\n", len(objects))

	adapterPath, err := findAdapterPath(objects)
	if err != nil {
		return err
	}

	m.objMu.Lock()
	m.objects = objects
	m.objMu.Unlock()

	m.conn = conn
	m.adapterPath = adapterPath

	log.Printf("bluez: connected, using adapter %s\n", adapterPath)

	return nil
}

func findAdapterPath(objects managedObjects) (dbus.ObjectPath, error) {
	var best dbus.ObjectPath
	for path, ifaces := range objects {
		if _, ok := ifaces[adapter1Iface]; !ok {
			continue
		}
		if best == "" || path < best {
			best = path
		}
	}
	if best == "" {
		return "", errors.New("bluez: no Bluetooth adapter found")
	}
	return best, nil
}

func (m *bluezManager) dispatchLoop(ch <-chan *dbus.Signal) {
	for sig := range ch {
		switch sig.Name {
		case propertiesChangedSignal:
			if len(sig.Body) < 2 {
				continue
			}
			iface, _ := sig.Body[0].(string)
			changed, _ := sig.Body[1].(map[string]dbus.Variant)

			m.updateCache(sig.Path, iface, changed)
			m.notifyPropertiesChanged(sig.Path, iface, changed)

			m.anyPropMu.RLock()
			anySubs := make([]anyPropsSubscriber, 0, len(m.anyPropSubs))
			for _, s := range m.anyPropSubs {
				anySubs = append(anySubs, s)
			}
			m.anyPropMu.RUnlock()
			for _, s := range anySubs {
				s(sig.Path, iface, changed)
			}

		case interfacesAddedSignal:
			if len(sig.Body) < 2 {
				continue
			}
			path, _ := sig.Body[0].(dbus.ObjectPath)
			ifaces, _ := sig.Body[1].(map[string]map[string]dbus.Variant)

			m.addToCache(path, ifaces)

			m.addedMu.RLock()
			subs := make([]addedSubscriber, 0, len(m.addedSubs))
			for _, s := range m.addedSubs {
				subs = append(subs, s)
			}
			m.addedMu.RUnlock()
			for _, s := range subs {
				s(path, ifaces)
			}

		case interfacesRemovedSignal:
			if len(sig.Body) < 2 {
				continue
			}
			path, _ := sig.Body[0].(dbus.ObjectPath)
			removed, _ := sig.Body[1].([]string)

			hadDevice := containsStr(removed, device1Iface)
			m.removeFromCache(path, removed)

			if hadDevice {
				// The device object disappeared entirely (BlueZ does this for
				// untrusted/unpaired devices some time after they disconnect).
				// Treat it the same as a Connected=false property change so
				// reconnect logic doesn't need to know the difference.
				m.notifyPropertiesChanged(path, device1Iface, map[string]dbus.Variant{
					"Connected": dbus.MakeVariant(false),
				})
			}
		}
	}
}

func (m *bluezManager) notifyPropertiesChanged(path dbus.ObjectPath, iface string, changed map[string]dbus.Variant) {
	m.propMu.RLock()
	entry, ok := m.propSubs[path]
	m.propMu.RUnlock()
	if ok {
		entry.fn(iface, changed)
	}
}

func (m *bluezManager) addToCache(path dbus.ObjectPath, ifaces map[string]map[string]dbus.Variant) {
	m.objMu.Lock()
	defer m.objMu.Unlock()
	existing, ok := m.objects[path]
	if !ok {
		existing = map[string]map[string]dbus.Variant{}
		m.objects[path] = existing
	}
	for iface, props := range ifaces {
		existing[iface] = props
	}
}

func (m *bluezManager) removeFromCache(path dbus.ObjectPath, removedIfaces []string) {
	m.objMu.Lock()
	defer m.objMu.Unlock()
	existing, ok := m.objects[path]
	if !ok {
		return
	}
	for _, iface := range removedIfaces {
		delete(existing, iface)
	}
	if len(existing) == 0 {
		delete(m.objects, path)
	}
}

func (m *bluezManager) updateCache(path dbus.ObjectPath, iface string, changed map[string]dbus.Variant) {
	if iface == "" {
		return
	}
	m.objMu.Lock()
	defer m.objMu.Unlock()
	existing, ok := m.objects[path]
	if !ok {
		existing = map[string]map[string]dbus.Variant{}
		m.objects[path] = existing
	}
	props, ok := existing[iface]
	if !ok {
		props = map[string]dbus.Variant{}
		existing[iface] = props
	}
	for k, v := range changed {
		props[k] = v
	}
}

// onPropertiesChanged registers fn as the (sole) PropertiesChanged handler for
// path, replacing any previous subscriber for that same path. In practice
// every path we subscribe to (a device path, or a characteristic path) only
// ever has one logical subscriber at a time, so "last subscriber wins" is
// exactly what we want: it makes re-subscribing after a reconnect (BlueZ
// reuses the same object paths for the same address) automatically replace
// the stale subscriber instead of leaking it.
func (m *bluezManager) onPropertiesChanged(path dbus.ObjectPath, fn propSubscriber) func() {
	token := atomic.AddUint64(&m.subTokens, 1)
	m.propMu.Lock()
	m.propSubs[path] = propEntry{token: token, fn: fn}
	m.propMu.Unlock()

	return func() {
		m.propMu.Lock()
		defer m.propMu.Unlock()
		if cur, ok := m.propSubs[path]; ok && cur.token == token {
			delete(m.propSubs, path)
		}
	}
}

func (m *bluezManager) onInterfacesAdded(fn addedSubscriber) func() {
	token := atomic.AddUint64(&m.subTokens, 1)
	m.addedMu.Lock()
	m.addedSubs[token] = fn
	m.addedMu.Unlock()

	return func() {
		m.addedMu.Lock()
		defer m.addedMu.Unlock()
		delete(m.addedSubs, token)
	}
}

func (m *bluezManager) onAnyPropertiesChanged(fn anyPropsSubscriber) func() {
	token := atomic.AddUint64(&m.subTokens, 1)
	m.anyPropMu.Lock()
	m.anyPropSubs[token] = fn
	m.anyPropMu.Unlock()

	return func() {
		m.anyPropMu.Lock()
		defer m.anyPropMu.Unlock()
		delete(m.anyPropSubs, token)
	}
}

func (m *bluezManager) powerOn() error {
	m.mu.Lock()
	conn := m.conn
	adapterPath := m.adapterPath
	m.mu.Unlock()

	obj := conn.Object(bluezDest, adapterPath)

	var powered bool
	if err := obj.StoreProperty(adapter1Iface+".Powered", &powered); err == nil && powered {
		log.Printf("bluez: adapter %s already powered on\n", adapterPath)
		return nil
	}

	if err := obj.SetProperty(adapter1Iface+".Powered", true); err != nil {
		return fmt.Errorf("bluez: failed to power on adapter %s: %w", adapterPath, err)
	}

	log.Printf("bluez: powered on adapter %s\n", adapterPath)
	return nil
}

func (m *bluezManager) startScan(cb func(BLEScanResult)) error {
	m.mu.Lock()
	conn := m.conn
	adapterPath := m.adapterPath
	m.mu.Unlock()

	if conn == nil {
		return errors.New("bluez: not connected")
	}

	prefix := string(adapterPath) + "/"
	seen := 0

	// Emit devices BlueZ already knows about (e.g. seen in a previous scan,
	// or paired) before waiting for new advertisements.
	m.objMu.RLock()
	for path, ifaces := range m.objects {
		if !strings.HasPrefix(string(path), prefix) {
			continue
		}
		if props, ok := ifaces[device1Iface]; ok {
			seen++
			sr := newDBusScanResult(path, props)
			log.Printf("bluez: already known device %s (name=%q alias=%q)\n", sr.Address(), stringProp(props, "Name"), stringProp(props, "Alias"))
			cb(sr)
		}
	}
	m.objMu.RUnlock()
	log.Printf("bluez: %d previously known device(s) under %s\n", seen, adapterPath)

	unsubscribeAdded := m.onInterfacesAdded(func(path dbus.ObjectPath, ifaces map[string]map[string]dbus.Variant) {
		if !strings.HasPrefix(string(path), prefix) {
			return
		}
		props, ok := ifaces[device1Iface]
		if !ok {
			return
		}
		sr := newDBusScanResult(path, props)
		log.Printf("bluez: discovered device %s (name=%q alias=%q)\n", sr.Address(), stringProp(props, "Name"), stringProp(props, "Alias"))
		cb(sr)
	})

	// BlueZ often reports a device for the first time (InterfacesAdded) with
	// only its address - the advertised name shows up moments later as a
	// PropertiesChanged update. Without this, a base station whose name
	// resolves late would never be matched against the "LHB-" prefix filter.
	unsubscribeProps := m.onAnyPropertiesChanged(func(path dbus.ObjectPath, iface string, changed map[string]dbus.Variant) {
		if iface != device1Iface || !strings.HasPrefix(string(path), prefix) {
			return
		}
		_, gotName := changed["Name"]
		_, gotAlias := changed["Alias"]
		if !gotName && !gotAlias {
			return
		}

		m.objMu.RLock()
		props := m.objects[path][device1Iface]
		m.objMu.RUnlock()
		if props == nil {
			return
		}

		sr := newDBusScanResult(path, props)
		log.Printf("bluez: device %s name/alias updated (name=%q alias=%q)\n", sr.Address(), stringProp(props, "Name"), stringProp(props, "Alias"))
		cb(sr)
	})

	m.scanMu.Lock()
	if m.scanUnsubscribe != nil {
		m.scanUnsubscribe()
	}
	m.scanUnsubscribe = func() {
		unsubscribeAdded()
		unsubscribeProps()
	}
	m.scanMu.Unlock()

	adapterObj := conn.Object(bluezDest, adapterPath)
	call := adapterObj.Call(adapter1Iface+".StartDiscovery", 0)
	if call.Err != nil && !isDBusError(call.Err, bluezErrInProgress) {
		log.Printf("bluez: StartDiscovery on %s failed: %v\n", adapterPath, call.Err)
		return call.Err
	}
	log.Printf("bluez: discovery started on %s\n", adapterPath)
	return nil
}

func (m *bluezManager) stopScan() error {
	m.scanMu.Lock()
	unsub := m.scanUnsubscribe
	m.scanUnsubscribe = nil
	m.scanMu.Unlock()

	if unsub != nil {
		unsub()
	}

	m.mu.Lock()
	conn := m.conn
	adapterPath := m.adapterPath
	m.mu.Unlock()

	if conn == nil {
		return nil
	}

	adapterObj := conn.Object(bluezDest, adapterPath)
	if call := adapterObj.Call(adapter1Iface+".StopDiscovery", 0); call.Err != nil {
		log.Printf("bluez: StopDiscovery: %v\n", call.Err)
	}
	return nil
}

// lockAddress returns an unlock function guarding connect attempts for a
// single MAC address, so a scan-triggered connect and a reconnect racing for
// the same base station can't step on each other. Each base station gets its
// own lock, so connecting to (or reconnecting to) many base stations at once
// proceeds fully in parallel.
func (m *bluezManager) lockAddress(address string) func() {
	m.connectMu.Lock()
	l, ok := m.connectLocks[address]
	if !ok {
		l = &sync.Mutex{}
		m.connectLocks[address] = l
	}
	m.connectMu.Unlock()

	l.Lock()
	return l.Unlock
}

func (m *bluezManager) connectDevice(address string, timeout time.Duration) (BLEDevice, error) {
	normalized := strings.ToUpper(strings.TrimSpace(address))
	if !macAddressPattern.MatchString(normalized) {
		return nil, fmt.Errorf("bluez: invalid MAC address %q", address)
	}

	unlock := m.lockAddress(normalized)
	defer unlock()

	m.mu.Lock()
	conn := m.conn
	adapterPath := m.adapterPath
	m.mu.Unlock()

	if conn == nil {
		return nil, errors.New("bluez: not connected")
	}

	devicePath := devicePathForAddress(adapterPath, normalized)

	log.Printf("bluez: connecting to %s at %s...\n", normalized, devicePath)

	// The BLE connect and the subsequent GATT service-resolution walk are two
	// separate phases that BlueZ performs one after the other; each gets its
	// own full timeout budget instead of splitting a single deadline between
	// them; a slow (but successful) connect must not eat into the time left
	// to wait for services to resolve.
	connectCtx, connectCancel := context.WithTimeout(context.Background(), timeout)
	defer connectCancel()

	// Adapter1.ConnectDevice would let us connect by address without needing
	// the device to already be known, but it isn't part of every BlueZ build
	// (observed missing entirely on at least one current distro package) -
	// so instead make sure BlueZ actually knows about this device (running a
	// brief rediscovery scan if it doesn't, e.g. after a bluetoothd restart
	// forgot about it) and then connect to its device object directly, which
	// is standard core Device1 API present on every BlueZ version.
	if err := m.ensureDeviceKnown(connectCtx, adapterPath, devicePath, normalized); err != nil {
		return nil, err
	}

	deviceObj := conn.Object(bluezDest, devicePath)
	call := deviceObj.CallWithContext(connectCtx, device1Iface+".Connect", 0)
	if call.Err != nil && !isDBusError(call.Err, bluezErrAlreadyConnected) {
		return nil, fmt.Errorf("bluez: failed to connect to %s: %w", normalized, call.Err)
	}

	log.Printf("bluez: connected to %s, waiting for GATT services to resolve...\n", normalized)

	resolveCtx, resolveCancel := context.WithTimeout(context.Background(), timeout)
	defer resolveCancel()

	if err := m.waitServicesResolved(resolveCtx, devicePath); err != nil {
		return nil, err
	}

	log.Printf("bluez: GATT services resolved for %s\n", normalized)

	return newDBusDevice(m, devicePath, normalized), nil
}

func (m *bluezManager) deviceKnown(path dbus.ObjectPath) bool {
	m.objMu.RLock()
	defer m.objMu.RUnlock()
	_, ok := m.objects[path][device1Iface]
	return ok
}

// ensureDeviceKnown makes sure BlueZ has a Device1 object for devicePath,
// running a short discovery scan to (re)find it by address if it doesn't -
// this happens for a base station BlueZ saw and connected to previously but
// has since forgotten about, e.g. because bluetoothd was restarted (BlueZ
// only remembers unpaired devices for as long as it keeps running). Devices
// we've just scanned (the normal path when a new base station is found) are
// already known, so this returns immediately without touching discovery.
func (m *bluezManager) ensureDeviceKnown(ctx context.Context, adapterPath, devicePath dbus.ObjectPath, address string) error {
	if m.deviceKnown(devicePath) {
		return nil
	}

	log.Printf("bluez: %s not currently known to bluetoothd, scanning briefly to rediscover it...\n", address)

	m.mu.Lock()
	conn := m.conn
	m.mu.Unlock()

	found := make(chan struct{})
	var once sync.Once
	unsubscribe := m.onInterfacesAdded(func(path dbus.ObjectPath, ifaces map[string]map[string]dbus.Variant) {
		if path != devicePath {
			return
		}
		if _, ok := ifaces[device1Iface]; ok {
			once.Do(func() { close(found) })
		}
	})
	defer unsubscribe()

	// The device may have appeared between the check above and subscribing.
	if m.deviceKnown(devicePath) {
		return nil
	}

	adapterObj := conn.Object(bluezDest, adapterPath)
	call := adapterObj.CallWithContext(ctx, adapter1Iface+".StartDiscovery", 0)
	weStartedDiscovery := call.Err == nil
	if call.Err != nil && !isDBusError(call.Err, bluezErrInProgress) {
		return fmt.Errorf("bluez: failed to start discovery to rediscover %s: %w", address, call.Err)
	}

	defer func() {
		if !weStartedDiscovery {
			return
		}
		// Don't stop discovery out from under an unrelated in-progress scan
		// (e.g. the app's own periodic base-station scan).
		m.scanMu.Lock()
		scanStillWanted := m.scanUnsubscribe != nil
		m.scanMu.Unlock()
		if !scanStillWanted {
			if c := adapterObj.Call(adapter1Iface+".StopDiscovery", 0); c.Err != nil {
				log.Printf("bluez: StopDiscovery (rediscovery of %s): %v\n", address, c.Err)
			}
		}
	}()

	select {
	case <-found:
		return nil
	case <-ctx.Done():
		return fmt.Errorf("bluez: timed out rediscovering %s", address)
	}
}

func (m *bluezManager) waitServicesResolved(ctx context.Context, devicePath dbus.ObjectPath) error {
	if m.getBoolProperty(devicePath, device1Iface, "ServicesResolved") {
		return nil
	}

	done := make(chan struct{})
	var once sync.Once
	unsubscribe := m.onPropertiesChanged(devicePath, func(iface string, changed map[string]dbus.Variant) {
		if iface != device1Iface {
			return
		}
		if v, ok := changed["ServicesResolved"]; ok {
			if resolved, ok := v.Value().(bool); ok && resolved {
				once.Do(func() { close(done) })
			}
		}
	})
	defer unsubscribe()

	// Guard against ServicesResolved flipping to true in the window between
	// the initial check above and the subscription just above taking effect.
	if m.getBoolProperty(devicePath, device1Iface, "ServicesResolved") {
		return nil
	}

	select {
	case <-done:
		return nil
	case <-ctx.Done():
		return fmt.Errorf("bluez: timed out waiting for GATT services to resolve on %s", devicePath)
	}
}

func (m *bluezManager) getBoolProperty(path dbus.ObjectPath, iface, name string) bool {
	m.objMu.RLock()
	defer m.objMu.RUnlock()
	props, ok := m.objects[path][iface]
	if !ok {
		return false
	}
	v, ok := props[name]
	if !ok {
		return false
	}
	b, _ := v.Value().(bool)
	return b
}

func devicePathForAddress(adapterPath dbus.ObjectPath, mac string) dbus.ObjectPath {
	return dbus.ObjectPath(string(adapterPath) + "/dev_" + strings.ReplaceAll(mac, ":", "_"))
}

func macFromDevicePath(path dbus.ObjectPath) string {
	segments := strings.Split(string(path), "/")
	last := segments[len(segments)-1]
	if !strings.HasPrefix(last, "dev_") {
		return ""
	}
	return strings.ReplaceAll(strings.TrimPrefix(last, "dev_"), "_", ":")
}

func stringProp(props map[string]dbus.Variant, key string) string {
	v, ok := props[key]
	if !ok {
		return ""
	}
	s, _ := v.Value().(string)
	return s
}

func containsStr(list []string, needle string) bool {
	for _, v := range list {
		if v == needle {
			return true
		}
	}
	return false
}

// dbus.Error's Error() method returns the human-readable message from the
// error reply body when present (e.g. `Method "ConnectDevice" with signature
// "a{sv}" on interface "org.bluez.Adapter1" doesn't exist`), NOT the actual
// D-Bus error name (e.g. "org.freedesktop.DBus.Error.UnknownMethod") - so
// matching on err.Error() essentially never finds the name we're looking for.
// Compare against the structured Name field instead.
func isDBusError(err error, name string) bool {
	var derr dbus.Error
	if errors.As(err, &derr) {
		return derr.Name == name
	}
	return false
}

const (
	bluezErrAlreadyConnected = "org.bluez.Error.AlreadyConnected"
	bluezErrInProgress       = "org.bluez.Error.InProgress"
)

type dbusAdapter struct{}

func (a *dbusAdapter) Enable() error {
	if err := bluez.ensureConnected(); err != nil {
		return err
	}
	return bluez.powerOn()
}

func (a *dbusAdapter) Scan(callback func(BLEAdapter, BLEScanResult)) error {
	if err := bluez.ensureConnected(); err != nil {
		return err
	}
	return bluez.startScan(func(sr BLEScanResult) { callback(a, sr) })
}

func (a *dbusAdapter) StopScan() error {
	return bluez.stopScan()
}

func (a *dbusAdapter) Connect(address string, timeout time.Duration) (BLEDevice, error) {
	if err := bluez.ensureConnected(); err != nil {
		return nil, err
	}
	return bluez.connectDevice(address, timeout)
}

type dbusScanResult struct {
	name    string
	address string
}

func newDBusScanResult(path dbus.ObjectPath, props map[string]dbus.Variant) *dbusScanResult {
	name := stringProp(props, "Name")
	if name == "" {
		name = stringProp(props, "Alias")
	}
	address := stringProp(props, "Address")
	if address == "" {
		address = macFromDevicePath(path)
	}
	return &dbusScanResult{name: name, address: address}
}

func (r *dbusScanResult) LocalName() string { return r.name }
func (r *dbusScanResult) Address() string   { return r.address }

type dbusDevice struct {
	m       *bluezManager
	path    dbus.ObjectPath
	address string

	mu           sync.Mutex
	onDisconnect func()
	unsubscribe  func()
}

func newDBusDevice(m *bluezManager, path dbus.ObjectPath, address string) *dbusDevice {
	d := &dbusDevice{m: m, path: path, address: address}
	d.unsubscribe = m.onPropertiesChanged(path, func(iface string, changed map[string]dbus.Variant) {
		if iface != device1Iface {
			return
		}
		v, ok := changed["Connected"]
		if !ok {
			return
		}
		if connected, ok := v.Value().(bool); !ok || connected {
			return
		}
		d.mu.Lock()
		cb := d.onDisconnect
		d.mu.Unlock()
		if cb != nil {
			cb()
		}
	})
	return d
}

// OnDisconnected implements optionalDisconnectWatcher (see bs.go), letting
// bs.go wire BlueZ's disconnect signal straight into the regular Reconnect
// path instead of only noticing on the next failed read/write.
func (d *dbusDevice) OnDisconnected(callback func()) {
	d.mu.Lock()
	d.onDisconnect = callback
	d.mu.Unlock()
}

func (d *dbusDevice) Address() string { return d.address }

func (d *dbusDevice) Disconnect() error {
	if d.unsubscribe != nil {
		d.unsubscribe()
	}
	obj := d.m.conn.Object(bluezDest, d.path)
	call := obj.Call(device1Iface+".Disconnect", 0)
	return call.Err
}

func (d *dbusDevice) DiscoverServices() ([]BLEService, error) {
	prefix := string(d.path) + "/"

	d.m.objMu.RLock()
	defer d.m.objMu.RUnlock()

	var services []BLEService
	for path, ifaces := range d.m.objects {
		if !strings.HasPrefix(string(path), prefix) {
			continue
		}
		props, ok := ifaces[gattService1Iface]
		if !ok {
			continue
		}
		services = append(services, &dbusService{m: d.m, path: path, uuid: stringProp(props, "UUID")})
	}
	return services, nil
}

type dbusService struct {
	m    *bluezManager
	path dbus.ObjectPath
	uuid string
}

func (s *dbusService) UUID() string { return s.uuid }

func (s *dbusService) DiscoverCharacteristics() ([]BLECharacteristic, error) {
	prefix := string(s.path) + "/"

	s.m.objMu.RLock()
	defer s.m.objMu.RUnlock()

	var chars []BLECharacteristic
	for path, ifaces := range s.m.objects {
		if !strings.HasPrefix(string(path), prefix) {
			continue
		}
		props, ok := ifaces[gattChar1Iface]
		if !ok {
			continue
		}
		chars = append(chars, &dbusCharacteristic{m: s.m, path: path, uuid: stringProp(props, "UUID")})
	}
	return chars, nil
}

type dbusCharacteristic struct {
	m    *bluezManager
	path dbus.ObjectPath
	uuid string
}

func (c *dbusCharacteristic) UUID() string { return c.uuid }

func (c *dbusCharacteristic) Read() ([]byte, error) {
	obj := c.m.conn.Object(bluezDest, c.path)
	var data []byte
	if err := obj.Call(gattChar1Iface+".ReadValue", 0, map[string]dbus.Variant{}).Store(&data); err != nil {
		return nil, err
	}
	return data, nil
}

func (c *dbusCharacteristic) writeValue(value []byte, writeType string) (int, error) {
	obj := c.m.conn.Object(bluezDest, c.path)
	call := obj.Call(gattChar1Iface+".WriteValue", 0, value, map[string]dbus.Variant{
		"type": dbus.MakeVariant(writeType),
	})
	if call.Err != nil {
		return 0, call.Err
	}
	return len(value), nil
}

func (c *dbusCharacteristic) Write(value []byte) (int, error) {
	return c.writeValue(value, "request")
}

func (c *dbusCharacteristic) WriteWithoutResponse(value []byte) (int, error) {
	return c.writeValue(value, "command")
}

func (c *dbusCharacteristic) EnableNotifications(callback func(buf []byte)) error {
	obj := c.m.conn.Object(bluezDest, c.path)
	if call := obj.Call(gattChar1Iface+".StartNotify", 0); call.Err != nil {
		return call.Err
	}

	c.m.onPropertiesChanged(c.path, func(iface string, changed map[string]dbus.Variant) {
		if iface != gattChar1Iface {
			return
		}
		v, ok := changed["Value"]
		if !ok {
			return
		}
		if b, ok := v.Value().([]byte); ok {
			callback(b)
		}
	})
	return nil
}
