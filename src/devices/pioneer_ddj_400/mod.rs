// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::midi::DeviceDescriptor;

pub mod input;
pub use self::input::{Input, InputEvent, InputGateway};

pub const DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    vendor_name: "Pioneer",
    model_name: "DDJ-400",
    port_name_prefix: "DDJ-400",
};

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    Left,
    Right,
}
