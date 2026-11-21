//! Lighthouse-only GATT access. Reuse the Windows service cache and skip
//! unrelated services and descriptors; read live channel/power values uncached.

use anyhow::{ensure, Context, Result};
use btleplug::{api::{BDAddr, Peripheral as _}, platform::Peripheral};
use windows::{
    core::GUID,
    Devices::Bluetooth::{
        BluetoothCacheMode, BluetoothLEDevice,
        GenericAttributeProfile::{
            GattCharacteristic, GattCommunicationStatus, GattDeviceService, GattSession,
            GattWriteOption,
        },
    },
    Storage::Streams::{DataReader, DataWriter},
};

const SERVICE: GUID = GUID::from_u128(0x00001523_1212_efde_1523_785feabcd124);
const MODE: GUID = GUID::from_u128(0x00001524_1212_efde_1523_785feabcd124);
const POWER: GUID = GUID::from_u128(0x00001525_1212_efde_1523_785feabcd124);
const IDENTIFY: GUID = GUID::from_u128(0x00008421_1212_efde_1523_785feabcd124);

pub(super) struct Device {
    address: BDAddr,
    device: BluetoothLEDevice,
    session: GattSession,
    service: Option<GattDeviceService>,
    mode: Option<GattCharacteristic>,
    power: Option<GattCharacteristic>,
    identify: Option<GattCharacteristic>,
}

impl Device {
    pub(super) async fn connect(peripheral: Peripheral) -> Result<Self> {
        let address = peripheral.address();
        let device = BluetoothLEDevice::FromBluetoothAddressAsync(address.into())?.await?;
        let session = match async {
            GattSession::FromDeviceIdAsync(&device.BluetoothDeviceId()?)?.await
        }.await {
            Ok(session) => session,
            Err(e) => {
                let _ = device.Close();
                return Err(e.into());
            }
        };
        let mut connection = Self {
            address, device, session, service: None,
            mode: None, power: None, identify: None,
        };
        connection.session.SetMaintainConnection(true)?;
        let result = connection.device
            .GetGattServicesForUuidWithCacheModeAsync(SERVICE, BluetoothCacheMode::Cached)?
            .await?;
        check_status(result.Status()?, "lighthouse service discovery")?;
        connection.service = Some(result.Services()?.GetAt(0).context("lighthouse service not found")?);
        let result = connection.service.as_ref().unwrap()
            .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Cached)?.await?;
        check_status(result.Status()?, "lighthouse characteristic discovery")?;
        for characteristic in result.Characteristics()? {
            let uuid = characteristic.Uuid()?;
            if uuid == MODE {
                connection.mode = Some(characteristic);
            } else if uuid == POWER {
                connection.power = Some(characteristic);
            } else if uuid == IDENTIFY {
                connection.identify = Some(characteristic);
            }
        }
        ensure!(connection.mode.is_some() && connection.power.is_some(),
            "lighthouse channel/power characteristics not found");
        Ok(connection)
    }

    pub(super) fn address(&self) -> BDAddr { self.address }

    pub(super) async fn read_channel(&self) -> Result<u8> { read_u8(self.mode.as_ref()).await }
    pub(super) async fn read_power(&self) -> Result<u8> { read_u8(self.power.as_ref()).await }
    pub(super) async fn write_channel(&self, value: u8) -> Result<()> { write_u8(self.mode.as_ref(), value).await }
    pub(super) async fn write_power(&self, value: u8) -> Result<()> { write_u8(self.power.as_ref(), value).await }
    pub(super) async fn identify(&self) -> Result<()> { write_u8(self.identify.as_ref(), 1).await }
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Some(service) = &self.service { let _ = service.Close(); }
        let _ = self.session.SetMaintainConnection(false);
        let _ = self.session.Close();
        let _ = self.device.Close();
    }
}

fn check_status(status: GattCommunicationStatus, operation: &str) -> Result<()> {
    ensure!(status == GattCommunicationStatus::Success, "{operation}: {status:?}");
    Ok(())
}

async fn read_u8(characteristic: Option<&GattCharacteristic>) -> Result<u8> {
    let characteristic = characteristic.context("characteristic not found")?;
    let result = characteristic.ReadValueWithCacheModeAsync(BluetoothCacheMode::Uncached)?.await?;
    check_status(result.Status()?, "read characteristic")?;
    Ok(DataReader::FromBuffer(&result.Value()?)?.ReadByte()?)
}

async fn write_u8(characteristic: Option<&GattCharacteristic>, value: u8) -> Result<()> {
    let characteristic = characteristic.context("characteristic not found")?;
    let writer = DataWriter::new()?;
    writer.WriteByte(value)?;
    let result = characteristic.WriteValueWithResultAndOptionAsync(
        &writer.DetachBuffer()?, GattWriteOption::WriteWithoutResponse,
    )?.await?;
    check_status(result.Status()?, "write characteristic")
}
