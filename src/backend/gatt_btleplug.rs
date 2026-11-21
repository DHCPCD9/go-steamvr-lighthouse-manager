//! GATT access through btleplug on Linux and macOS.

use anyhow::{Context, Result};
use btleplug::{
    api::{BDAddr, Characteristic, Peripheral as _, WriteType},
    platform::Peripheral,
};
use uuid::Uuid;

const MODE: Uuid = Uuid::from_u128(0x00001524_1212_efde_1523_785feabcd124);
const POWER: Uuid = Uuid::from_u128(0x00001525_1212_efde_1523_785feabcd124);
const IDENTIFY: Uuid = Uuid::from_u128(0x00008421_1212_efde_1523_785feabcd124);

pub(super) struct Device {
    peripheral: Peripheral,
    mode: Option<Characteristic>,
    power: Option<Characteristic>,
    identify: Option<Characteristic>,
}

impl Device {
    pub(super) async fn connect(peripheral: Peripheral) -> Result<Self> {
        peripheral.connect().await?;
        if let Err(e) = peripheral.discover_services().await {
            let _ = peripheral.disconnect().await;
            return Err(e.into());
        }
        let mut device = Self { peripheral, mode: None, power: None, identify: None };
        for characteristic in device.peripheral.characteristics() {
            let uuid = characteristic.uuid;
            if uuid == MODE {
                device.mode = Some(characteristic);
            } else if uuid == POWER {
                device.power = Some(characteristic);
            } else if uuid == IDENTIFY {
                device.identify = Some(characteristic);
            }
        }
        Ok(device)
    }

    pub(super) fn address(&self) -> BDAddr { self.peripheral.address() }

    pub(super) async fn read_channel(&self) -> Result<u8> { self.read_u8(self.mode.as_ref()).await }
    pub(super) async fn read_power(&self) -> Result<u8> { self.read_u8(self.power.as_ref()).await }
    pub(super) async fn write_channel(&self, value: u8) -> Result<()> { self.write_u8(self.mode.as_ref(), value).await }
    pub(super) async fn write_power(&self, value: u8) -> Result<()> { self.write_u8(self.power.as_ref(), value).await }
    pub(super) async fn identify(&self) -> Result<()> { self.write_u8(self.identify.as_ref(), 1).await }

    async fn read_u8(&self, characteristic: Option<&Characteristic>) -> Result<u8> {
        let characteristic = characteristic.context("characteristic not found")?;
        let data = self.peripheral.read(characteristic).await?;
        data.first().copied().context("empty characteristic value")
    }

    async fn write_u8(&self, characteristic: Option<&Characteristic>, value: u8) -> Result<()> {
        let characteristic = characteristic.context("characteristic not found")?;
        Ok(self.peripheral.write(characteristic, &[value], WriteType::WithoutResponse).await?)
    }
}
