// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#![allow(rustdoc::invalid_rust_codeblocks)]
#![doc = include_str!("../README.md")]
#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
//#![warn(missing_docs)] // FIXME
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(rustdoc::broken_intra_doc_links)]
// Repetitions of module/type names occur frequently when using many
// modules for keeping the size of the source files handy. Often
// types have the same name as their parent module.
#![allow(clippy::module_name_repetitions)]
// Repeating the type name in `..Default::default()` expressions
// is not needed since the context is obvious.
#![allow(clippy::default_trait_access)]
#![allow(clippy::missing_errors_doc)] // FIXME

use std::{borrow::Cow, fmt, time::Duration};

pub mod devices;

mod input;
pub use self::input::{
    ButtonInput, CenterSliderInput, ControlInputEvent, EmitInputEvent, InputEvent, PadButtonInput,
    SliderEncoderInput, SliderInput, StepEncoderInput,
};

mod output;
pub use self::output::{DimLedOutput, LedOutput, OutputError, OutputResult, RgbLedOutput};

/// Common, information properties about a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub vendor_name: Cow<'static, str>,
    pub product_name: Cow<'static, str>,
}

impl DeviceDescriptor {
    /// The qualified device name including both vendor and model name.
    #[must_use]
    pub fn name(&self) -> Cow<'static, str> {
        let Self {
            vendor_name,
            product_name,
            ..
        } = self;
        debug_assert!(!product_name.is_empty());
        if vendor_name.is_empty() {
            product_name.clone()
        } else {
            format!("{vendor_name} {product_name}").into()
        }
    }
}

/// Index for addressing either or both device inputs and outputs
/// in a generic manner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ControlIndex {
    value: u32,
}

impl ControlIndex {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self { value }
    }

    #[must_use]
    pub const fn value(self) -> u32 {
        let Self { value } = self;
        value
    }

    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.value() as usize
    }
}

/// A generic, encoded control value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ControlValue {
    bits: u32,
}

impl ControlValue {
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    #[must_use]
    pub const fn to_bits(self) -> u32 {
        let Self { bits } = self;
        bits
    }
}

/// Generic, indexed input/output control value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRegister {
    pub index: ControlIndex,
    pub value: ControlValue,
}

/// Time stamp with microsecond precision
///
/// The actual value has no meaning, i.e. the origin with value 0 is arbitrary.
/// Only the difference between two time stamps should be considered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeStamp(u64);

impl TimeStamp {
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    #[must_use]
    pub const fn to_micros(self) -> u64 {
        let Self(micros) = self;
        micros
    }

    #[must_use]
    pub const fn to_duration(self) -> Duration {
        Duration::from_micros(self.to_micros())
    }
}

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{micros} \u{00B5}s",
            micros = self.to_micros()
        ))
    }
}

/// A commonly needed conversion for MIDI and (maybe other) devices.
#[must_use]
pub fn u7_be_to_u14(hi: u8, lo: u8) -> u16 {
    debug_assert_eq!(hi, hi & 0x7f);
    debug_assert_eq!(lo, lo & 0x7f);
    u16::from(hi) << 7 | u16::from(lo)
}

#[cfg(feature = "hid")]
pub mod hid;

#[cfg(feature = "hid")]
pub use hid::{HidApi, HidDevice, HidDeviceError, HidError, HidResult, HidThread, HidUsagePage};

#[cfg(feature = "midi")]
mod midi;

#[cfg(feature = "midi")]
pub use self::midi::{
    GenericMidiDevice, GenericMidirDeviceManager, MidiDevice, MidiDeviceDescriptor,
    MidiInputReceiver, MidiPortError, MidirDevice, MidirDeviceManager, MidirInputConnector,
};
