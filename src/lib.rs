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

use std::{
    borrow::Cow,
    fmt,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

pub mod devices;

mod input;
pub use self::input::{
    input_events_ordered_chronologically, ButtonInput, CenterSliderInput, ControlInputEvent,
    ControlInputEventSink, InputEvent, PadButtonInput, SelectorInput, SliderEncoderInput,
    SliderInput, StepEncoderInput,
};

mod output;
pub use self::output::{
    ControlOutputGateway, DimLedOutput, LedOutput, OutputError, OutputResult, RgbLedOutput,
    SendOutputsError,
};

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

/// Index for addressing multiple, connected devices.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display,
)]
#[repr(transparent)]
pub struct PortIndex {
    value: u32,
}

impl PortIndex {
    pub const INVALID: Self = Self::new(0);
    pub const MIN: Self = Self::new(1);
    pub const MAX: Self = Self::new(u32::MAX);

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

    #[must_use]
    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }

    #[must_use]
    pub fn next(self) -> Self {
        let Self { mut value } = self;
        let next_value = loop {
            let next_value = value.wrapping_add(1);
            if next_value != Self::INVALID.value() {
                break next_value;
            }
            value = next_value;
        };
        let next = Self { value: next_value };
        debug_assert!(next.is_valid());
        next
    }
}

/// Thread-safe [`PortIndex`] generator
#[derive(Debug)]
pub struct PortIndexGenerator(AtomicU32);

impl PortIndexGenerator {
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicU32::new(PortIndex::INVALID.value()))
    }

    #[must_use]
    pub fn next(&self) -> PortIndex {
        let mut old_value = self.0.load(Ordering::Relaxed);
        loop {
            let next = PortIndex::new(old_value).next();
            match self.0.compare_exchange_weak(
                old_value,
                next.value(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(value) => {
                    debug_assert_eq!(old_value, value);
                    return next;
                }
                Err(value) => {
                    debug_assert_ne!(old_value, value);
                    old_value = value;
                }
            }
        }
    }
}

/// Index for addressing either or both device inputs and outputs
/// in a generic manner.
///
/// Only valid in the scope of a single device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
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
pub use self::hid::{
    HidApi, HidDevice, HidDeviceError, HidError, HidResult, HidThread, HidUsagePage,
};

#[cfg(feature = "midi")]
mod midi;

#[cfg(feature = "midir")]
pub use self::midi::midir::{
    MidiPortError, MidirDevice, MidirDeviceManager, MidirInputPort, MidirOutputPort,
};
#[cfg(feature = "midi")]
pub use self::midi::{
    consume_midi_input_event, MidiControlOutputGateway, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputDecodeError, MidiInputEventDecoder, MidiInputGateway, MidiInputHandler,
    MidiOutputConnection, MidiOutputGateway, MidiPortDescriptor, NewMidiInputGateway,
};

#[cfg(feature = "experimental-param")]
pub mod param;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_index() {
        assert_eq!(PortIndex::INVALID, PortIndex::default());
    }

    #[test]
    fn next_port_index() {
        assert_eq!(PortIndex::MIN, PortIndex::INVALID.next());
        assert!(PortIndex::MIN < PortIndex::MIN.next());
        assert_eq!(PortIndex::MIN, PortIndex::MAX.next());
    }
}
