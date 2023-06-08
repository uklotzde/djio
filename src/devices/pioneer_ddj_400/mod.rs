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

const MIDI_CHANNEL_DECK_ONE: u8 = 0x0;
const MIDI_CHANNEL_DECK_TWO: u8 = 0x1;

/// Button (Note On)
//const MIDI_STATUS_BUTTON: u8 = 0x96;

/// Button (Note On) Deck 1
const MIDI_STATUS_BUTTON_DECK_ONE: u8 = 0x90;

/// Button (Note On) Deck 2
const MIDI_STATUS_BUTTON_DECK_TWO: u8 = 0x91;

/// Control Change (Knob/Fader/Slider/Encoder)
const MIDI_STATUS_CC: u8 = 0xb6;

/// Control Change (Knob/Fader/Slider/Encoder) Deck 1
const MIDI_STATUS_CC_DECK_ONE: u8 = 0xb0;

/// Control Change (Knob/Fader/Slider/Encoder) Deck 2
const MIDI_STATUS_CC_DECK_TWO: u8 = 0xb1;

const MIDI_DECK_PLAYPAUSE_BUTTON: u8 = 0x0b;
const MIDI_DECK_CUE_BUTTON: u8 = 0x0c;

const CONTROL_INDEX_DECK_ONE: u32 = 0x0100;
const CONTROL_INDEX_DECK_TWO: u32 = 0x0200;
const CONTROL_INDEX_DECK_BIT_MASK: u32 = CONTROL_INDEX_DECK_ONE | CONTROL_INDEX_DECK_TWO;
const CONTROL_INDEX_ENUM_BIT_MASK: u32 = (1 << CONTROL_INDEX_DECK_BIT_MASK.trailing_zeros()) - 1;
