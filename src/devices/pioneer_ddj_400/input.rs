// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr};

use super::{
    Deck, CONTROL_INDEX_DECK_BIT_MASK, CONTROL_INDEX_DECK_ONE, CONTROL_INDEX_DECK_TWO,
    CONTROL_INDEX_ENUM_BIT_MASK, MIDI_CHANNEL_DECK_ONE, MIDI_CHANNEL_DECK_TWO,
    MIDI_DECK_CUE_BUTTON, MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DEVICE_DESCRIPTOR,
    MIDI_STATUS_BUTTON_DECK_ONE, MIDI_STATUS_BUTTON_DECK_TWO, MIDI_STATUS_CC,
    MIDI_STATUS_CC_DECK_ONE, MIDI_STATUS_CC_DECK_TWO,
};
use crate::{
    u7_be_to_u14, ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent, ControlRegister,
    Input, MidiInputConnector, MidiInputDecodeError, SliderInput, TimeStamp,
};

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum MainSensor {
    Crossfader,
}

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DeckSensor {
    CueButton,
    PlayPauseButton,
    PitchFaderCenterSlider,
    JogWheelSliderEncoder,
    LevelFader,
}

#[derive(Debug, Clone, Copy)]
pub enum Sensor {
    Main(MainSensor),
    Deck(Deck, DeckSensor),
}

impl From<MainSensor> for Sensor {
    fn from(from: MainSensor) -> Self {
        Self::Main(from)
    }
}

impl Sensor {
    #[must_use]
    pub const fn deck(self) -> Option<Deck> {
        match self {
            Self::Main(_) => None,
            Self::Deck(deck, _) => Some(deck),
        }
    }

    #[must_use]
    pub const fn to_control_index(self) -> ControlIndex {
        match self {
            Self::Main(sensor) => ControlIndex::new(sensor as u32),
            Self::Deck(deck, sensor) => {
                let deck_bit = match deck {
                    Deck::One => CONTROL_INDEX_DECK_ONE,
                    Deck::Two => CONTROL_INDEX_DECK_TWO,
                };
                ControlIndex::new(deck_bit | sensor as u32)
            }
        }
    }
}

impl From<Sensor> for ControlIndex {
    fn from(from: Sensor) -> Self {
        from.to_control_index()
    }
}

#[derive(Debug)]
pub struct InvalidInputControlIndex;

impl TryFrom<ControlIndex> for Sensor {
    type Error = InvalidInputControlIndex;

    fn try_from(from: ControlIndex) -> Result<Self, Self::Error> {
        let value = from.value();
        debug_assert!(CONTROL_INDEX_ENUM_BIT_MASK <= u8::MAX.into());
        let enum_index = (value & CONTROL_INDEX_ENUM_BIT_MASK) as u8;
        let deck = match value & CONTROL_INDEX_DECK_BIT_MASK {
            CONTROL_INDEX_DECK_ONE => Deck::One,
            CONTROL_INDEX_DECK_TWO => Deck::Two,
            CONTROL_INDEX_DECK_BIT_MASK => return Err(InvalidInputControlIndex),
            _ => {
                return MainSensor::from_repr(enum_index)
                    .map(Sensor::Main)
                    .ok_or(InvalidInputControlIndex);
            }
        };
        DeckSensor::from_repr(enum_index)
            .map(|sensor| Sensor::Deck(deck, sensor))
            .ok_or(InvalidInputControlIndex)
    }
}

fn u7_to_button(input: u8) -> ButtonInput {
    match input {
        0x00 => ButtonInput::Released,
        0x7f => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

fn midi_status_to_deck(status: u8) -> Deck {
    match status & 0xf {
        MIDI_CHANNEL_DECK_ONE => Deck::One,
        MIDI_CHANNEL_DECK_TWO => Deck::Two,
        _ => unreachable!("Unexpected MIDI status {status}"),
    }
}

#[derive(Debug, Clone, Default)]
pub struct MidiInputEventDecoder {
    last_hi: u8,
}

impl crate::MidiInputEventDecoder for MidiInputEventDecoder {
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
        let (sensor, input): (Sensor, Input) = match *input {
            [MIDI_STATUS_CC, data1, data2] => match data1 {
                0x1f => {
                    self.last_hi = data2;
                    return Ok(None);
                }
                0x3f => (
                    MainSensor::Crossfader.into(),
                    CenterSliderInput::from_u14(u7_be_to_u14(self.last_hi, data2)).into(),
                ),
                _ => {
                    return Err(MidiInputDecodeError);
                }
            },
            [status @ (MIDI_STATUS_BUTTON_DECK_ONE | MIDI_STATUS_BUTTON_DECK_TWO), data1, data2] => {
                let deck = midi_status_to_deck(status);
                let input = u7_to_button(data2);
                let sensor = match data1 {
                    MIDI_DECK_PLAYPAUSE_BUTTON => DeckSensor::PlayPauseButton,
                    MIDI_DECK_CUE_BUTTON => DeckSensor::CueButton,
                    _ => {
                        return Err(MidiInputDecodeError);
                    }
                };
                (Sensor::Deck(deck, sensor), input.into())
            }
            [status @ (MIDI_STATUS_CC_DECK_ONE | MIDI_STATUS_CC_DECK_TWO), data1, data2] => {
                let deck = midi_status_to_deck(status);
                match data1 {
                    0x00 | 0x13 => {
                        self.last_hi = data2;
                        return Ok(None);
                    }
                    0x20 => (
                        Sensor::Deck(deck, DeckSensor::PitchFaderCenterSlider),
                        CenterSliderInput::from_u14(u7_be_to_u14(self.last_hi, data2))
                            .inverse()
                            .into(),
                    ),
                    0x33 => (
                        Sensor::Deck(deck, DeckSensor::LevelFader),
                        SliderInput::from_u14(u7_be_to_u14(self.last_hi, data2)).into(),
                    ),
                    _ => {
                        return Err(MidiInputDecodeError);
                    }
                }
            }
            _ => {
                return Err(MidiInputDecodeError);
            }
        };
        let input = ControlRegister {
            index: sensor.into(),
            value: input.into(),
        };
        let event = ControlInputEvent { ts, input };
        Ok(Some(event))
    }
}

impl MidiInputConnector for MidiInputEventDecoder {
    fn connect_midi_input_port(
        &mut self,
        device: &crate::MidiDeviceDescriptor,
        _input_port: &crate::MidiPortDescriptor,
    ) {
        assert_eq!(device, MIDI_DEVICE_DESCRIPTOR);
    }
}
