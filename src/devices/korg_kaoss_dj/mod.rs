// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use strum::{EnumCount, EnumIter};

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

mod input;
pub use self::input::{
    try_decode_midi_input, try_decode_midi_input_event, DeckSensor, MainSensor,
    MidiInputEventDecoder, Sensor,
};

mod output;
pub use self::output::{
    Actuator, ButtonLed, DeckButtonLed, DeckKnobLed, DeckLed, KnobLed, Led, OutputGateway,
};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Korg"),
        product_name: Cow::Borrowed("KAOSS DJ"),
    },
    port_name_prefix: "KAOSS DJ",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;

#[derive(Debug, Clone, Copy, EnumIter, EnumCount)]
pub enum Deck {
    /// Left deck
    A,
    /// Right deck
    B,
}

const MIDI_CHANNEL_DECK_A: u8 = 0x7;
const MIDI_CHANNEL_DECK_B: u8 = 0x8;

/// Button (Note On)
const MIDI_STATUS_BUTTON: u8 = 0x96;

/// Button (Note On) Deck A
const MIDI_STATUS_BUTTON_DECK_A: u8 = 0x97;

/// Button (Note On) Deck B
const MIDI_STATUS_BUTTON_DECK_B: u8 = 0x98;

/// Control Change (Knob/Fader/Slider/Encoder)
const MIDI_STATUS_CC: u8 = 0xb6;

/// Control Change (Knob/Fader/Slider/Encoder) Deck A
const MIDI_STATUS_CC_DECK_A: u8 = 0xb7;

/// Control Change (Knob/Fader/Slider/Encoder) Deck B
const MIDI_STATUS_CC_DECK_B: u8 = 0xb8;

const MIDI_TAP_BUTTON: u8 = 0x0b;

const MIDI_MONITOR_LEVEL_KNOB: u8 = 0x14;
const MIDI_MONITOR_MIX_KNOB: u8 = 0x15;
const MIDI_MASTER_LEVEL_KNOB: u8 = 0x16;

const MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON: u8 = 0x0f;
const MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON: u8 = 0x10;
const MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON: u8 = 0x11;
const MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON: u8 = 0x12;
const MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON: u8 = 0x13;
const MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON: u8 = 0x14;
const MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON: u8 = 0x15;
const MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON: u8 = 0x16;
const MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON: u8 = 0x17;
const MIDI_DECK_MONITOR_BUTTON: u8 = 0x19;
const MIDI_DECK_PLAYPAUSE_BUTTON: u8 = 0x1b;
const MIDI_DECK_SYNC_BUTTON: u8 = 0x1d;
const MIDI_DECK_CUE_BUTTON: u8 = 0x1e;

const MIDI_DECK_GAIN_KNOB: u8 = 0x1a;
const MIDI_DECK_EQ_HI_KNOB: u8 = 0x1b;
const MIDI_DECK_EQ_MID_KNOB: u8 = 0x1c;
const MIDI_DECK_EQ_LO_KNOB: u8 = 0x1d;
