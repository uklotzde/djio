// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

mod input;
pub use self::input::{
    Button, CenterSlider, DeckCenterSlider, Input, InputEvent, InputGateway, Sensor,
};

mod output;
pub use self::output::{Actuator, DeckLed, Led, OutputGateway};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Korg"),
        product_name: Cow::Borrowed("KAOSS DJ"),
    },
    port_name_prefix: "KAOSS DJ",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    /// Left deck
    A,
    /// Right deck
    B,
}
