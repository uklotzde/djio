// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

pub mod input;
pub use self::input::{Input, InputEvent, InputGateway};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Pioneer"),
        model_name: Cow::Borrowed("DDJ-400"),
    },
    port_name_prefix: "DDJ-400",
};

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    Left,
    Right,
}
