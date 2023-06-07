// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use strum::{EnumCount, EnumIter};

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

mod input;
pub use self::input::{Input, InputEvent, InputGateway};

mod output;
pub use self::output::OutputGateway;

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Denon DJ"),
        product_name: Cow::Borrowed("MC6000MK2"),
    },
    port_name_prefix: "MC6000MK2",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;

#[derive(Debug, Clone, Copy, EnumIter, EnumCount)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, EnumIter, EnumCount)]
pub enum Deck {
    /// Primary left deck
    One,
    /// Primary right deck
    Two,
    /// Secondary left deck
    Three,
    /// Secondary right deck
    Four,
}

impl Deck {
    #[must_use]
    pub const fn side(self) -> Side {
        match self {
            Self::One | Self::Three => Side::Left,
            Self::Two | Self::Four => Side::Right,
        }
    }
}

const MIDI_CMD_NOTE_OFF: u8 = 0x80;
const MIDI_CMD_NOTE_ON: u8 = 0x90;
const MIDI_CMD_CC: u8 = 0xb0;

const MIDI_DECK_CUE_BUTTON: u8 = 0x42;
const MIDI_DECK_PLAYPAUSE_BUTTON: u8 = 0x43;
const MIDI_DECK_SYNC_BUTTON: u8 = 0x6b;
