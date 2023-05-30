// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::midi::DeviceDescriptor;

pub mod input;
pub use self::input::{Event as InputEvent, Gateway as InputGateway, Input};

pub mod output;
pub use self::output::Gateway as OutputGateway;

pub const DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    vendor_name: "Korg",
    model_name: "KAOSS DJ",
    port_name_prefix: "KAOSS DJ",
};

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    /// Left deck
    A,
    /// Right deck
    B,
}
