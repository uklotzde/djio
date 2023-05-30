// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Send: {msg}")]
    Send { msg: Cow<'static, str> },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Simple LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Led {
    Off,
    On,
}

/// Dimmable LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimLed {
    Off,
    On,
    Dim,
}

/// RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbLed {
    red: u8,
    green: u8,
    blue: u8,
}
