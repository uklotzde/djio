// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Sending control data to actuators like LEDs and motorized platters
//! or for configuring devices.

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use strum::FromRepr;
use thiserror::Error;

use crate::{ControlRegister, ControlValue};

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
    fn send_output(&mut self, output: &ControlRegister) -> OutputResult<()>;

    /// Send multiple outputs
    ///
    /// The default implementation sends single outputs subsequently in order.
    fn send_outputs(&mut self, outputs: &[ControlRegister]) -> Result<(), SendOutputsError> {
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
    fn send_output(&mut self, output: &ControlRegister) -> OutputResult<()> {
        self.deref_mut().send_output(output)
    }

    fn send_outputs(&mut self, outputs: &[ControlRegister]) -> Result<(), SendOutputsError> {
        self.deref_mut().send_outputs(outputs)
    }
}
