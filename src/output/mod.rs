// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Sending control data to actuators like LEDs and motorized platters
//! or for configuring devices.

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use futures::StreamExt as _;
use strum::FromRepr;
use thiserror::Error;

use crate::{Control, ControlValue};

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
    brightness: u8,
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
    red: u8,
    green: u8,
    blue: u8,
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
        self.output(BlinkingLedsOutput::ON)
    }

    /// LED output depending on the current blinking state
    #[must_use]
    pub const fn output(self, blinking_leds_output: BlinkingLedsOutput) -> LedOutput {
        match self {
            Self::Off => LedOutput::Off,
            Self::BlinkFast => blinking_leds_output.fast,
            Self::BlinkSlow => blinking_leds_output.slow,
            Self::On => LedOutput::On,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkingLedsOutput {
    pub fast: LedOutput,
    pub slow: LedOutput,
}

impl BlinkingLedsOutput {
    pub const ON: Self = Self {
        fast: LedOutput::On,
        slow: LedOutput::On,
    };
}

#[derive(Debug, Default)]
pub struct BlinkingLedsTicker(usize);

impl BlinkingLedsTicker {
    fn output_from_value(value: usize) -> BlinkingLedsOutput {
        match value & 0b11 {
            0b00 => BlinkingLedsOutput {
                fast: LedOutput::On,
                slow: LedOutput::On,
            },
            0b01 => BlinkingLedsOutput {
                fast: LedOutput::Off,
                slow: LedOutput::On,
            },
            0b10 => BlinkingLedsOutput {
                fast: LedOutput::On,
                slow: LedOutput::Off,
            },
            0b11 => BlinkingLedsOutput {
                fast: LedOutput::Off,
                slow: LedOutput::Off,
            },
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn tick(&mut self) -> BlinkingLedsOutput {
        let value = self.0;
        self.0 = self.0.wrapping_add(1);
        Self::output_from_value(value)
    }

    #[must_use]
    pub fn output(&self) -> BlinkingLedsOutput {
        let value = self.0;
        Self::output_from_value(value)
    }

    pub fn map_into_output_stream(
        self,
        periodic: impl futures::Stream<Item = ()> + 'static,
    ) -> impl futures::Stream<Item = BlinkingLedsOutput> {
        futures::stream::unfold(
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
