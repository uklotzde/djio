// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    borrow::Cow,
    collections::HashSet,
    ops::{Deref, DerefMut},
    time::Duration,
};

use hidapi::DeviceInfo;
use thiserror::Error;

pub mod report;

pub mod thread;
pub use thread::HidThread;

#[derive(Debug, Error)]
pub enum HidDeviceError {
    #[error("Device not connected")]
    NotConnected,

    #[error("Device not supported")]
    NotSupported,
}

#[derive(Debug, Error)]
pub enum HidError {
    #[error(transparent)]
    Device(#[from] HidDeviceError),

    #[error(transparent)]
    Api(#[from] hidapi::HidError),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type HidResult<T> = std::result::Result<T, HidError>;

/// <https://www.usb.org/document-library/hid-usage-tables-13>
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum HidUsagePage {
    Undefined,
    GenericDesktop,
    SimulationControls,
    VRControls,
    SportControls,
    GameControls,
    GenericDeviceControls,
    Keyboard,
    LED,
    Button,
    Ordinal,
    Telephony,
    Consumer,
    Digitizer,
    Haptics,
    PhysicalInput,
    Unicode,
    EyeAndHeadTracker,
    AuxiliaryDisplay,
    Sensors,
    MedicalInstrument,
    BrailleDisplay,
    Light,
    Monitor,
    MonitorEnumerated,
    VESAVirtualControls,
    Power,
    BatterySystem,
    BarcodeScanner,
    Scale,
    MagneticStripeReader,
    CameraControl,
    Arcade,
    GamingDevice,
    FIDO,
    Reserved(u16),
    VendorDefined(u16),
}

impl From<u16> for HidUsagePage {
    fn from(number: u16) -> Self {
        #[allow(clippy::match_same_arms)]
        match number {
            0x00 => Self::Undefined,
            0x01 => Self::GenericDesktop,
            0x02 => Self::SimulationControls,
            0x03 => Self::VRControls,
            0x04 => Self::SportControls,
            0x05 => Self::GameControls,
            0x06 => Self::GenericDeviceControls,
            0x07 => Self::Keyboard,
            0x08 => Self::LED,
            0x09 => Self::Button,
            0x0a => Self::Ordinal,
            0x0b => Self::Telephony,
            0x0c => Self::Consumer,
            0x0d => Self::Digitizer,
            0x0e => Self::Haptics,
            0x0f => Self::PhysicalInput,
            0x10 => Self::Unicode,
            0x11 => Self::Reserved(number),
            0x12 => Self::EyeAndHeadTracker,
            0x13 => Self::Reserved(number),
            0x14 => Self::AuxiliaryDisplay,
            0x15..=0x1f => Self::Reserved(number),
            0x20 => Self::Sensors,
            0x21..=0x3f => Self::Reserved(number),
            0x40 => Self::MedicalInstrument,
            0x41 => Self::BrailleDisplay,
            0x42..=0x58 => Self::Reserved(number),
            0x59 => Self::Light,
            0x5a..=0x7f => Self::Reserved(number),
            0x80 => Self::Monitor,
            0x81 => Self::MonitorEnumerated,
            0x82 => Self::VESAVirtualControls,
            0x83 => Self::Reserved(number),
            0x84 => Self::Power,
            0x85 => Self::BatterySystem,
            0x86..=0x8b => Self::Reserved(number),
            0x8c => Self::BarcodeScanner,
            0x8d => Self::Scale,
            0x8e => Self::MagneticStripeReader,
            0x8f => Self::Reserved(number),
            0x90 => Self::CameraControl,
            0x91 => Self::Arcade,
            0x92 => Self::GamingDevice,
            0x93..=0xf1cf => Self::Reserved(number),
            0xf1d0 => Self::FIDO,
            0xf1d1..=0xfeff => Self::Reserved(number),
            0xff00..=0xffff => Self::VendorDefined(number),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct HidApi(hidapi::HidApi);

impl AsRef<hidapi::HidApi> for HidApi {
    fn as_ref(&self) -> &hidapi::HidApi {
        let Self(inner) = self;
        inner
    }
}

impl AsMut<hidapi::HidApi> for HidApi {
    fn as_mut(&mut self) -> &mut hidapi::HidApi {
        let Self(inner) = self;
        inner
    }
}

impl Deref for HidApi {
    type Target = hidapi::HidApi;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for HidApi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl HidApi {
    pub fn new() -> HidResult<Self> {
        let inner = hidapi::HidApi::new_without_enumerate()?;
        Ok(Self(inner))
    }

    pub fn query_devices(&mut self) -> HidResult<impl Iterator<Item = &DeviceInfo>> {
        self.refresh_devices()?;
        Ok(self.device_list())
    }

    pub fn query_devices_dedup(&mut self) -> HidResult<Vec<HidDevice>> {
        let mut visited_paths = HashSet::new();
        Ok(self
            .query_devices()?
            .filter_map(|info| {
                visited_paths
                    .insert(info.path())
                    .then(|| HidDevice::new(info.clone()))
            })
            .collect())
    }

    pub fn query_device_by_id(&mut self, id: &DeviceId<'_>) -> HidResult<Option<HidDevice>> {
        Ok(self.query_devices()?.find_map(|info| {
            let found_id = DeviceId::try_from(info).ok();
            if Some(id) == found_id.as_ref() {
                return Some(HidDevice::new(info.clone()));
            }
            None
        }))
    }

    pub fn connect_device(&self, info: DeviceInfo) -> HidResult<HidDevice> {
        let mut device = HidDevice::new(info);
        device.connect(self)?;
        Ok(device)
    }
}

#[allow(missing_debug_implementations)]
pub struct HidDevice {
    info: DeviceInfo,

    connected: Option<hidapi::HidDevice>,
}

/// Permanent, connection-independent device identifier.
///
/// Could be used for referencing devices persistently, e.g. in configurations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId<'a> {
    /// Vendor id
    pub vid: u16,

    /// Product id
    pub pid: u16,

    /// Non-empty serial number
    pub sn: Cow<'a, str>,
}

impl DeviceId<'_> {
    #[must_use]
    pub fn into_owned(self) -> DeviceId<'static> {
        let Self { vid, pid, sn } = self;
        DeviceId {
            vid,
            pid,
            sn: Cow::Owned(sn.into_owned()),
        }
    }
}

impl<'a> TryFrom<&'a DeviceInfo> for DeviceId<'a> {
    type Error = ();

    #[allow(clippy::similar_names)]
    fn try_from(from: &'a DeviceInfo) -> std::result::Result<Self, Self::Error> {
        if let Some(sn) = from.serial_number() {
            let sn = sn.trim();
            if !sn.is_empty() {
                let vid = from.vendor_id();
                let pid = from.product_id();
                let sn = Cow::Borrowed(sn);
                return Ok(Self { vid, pid, sn });
            }
        }
        Err(())
    }
}

impl HidDevice {
    #[must_use]
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            info,
            connected: None,
        }
    }

    #[must_use]
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected.is_some()
    }

    pub fn connect(&mut self, api: &HidApi) -> HidResult<()> {
        if self.is_connected() {
            return Ok(());
        }
        let connected = api.0.open_path(self.info.path())?;
        // Blocking is controlled explicitly by a timeout with each read request.
        // The following function is only called as a safeguard to ensure a consistent
        // initial state.
        connected.set_blocking_mode(true)?;
        self.connected = Some(connected);
        debug_assert!(self.is_connected());
        Ok(())
    }

    pub fn disconnect(&mut self) {
        // The optional `HidDevice` will implicitly be dropped and closed by the assignment.
        self.connected = None;
        debug_assert!(!self.is_connected());
    }

    fn connected(&self) -> HidResult<&hidapi::HidDevice> {
        self.connected
            .as_ref()
            .ok_or(HidDeviceError::NotConnected.into())
    }

    pub fn get_feature_report(&self, buffer: &mut [u8]) -> HidResult<usize> {
        Ok(self.connected()?.get_feature_report(buffer)?)
    }

    pub fn send_feature_report(&self, data: &[u8]) -> HidResult<()> {
        Ok(self.connected()?.send_feature_report(data)?)
    }

    /// Blocking read into buffer with optional timeout (millisecond precision).
    pub fn read(&self, buffer: &mut [u8], timeout: Option<Duration>) -> HidResult<usize> {
        let timeout_millis = timeout_millis(timeout);
        Ok(self.connected()?.read_timeout(buffer, timeout_millis)?)
    }

    pub fn write(&self, data: &[u8]) -> HidResult<usize> {
        Ok(self.connected()?.write(data)?)
    }
}

const INF_TIMEOUT_MILLIS: i32 = -1;
const MAX_TIMEOUT_MILLIS: i32 = i32::MAX;

#[allow(clippy::cast_possible_truncation)]
fn timeout_millis(timeout: Option<Duration>) -> i32 {
    // Verify that the timeout is specified in full milliseconds
    // to prevent losing precision unintentionally.
    debug_assert_eq!(0, timeout.unwrap_or_default().subsec_nanos() % 1_000_000);
    timeout
            .as_ref()
            .map(Duration::as_millis)
            // Saturating conversion from u128 to i32
            .map_or(INF_TIMEOUT_MILLIS, |millis| millis.min(MAX_TIMEOUT_MILLIS as _) as _)
}
