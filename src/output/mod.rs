// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Sending control data to actuators like LEDs and motorized platters
//! or for configuring devices.

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    time::Duration,
};

use futures_core::Stream;
use futures_util::{stream, StreamExt as _};
use strum::FromRepr;
use thiserror::Error;

use crate::{Control, ControlValue};

#[cfg(feature = "blinking-led-task")]
mod blinking_led_task;
#[cfg(feature = "blinking-led-task")]
pub use blinking_led_task::blinking_led_task;
#[cfg(feature = "blinking-led-task-tokio-rt")]
pub use blinking_led_task::spawn_blinking_led_task;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("disconnected")]
    Disconnected,
    #[error("send: {msg}")]
    Send { msg: Cow<'static, str> },
}

pub type OutputResult<T> = std::result::Result<T, OutputError>;

/// Simple LED
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u8)]
pub enum LedOutput {
    Off = 0,
    On = 1,
}

impl From<LedOutput> for ControlValue {
    fn from(value: LedOutput) -> Self {
        Self::from_bits(value as _)
    }
}

impl From<ControlValue> for LedOutput {
    fn from(value: ControlValue) -> Self {
        match value.to_bits() {
            0 => Self::Off,
            _ => Self::On,
        }
    }
}

/// Dimmable LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct DimLedOutput {
    pub brightness: u8,
}

impl From<DimLedOutput> for ControlValue {
    fn from(value: DimLedOutput) -> Self {
        let DimLedOutput { brightness } = value;
        Self::from_bits(u32::from(brightness))
    }
}

impl From<ControlValue> for DimLedOutput {
    fn from(value: ControlValue) -> Self {
        let brightness = (value.to_bits() & 0xff) as u8;
        Self { brightness }
    }
}

/// RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbLedOutput {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl From<RgbLedOutput> for ControlValue {
    fn from(value: RgbLedOutput) -> Self {
        let RgbLedOutput { red, green, blue } = value;
        Self::from_bits(u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue))
    }
}

impl From<ControlValue> for RgbLedOutput {
    fn from(value: ControlValue) -> Self {
        let red = ((value.to_bits() >> 16) & 0xff) as u8;
        let green = ((value.to_bits() >> 8) & 0xff) as u8;
        let blue = (value.to_bits() & 0xff) as u8;
        Self { red, green, blue }
    }
}

/// First error after sending multiple outputs
#[derive(Debug)]
pub struct SendOutputsError {
    /// The number of outputs that have been sent successfully before an error occurred.
    ///
    /// This could only be set if the outputs are sent subsequently and in order.
    /// If `None` then it is unknown which outputs have arrived at their destination
    /// despite the error.
    pub sent_ok: Option<usize>,

    /// The actual error that occurred.
    pub err: OutputError,
}

pub trait ControlOutputGateway {
    /// Send a single output
    fn send_output(&mut self, output: &Control) -> OutputResult<()>;

    /// Send multiple outputs
    ///
    /// The default implementation sends single outputs subsequently in order.
    fn send_outputs(&mut self, outputs: &[Control]) -> Result<(), SendOutputsError> {
        let mut sent_ok = 0;
        for output in outputs {
            match self.send_output(output) {
                Ok(()) => {
                    sent_ok += 1;
                }
                Err(err) => {
                    return Err(SendOutputsError {
                        sent_ok: Some(sent_ok),
                        err,
                    });
                }
            }
        }
        debug_assert_eq!(sent_ok, outputs.len());
        Ok(())
    }
}

impl<T> ControlOutputGateway for T
where
    T: DerefMut + ?Sized,
    <T as Deref>::Target: ControlOutputGateway,
{
    fn send_output(&mut self, output: &Control) -> OutputResult<()> {
        self.deref_mut().send_output(output)
    }

    fn send_outputs(&mut self, outputs: &[Control]) -> Result<(), SendOutputsError> {
        self.deref_mut().send_outputs(outputs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    Off,
    BlinkFast,
    BlinkSlow,
    On,
}

impl LedState {
    #[must_use]
    pub const fn is_blinking(self) -> bool {
        match self {
            Self::BlinkFast | Self::BlinkSlow => true,
            Self::Off | Self::On => false,
        }
    }

    /// Initial LED output
    ///
    /// Blinking should always start by turning on the LED before
    /// the periodic switching takes over.
    #[must_use]
    pub const fn initial_output(self) -> LedOutput {
        self.output(BlinkingLedOutput::ON)
    }

    /// LED output depending on the current blinking state
    #[must_use]
    pub const fn output(self, blinking_led_output: BlinkingLedOutput) -> LedOutput {
        match self {
            Self::Off => LedOutput::Off,
            Self::BlinkFast => blinking_led_output.fast(),
            Self::BlinkSlow => blinking_led_output.slow(),
            Self::On => LedOutput::On,
        }
    }
}

pub const DEFAULT_BLINKING_LED_PERIOD: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkingLedOutput(u8);

impl BlinkingLedOutput {
    pub const ON: Self = Self(0b11);

    #[must_use]
    pub const fn fast(self) -> LedOutput {
        match self.0 & 0b01 {
            0b00 => LedOutput::Off,
            0b01 => LedOutput::On,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub const fn slow(self) -> LedOutput {
        match self.0 & 0b10 {
            0b00 => LedOutput::Off,
            0b10 => LedOutput::On,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
pub struct BlinkingLedTicker(usize);

impl BlinkingLedTicker {
    const fn output_from_value(value: usize) -> BlinkingLedOutput {
        #[allow(clippy::cast_possible_truncation)]
        BlinkingLedOutput(!value as u8 & 0b11)
    }

    #[must_use]
    pub fn tick(&mut self) -> BlinkingLedOutput {
        let value = self.0;
        self.0 = self.0.wrapping_add(1);
        Self::output_from_value(value)
    }

    #[must_use]
    pub const fn output(&self) -> BlinkingLedOutput {
        Self::output_from_value(self.0)
    }

    pub fn map_into_output_stream(
        self,
        periodic: impl Stream<Item = ()> + 'static,
    ) -> impl Stream<Item = BlinkingLedOutput> {
        stream::unfold(
            (self, Box::pin(periodic)),
            |(mut ticker, mut periodic)| async move {
                periodic.next().await.map(|()| {
                    let output = ticker.tick();
                    (output, (ticker, periodic))
                })
            },
        )
    }
}

/// Virtual LED
///
/// For displaying virtual LEDs or illuminated buttons in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualLed {
    pub state: LedState,
    pub output: LedOutput,
}

impl VirtualLed {
    pub const OFF: Self = Self::initial_state(LedState::Off);

    /// Create a new virtual LED
    #[must_use]
    pub const fn initial_state(state: LedState) -> Self {
        let output = state.initial_output();
        Self { state, output }
    }

    /// Update the state
    ///
    /// The output is initialized accordingly to reflect the new state.
    ///
    /// Returns `true` if the state has changed.
    pub fn update_state(&mut self, state: LedState) -> bool {
        if self.state == state {
            // Unchanged
            return false;
        }
        *self = Self::initial_state(state);
        true
    }

    /// Update the blinking output
    ///
    /// The output is updated accordingly while the state remains unchanged.
    pub fn update_blinking_output(&mut self, blinking_led_output: BlinkingLedOutput) {
        let Self { state, output } = self;
        *output = state.output(blinking_led_output);
    }
}

impl Default for VirtualLed {
    fn default() -> Self {
        Self::OFF
    }
}

#[cfg(test)]
mod tests {
    use crate::{BlinkingLedOutput, BlinkingLedTicker, LedOutput};

    #[test]
    fn blinking_led_output_on() {
        assert_eq!(LedOutput::On, BlinkingLedOutput::ON.fast());
        assert_eq!(LedOutput::On, BlinkingLedOutput::ON.slow());
    }

    #[test]
    fn blinking_led_ticker_initial_output_is_on() {
        assert_eq!(BlinkingLedOutput::ON, BlinkingLedTicker::default().output());
    }
}
