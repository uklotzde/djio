// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#![allow(rustdoc::invalid_rust_codeblocks)]
#![doc = include_str!("../README.md")]

use std::{
    fmt,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use smol_str::{SmolStr, format_smolstr};

mod controller;
#[cfg(all(feature = "midi", feature = "controller-thread"))]
pub use self::controller::midi::context::SingleMidiControllerContext;
#[cfg(feature = "midi")]
pub use self::controller::midi::{BoxedMidiController, MidiController};
#[cfg(feature = "controller-thread")]
pub use self::controller::thread::ControllerThread;
pub use self::controller::{
    BoxedControllerTask, Controller, ControllerDescriptor, ControllerTypes,
};

pub mod devices;

mod input;
pub use self::input::{
    ButtonInput, CenterSliderInput, ControlInputEvent, ControlInputEventSink, CrossfaderCurve,
    InputEvent, PadButtonInput, SelectorInput, SliderEncoderInput, SliderInput, StepEncoderInput,
    input_events_ordered_chronologically, split_crossfader_input_amplitude_preserving_approx,
    split_crossfader_input_energy_preserving_approx, split_crossfader_input_linear,
    split_crossfader_input_square,
};

mod output;
#[cfg(feature = "blinking-led-task")]
pub use self::output::blinking_led_task;
#[cfg(feature = "blinking-led-task-tokio-rt")]
pub use self::output::spawn_blinking_led_task;
pub use self::output::{
    BlinkingLedOutput, BlinkingLedTicker, ControlOutputGateway, DEFAULT_BLINKING_LED_PERIOD,
    DimLedOutput, LedOutput, LedState, OutputError, OutputResult, RgbLedOutput, SendOutputsError,
    VirtualLed,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInterfaceDescriptor {
    pub num_input_channels: u8,
    pub num_output_channels: u8,
}

/// Common, information properties about a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub vendor_name: SmolStr,
    pub product_name: SmolStr,
    pub audio_interface: Option<AudioInterfaceDescriptor>,
}

impl DeviceDescriptor {
    /// The qualified device name including both vendor and model name.
    #[must_use]
    pub fn name(&self) -> SmolStr {
        let Self {
            vendor_name,
            product_name,
            ..
        } = self;
        debug_assert!(!product_name.is_empty());
        if vendor_name.is_empty() {
            product_name.clone()
        } else {
            format_smolstr!("{vendor_name} {product_name}")
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
        loop {
            let prev_value = self.0.fetch_add(1, Ordering::Relaxed);
            // fetch_add() wraps around on overflow
            let next_value = prev_value.wrapping_add(1);
            if next_value != PortIndex::INVALID.value() {
                return PortIndex::new(next_value);
            }
        }
    }
}

impl Default for PortIndexGenerator {
    fn default() -> Self {
        Self::new()
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
pub struct Control {
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
    (u16::from(hi) << 7) | u16::from(lo)
}

#[cfg(all(feature = "hid", not(target_family = "wasm")))]
pub mod hid;
#[cfg(all(feature = "hid", not(target_family = "wasm")))]
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
    BoxedMidiOutputConnection, MidiControlOutputGateway, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputDecodeError, MidiInputEventDecoder, MidiInputGateway, MidiInputHandler,
    MidiOutputConnection, MidiOutputGateway, MidiPortDescriptor, NewMidiInputGateway,
    consume_midi_input_event,
};

pub mod deck;
#[cfg(feature = "observables")]
pub use self::deck::Observables as DeckObservables;
pub use self::deck::{Adapter as DeckAdapter, Input as DeckInput};

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
