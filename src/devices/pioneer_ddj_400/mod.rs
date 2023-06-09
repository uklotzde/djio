// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

pub mod input;
pub use self::input::{DeckSensor, MainSensor, MidiInputEventDecoder};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Pioneer"),
        product_name: Cow::Borrowed("DDJ-400"),
    },
    port_name_prefix: "DDJ-400",
};

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    /// Left
    One,

    /// Right
    Two,
}

const MIDI_CHANNEL_MAIN: u8 = 0x06;
const MIDI_CHANNEL_DECK_ONE: u8 = 0x00;
const MIDI_CHANNEL_DECK_TWO: u8 = 0x01;

const MIDI_COMMAND_NOTE_ON: u8 = 0x90;
const MIDI_COMMAND_CC: u8 = 0xb0;

//const MIDI_STATUS_BUTTON_MAIN: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_BUTTON_DECK_ONE: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_ONE;
const MIDI_STATUS_BUTTON_DECK_TWO: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_TWO;

const MIDI_STATUS_CC_MAIN: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_CC_DECK_ONE: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_ONE;
const MIDI_STATUS_CC_DECK_TWO: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_TWO;

const MIDI_DECK_PLAYPAUSE_BUTTON: u8 = 0x0b;
const MIDI_DECK_CUE_BUTTON: u8 = 0x0c;

const CONTROL_INDEX_DECK_ONE: u32 = 0x0100;
const CONTROL_INDEX_DECK_TWO: u32 = 0x0200;
const CONTROL_INDEX_DECK_BIT_MASK: u32 = CONTROL_INDEX_DECK_ONE | CONTROL_INDEX_DECK_TWO;
const CONTROL_INDEX_ENUM_BIT_MASK: u32 = (1 << CONTROL_INDEX_DECK_BIT_MASK.trailing_zeros()) - 1;
