// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use smol_str::SmolStr;
use strum::{EnumCount, EnumIter, FromRepr};

use crate::{
    AudioInterfaceDescriptor, ControllerDescriptor, DeviceDescriptor, MidiDeviceDescriptor,
};

pub mod input;
pub use self::input::{DeckSensor, EffectSensor, MainSensor, MidiInputEventDecoder, Sensor};

pub mod output;
pub use self::output::{
    DeckLed, InvalidOutputControlIndex, Led, MainLed, OutputGateway, led_output_into_midi_message,
};

pub const AUDIO_INTERFACE_DESCRIPTOR: AudioInterfaceDescriptor = AudioInterfaceDescriptor {
    num_input_channels: 0,
    num_output_channels: 4,
};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: SmolStr::new_static("Pioneer"),
        product_name: SmolStr::new_static("DDJ-400"),
        audio_interface: Some(AUDIO_INTERFACE_DESCRIPTOR),
    },
    port_name_prefix: "DDJ-400",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;

pub const CONTROLLER_DESCRIPTOR: &ControllerDescriptor = &ControllerDescriptor {
    num_decks: 2,
    num_virtual_decks: 2,
    num_mixer_channels: 2,
    num_pads_per_deck: 8,
    num_effect_units: 1,
};

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum Deck {
    /// Left
    One,
    /// Right
    Two,
}

impl Deck {
    const fn midi_channel(self) -> u8 {
        match self {
            Deck::One => MIDI_CHANNEL_DECK_ONE,
            Deck::Two => MIDI_CHANNEL_DECK_TWO,
        }
    }

    const fn control_index_bit_mask(self) -> u32 {
        match self {
            Deck::One => CONTROL_INDEX_DECK_ONE,
            Deck::Two => CONTROL_INDEX_DECK_TWO,
        }
    }
}

const MIDI_CHANNEL_MAIN: u8 = 0x06;
const MIDI_CHANNEL_EFFECT: u8 = 0x04;
const MIDI_CHANNEL_DECK_ONE: u8 = 0x00;
const MIDI_CHANNEL_DECK_TWO: u8 = 0x01;
const MIDI_CHANNEL_PERFORMANCE_DECK_ONE: u8 = 0x07;
const MIDI_CHANNEL_PERFORMANCE_DECK_TWO: u8 = 0x09;

const MIDI_COMMAND_NOTE_ON: u8 = 0x90;
const MIDI_COMMAND_CC: u8 = 0xb0;

const MIDI_STATUS_BUTTON_MAIN: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_BUTTON_EFFECT: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_EFFECT;
const MIDI_STATUS_BUTTON_DECK_ONE: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_ONE;
const MIDI_STATUS_BUTTON_DECK_TWO: u8 = MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_DECK_TWO;
const MIDI_STATUS_BUTTON_PERFORMANCE_DECK_ONE: u8 =
    MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_PERFORMANCE_DECK_ONE;
const MIDI_STATUS_BUTTON_PERFORMANCE_DECK_TWO: u8 =
    MIDI_COMMAND_NOTE_ON | MIDI_CHANNEL_PERFORMANCE_DECK_TWO;

const MIDI_STATUS_CC_MAIN: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_MAIN;
const MIDI_STATUS_CC_EFFECT: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_EFFECT;
const MIDI_STATUS_CC_DECK_ONE: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_ONE;
const MIDI_STATUS_CC_DECK_TWO: u8 = MIDI_COMMAND_CC | MIDI_CHANNEL_DECK_TWO;

const MIDI_DECK_PLAYPAUSE_BUTTON: u8 = 0x0B;

const MIDI_MASTER_CUE: u8 = 0x63;
const MIDI_BEAT_FX: u8 = 0x47;

const CONTROL_INDEX_DECK_ONE: u32 = 0x0100;
const CONTROL_INDEX_DECK_TWO: u32 = 0x0200;
const CONTROL_INDEX_PERFORMANCE_DECK_ONE: u32 = 0x0300;
const CONTROL_INDEX_PERFORMANCE_DECK_TWO: u32 = 0x0400;

const CONTROL_INDEX_DECK_BIT_MASK: u32 = CONTROL_INDEX_DECK_ONE | CONTROL_INDEX_DECK_TWO;
const CONTROL_INDEX_ENUM_BIT_MASK: u32 = (1 << CONTROL_INDEX_DECK_BIT_MASK.trailing_zeros()) - 1;
