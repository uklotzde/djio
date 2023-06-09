// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use strum::{EnumCount, EnumIter, FromRepr};

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

mod input;
pub use self::input::{
    try_decode_midi_input, try_decode_midi_input_event, DeckSensor, InvalidInputControlIndex,
    MainSensor, MidiInputEventDecoder, Sensor,
};

mod output;
pub use self::output::{
    led_output_into_midi_message, DeckLed, InvalidOutputControlIndex, Led, MainLed, OutputGateway,
};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("KORG"),
        product_name: Cow::Borrowed("KAOSS DJ"),
    },
    port_name_prefix: "KAOSS DJ",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum Deck {
    /// Left deck
    A,
    /// Right deck
    B,
}

const MIDI_CHANNEL_MAIN: u8 = 0x06;
const MIDI_CHANNEL_DECK_A: u8 = 0x07;
const MIDI_CHANNEL_DECK_B: u8 = 0x08;

const MIDI_COMMAND_NOTE_ON: u8 = 0x90;
const MIDI_COMMAND_CC: u8 = 0xb0;

const MIDI_STATUS_BUTTON_MAIN: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_BUTTON_DECK_A: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_A;
const MIDI_STATUS_BUTTON_DECK_B: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_B;

const MIDI_STATUS_CC_MAIN: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_CC_DECK_A: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_A;
const MIDI_STATUS_CC_DECK_B: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_B;

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

const CONTROL_INDEX_DECK_A: u32 = 0x0100;
const CONTROL_INDEX_DECK_B: u32 = 0x0200;
const CONTROL_INDEX_DECK_BIT_MASK: u32 = CONTROL_INDEX_DECK_A | CONTROL_INDEX_DECK_B;
const CONTROL_INDEX_ENUM_BIT_MASK: u32 = (1 << CONTROL_INDEX_DECK_BIT_MASK.trailing_zeros()) - 1;
