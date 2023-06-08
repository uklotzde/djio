// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr};

use super::{Deck, Side};
use crate::{
    devices::denon_dj_mc6000mk2::{
        MIDI_CMD_CC, MIDI_CMD_NOTE_OFF, MIDI_CMD_NOTE_ON, MIDI_DECK_CUE_BUTTON,
        MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_SYNC_BUTTON,
    },
    u7_be_to_u14, ButtonInput, CenterSliderInput, Input, SliderEncoderInput, SliderInput,
    StepEncoderInput,
};

fn midi_status_to_deck_cmd(status: u8) -> (Deck, u8) {
    let cmd = status & 0xf;
    let deck = match status & 0x3 {
        0x0 => Deck::One,
        0x1 => Deck::Three,
        0x2 => Deck::Two,
        0x3 => Deck::Four,
        _ => unreachable!(),
    };
    (deck, cmd)
}

// Unused
// fn deck_cmd_to_midi_status(deck: Deck, cmd: u8) -> u8 {
//     debug_assert_eq!(0x0, cmd & 0x3);
//     let channel = match deck {
//         Deck::One => 0x0,
//         Deck::Three => 0x1,
//         Deck::Two => 0x2,
//         Deck::Four => 0x3,
//     };
//     cmd | channel
// }

fn midi_value_to_button(data2: u8) -> ButtonInput {
    match data2 {
        0x00 => ButtonInput::Released,
        0x40 => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, EnumIter, EnumCount)]
pub enum MainSensor {
    CrossfaderCenterSlider,
    BrowseKnobStepEncoder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, EnumIter, EnumCount)]
pub enum SideSensor {
    ShiftButton,
    PitchFaderCenterSlider,
    Efx1KnobSlider,
    Efx2KnobSlider,
    Efx3KnobSlider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, EnumIter, EnumCount)]
pub enum DeckSensor {
    CueButton,
    PlayPauseButton,
    SyncButton,
    LevelFaderSlider,
    JogWheelBendSliderEncoder,
    JogWheelScratchSliderEncoder,
    GainKnobCenterSlider,
    EqHiKnobCenterSlider,
    EqLoKnobCenterSlider,
    EqMidKnobCenterSlider,
}

#[derive(Debug)]
pub enum Sensor {
    Main(MainSensor),
    Side(Side, SideSensor),
    Deck(Deck, DeckSensor),
}

impl From<MainSensor> for Sensor {
    fn from(from: MainSensor) -> Self {
        Self::Main(from)
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn try_decode_midi_input(input: &[u8]) -> Option<(Sensor, Input)> {
    let [status, data1, data2] = *input else {
        return None;
    };
    let (deck, cmd) = midi_status_to_deck_cmd(status);
    let (sensor, input) = match cmd {
        MIDI_CMD_NOTE_OFF | MIDI_CMD_NOTE_ON => {
            let input = midi_value_to_button(data2);
            debug_assert_eq!(cmd == MIDI_CMD_NOTE_ON, input == ButtonInput::Pressed);
            debug_assert_eq!(cmd == MIDI_CMD_NOTE_OFF, input == ButtonInput::Released);
            let sensor = match data1 {
                0x60 | 0x61 => Sensor::Side(deck.side(), SideSensor::ShiftButton),
                MIDI_DECK_CUE_BUTTON => Sensor::Deck(deck, DeckSensor::CueButton),
                MIDI_DECK_PLAYPAUSE_BUTTON => Sensor::Deck(deck, DeckSensor::PlayPauseButton),
                MIDI_DECK_SYNC_BUTTON => Sensor::Deck(deck, DeckSensor::SyncButton),
                _ => {
                    return None;
                }
            };
            (sensor, input.into())
        }
        MIDI_CMD_CC => match data1 {
            0x01 | 0x07 | 0x0c | 0x11 => (
                Sensor::Deck(deck, DeckSensor::GainKnobCenterSlider),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x02 | 0x08 | 0x0d | 0x12 => (
                Sensor::Deck(deck, DeckSensor::EqHiKnobCenterSlider),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x03 | 0x09 | 0x0e | 0x13 => (
                Sensor::Deck(deck, DeckSensor::EqMidKnobCenterSlider),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x04 | 0x0a | 0x0f | 0x14 => (
                Sensor::Deck(deck, DeckSensor::EqLoKnobCenterSlider),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x05 | 0x0b | 0x10 | 0x15 => (
                Sensor::Deck(deck, DeckSensor::LevelFaderSlider),
                SliderInput::from_u7(data2).into(),
            ),
            0x16 | 0x17 => (
                MainSensor::CrossfaderCenterSlider.into(),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x51 => (
                Sensor::Deck(deck, DeckSensor::JogWheelBendSliderEncoder),
                SliderEncoderInput::from_u7(data2).into(),
            ),
            0x52 => (
                Sensor::Deck(deck, DeckSensor::JogWheelScratchSliderEncoder),
                SliderEncoderInput::from_u7(data2).into(),
            ),
            0x54 => (
                MainSensor::BrowseKnobStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            0x55 => (
                Sensor::Side(deck.side(), SideSensor::Efx1KnobSlider),
                SliderInput::from_u7(data2).into(),
            ),
            0x56 => (
                Sensor::Side(deck.side(), SideSensor::Efx2KnobSlider),
                SliderInput::from_u7(data2).into(),
            ),
            0x57 => (
                Sensor::Side(deck.side(), SideSensor::Efx3KnobSlider),
                SliderInput::from_u7(data2).into(),
            ),
            _ => {
                return None;
            }
        },
        0xe0 => (
            Sensor::Side(deck.side(), SideSensor::PitchFaderCenterSlider),
            CenterSliderInput::from_u14(u7_be_to_u14(data2, data1))
                .inverse()
                .into(),
        ),
        _ => {
            return None;
        }
    };
    Some((sensor, input))
}
