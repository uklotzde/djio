// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Sending control data to actuators like LEDs and motorized platters
//! or for configuring devices.

use std::borrow::Cow;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("Send: {msg}")]
    Send { msg: Cow<'static, str> },
}

pub type OutputResult<T> = std::result::Result<T, OutputError>;

/// Simple LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedOutput {
    Off,
    On,
}

/// Dimmable LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimLedOutput {
    Off,
    On,
    Dim,
}

/// RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbLedOutput {
    red: u8,
    green: u8,
    blue: u8,
}
