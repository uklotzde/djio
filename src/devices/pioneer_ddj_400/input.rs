// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! # Pioneer DDJ-400
//!
//! Most of the terms in this module have been taken
//! from the manual (`DDJ-400_manual_Manual_EN.pdf`).
//! The manual and detailed information can be found here:
//! <https://support.pioneerdj.com/hc/en-us/sections/4416577146009-ddj-400>
//! and here:
//! <https://www.pioneerdj.com/-/media/pioneerdj/software-info/controller/ddj-400/ddj-400_midi_message_list_e1.pdf>.
use derive_more::From;
use strum::{EnumCount, EnumIter, FromRepr};

use super::{
    Deck, CONTROL_INDEX_DECK_BIT_MASK, CONTROL_INDEX_DECK_ONE, CONTROL_INDEX_DECK_TWO,
    CONTROL_INDEX_ENUM_BIT_MASK, CONTROL_INDEX_PERFORMANCE_DECK_ONE,
    CONTROL_INDEX_PERFORMANCE_DECK_TWO, MIDI_CHANNEL_DECK_ONE, MIDI_CHANNEL_DECK_TWO,
    MIDI_CHANNEL_PERFORMANCE_DECK_ONE, MIDI_CHANNEL_PERFORMANCE_DECK_TWO, MIDI_DEVICE_DESCRIPTOR,
    MIDI_STATUS_BUTTON_DECK_ONE, MIDI_STATUS_BUTTON_DECK_TWO, MIDI_STATUS_BUTTON_EFFECT,
    MIDI_STATUS_BUTTON_MAIN, MIDI_STATUS_BUTTON_PERFORMANCE_DECK_ONE,
    MIDI_STATUS_BUTTON_PERFORMANCE_DECK_TWO, MIDI_STATUS_CC_DECK_ONE, MIDI_STATUS_CC_DECK_TWO,
    MIDI_STATUS_CC_EFFECT, MIDI_STATUS_CC_MAIN,
};
use crate::{
    u7_be_to_u14, ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent, ControlRegister,
    ControlValue, MidiInputConnector, MidiInputDecodeError, SelectorInput, SliderInput,
    StepEncoderInput, TimeStamp,
};

#[derive(Debug, Clone, Copy, From)]
pub enum Sensor {
    Main(MainSensor),
    Deck(Deck, DeckSensor),
    Effect(EffectSensor),
    Performance(Deck, PerformancePadSensor),
}

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum MainSensor {
    // -- Browser section -- //
    LoadLeftButton,
    LoadRightButton,
    RotarySelectorStepEncoder,
    RotarySelectorButton,
    // -- Mixer section -- //
    MasterLevelSlider,
    HeadphoneCueButton,
    HeadphonesMixingCenterSlider,
    HeadphonesLevelSlider,
    CrossfaderCenterSlider,
    FilterLeftCenterSlider,
    FilterRightCenterSlider,
}

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DeckSensor {
    // -- Deck section -- //
    BeatSyncButton,
    CueLoopCallRightButton,
    CueLoopCallLeftButton,
    DeleteButton,
    MemoryButton,
    ReloopExitButton,
    OutButton,
    InAdjustButton,
    OutAdjustButton,
    ActiveLoopButton,
    In4BeatButton,
    JogWheelTouch,
    JogWheelTopEncoder,
    JogWheelOuterEncoder,
    HotCueModeButton,
    BeatLoopModeButton,
    BeatJumpModeButton,
    SamplerModeButton,
    TempoCenterSlider,
    PlayPauseButton,
    CueButton,
    CueToStartButton,
    TempoRangeButton,
    ShiftButton,
    // -- Mixer section -- //
    TrimSlider,
    EqHighCenterSlider,
    EqMidCenterSlider,
    EqLowCenterSlider,
    HeadphoneCueButton,
    LevelFader,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum EffectSensor {
    BeatLeftButton,
    BeatRightButton,
    BeatFxSelectButton,
    BeatFxChannelSelectSwitch,
    BeatFxLevelDepthKnob,
    BeatFxOnOffButton,
}

#[derive(Debug, Clone, Copy)]
pub enum PerformancePadSensor {
    HotCue(u8),
    BeatLoop(u8),
    BeatJump(u8),
    Sampler(u8),
    Keyboard(u8),
    PadFx1(u8),
    PadFx2(u8),
    KeyShift(u8),
}

impl PerformancePadSensor {
    const fn as_u8(self) -> u8 {
        match self {
            Self::HotCue(nr) => nr,
            Self::BeatJump(nr) => nr + 0x20,
            Self::Sampler(nr) => nr + 0x30,
            Self::BeatLoop(nr) => nr + 0x60,
            Self::Keyboard(nr) => nr + 0x40,
            Self::PadFx1(nr) => nr + 0x10,
            Self::PadFx2(nr) => nr + 0x50,
            Self::KeyShift(nr) => nr + 0x70,
        }
    }
    fn try_from_u8(pad_id: u8) -> Option<Self> {
        let sensor = match pad_id {
            0x00..=0x07 => Self::HotCue(pad_id),
            0x10..=0x17 => Self::PadFx1(pad_id - 0x10),
            0x20..=0x27 => Self::BeatJump(pad_id - 0x20),
            0x30..=0x37 => Self::Sampler(pad_id - 0x30),
            0x40..=0x47 => Self::Keyboard(pad_id - 0x40),
            0x50..=0x57 => Self::PadFx2(pad_id - 0x50),
            0x60..=0x67 => Self::BeatLoop(pad_id - 0x60),
            0x70..=0x77 => Self::KeyShift(pad_id - 0x70),
            _ => return None,
        };
        Some(sensor)
    }
}

impl Sensor {
    #[must_use]
    pub const fn deck(self) -> Option<Deck> {
        match self {
            Self::Deck(deck, _) => Some(deck),
            _ => None,
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
            Self::Effect(sensor) => ControlIndex::new(sensor as u32),
            Self::Performance(deck, sensor) => {
                let deck_bit = match deck {
                    Deck::One => CONTROL_INDEX_PERFORMANCE_DECK_ONE,
                    Deck::Two => CONTROL_INDEX_PERFORMANCE_DECK_TWO,
                };
                let pad_id = sensor.as_u8();
                ControlIndex::new(deck_bit | pad_id as u32)
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

fn midi_status_to_performance_deck(status: u8) -> Deck {
    match status & 0xf {
        MIDI_CHANNEL_PERFORMANCE_DECK_ONE => Deck::One,
        MIDI_CHANNEL_PERFORMANCE_DECK_TWO => Deck::Two,
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
        // TODO: make this more readable
        let (sensor, value) = if let Some(ev) = try_decode_button_event(self, input)? {
            ev
        } else if let Some(ev) = try_decode_cc_event(self, input)? {
            ev
        } else {
            return Err(MidiInputDecodeError);
        };
        log::debug!("{sensor:?} {input:?}");
        let input = ControlRegister {
            index: sensor.into(),
            value,
        };
        let event = ControlInputEvent { ts, input };
        Ok(Some(event))
    }
}

fn try_decode_button_event(
    decoder: &mut MidiInputEventDecoder,
    input: &[u8],
) -> Result<Option<(Sensor, ControlValue)>, MidiInputDecodeError> {
    let sensor = match *input {
        [MIDI_STATUS_BUTTON_MAIN, data1, _] => {
            let sensor = match data1 {
                0x40 => MainSensor::RotarySelectorStepEncoder,
                0x41 => MainSensor::RotarySelectorButton,
                0x46 => MainSensor::LoadLeftButton,
                0x47 => MainSensor::LoadRightButton,
                0x63 => MainSensor::HeadphoneCueButton,
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            sensor.into()
        }
        [MIDI_STATUS_BUTTON_EFFECT, data1, data2] => {
            #[allow(clippy::bool_to_int_with_if)]
            let sensor = match data1 {
                0x10 => {
                    decoder.last_hi = if data2 == 0x7f { 0 } else { 1 };
                    return Ok(None);
                }
                0x14 => {
                    decoder.last_hi = if data2 == 0x7f { 2 } else { 1 };
                    return Ok(None);
                }
                0x47 => EffectSensor::BeatFxOnOffButton,
                0x4a => EffectSensor::BeatLeftButton,
                0x4b => EffectSensor::BeatRightButton,
                0x63 => EffectSensor::BeatFxSelectButton,
                0x11 => EffectSensor::BeatFxChannelSelectSwitch,
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            Sensor::Effect(sensor)
        }
        [status @ (MIDI_STATUS_BUTTON_DECK_ONE | MIDI_STATUS_BUTTON_DECK_TWO), data1, _] => {
            let deck = midi_status_to_deck(status);
            let sensor = match data1 {
                0x0b => DeckSensor::PlayPauseButton,
                0x0c => DeckSensor::CueButton,
                0x10 => DeckSensor::In4BeatButton,
                0x11 => DeckSensor::OutButton,
                0x1b => DeckSensor::HotCueModeButton,
                0x20 => DeckSensor::BeatJumpModeButton,
                0x22 => DeckSensor::SamplerModeButton,
                0x36 => DeckSensor::JogWheelTouch,
                0x3d => DeckSensor::MemoryButton,
                0x3e => DeckSensor::DeleteButton,
                0x3f => DeckSensor::ShiftButton,
                0x48 => DeckSensor::CueToStartButton,
                0x4c => DeckSensor::InAdjustButton,
                0x4d => DeckSensor::ReloopExitButton,
                0x4e => DeckSensor::OutAdjustButton,
                0x50 => DeckSensor::ActiveLoopButton,
                0x51 => DeckSensor::CueLoopCallLeftButton,
                0x53 => DeckSensor::CueLoopCallRightButton,
                0x54 => DeckSensor::HeadphoneCueButton,
                0x58 => DeckSensor::BeatSyncButton,
                0x60 => DeckSensor::TempoRangeButton,
                0x6d => DeckSensor::BeatLoopModeButton,
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            Sensor::Deck(deck, sensor)
        }
        [status @ (MIDI_STATUS_BUTTON_PERFORMANCE_DECK_ONE
        | MIDI_STATUS_BUTTON_PERFORMANCE_DECK_TWO), data1, _] => {
            let deck = midi_status_to_performance_deck(status);
            let Some(sensor) = PerformancePadSensor::try_from_u8(data1) else {
                  return Err(MidiInputDecodeError);
                };
            Sensor::Performance(deck, sensor)
        }
        _ => return Ok(None),
    };

    let value = if input[1] == 0x11 {
        let choice = u32::from(decoder.last_hi);
        SelectorInput { choice }.into()
    } else {
        u7_to_button(input[2]).into()
    };
    Ok(Some((sensor, value)))
}

#[allow(clippy::too_many_lines)]
fn try_decode_cc_event(
    decoder: &mut MidiInputEventDecoder,
    input: &[u8],
) -> Result<Option<(Sensor, ControlValue)>, MidiInputDecodeError> {
    let (sensor, value) = match *input {
        [MIDI_STATUS_CC_MAIN, data1, data2] => match data1 {
            0x1f | 0x08 | 0x0d | 0x0c | 0x17 | 0x18 => {
                decoder.last_hi = data2;
                return Ok(None);
            }
            0x3f => (
                MainSensor::CrossfaderCenterSlider.into(),
                CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            0x28 => (
                MainSensor::MasterLevelSlider.into(),
                SliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            0x2d => (
                MainSensor::HeadphonesLevelSlider.into(),
                SliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            0x2c => (
                MainSensor::HeadphonesMixingCenterSlider.into(),
                CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            0x40 => (
                MainSensor::RotarySelectorStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            0x37 => (
                MainSensor::FilterLeftCenterSlider.into(),
                CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            0x38 => (
                MainSensor::FilterRightCenterSlider.into(),
                CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            _ => {
                return Err(MidiInputDecodeError);
            }
        },
        [MIDI_STATUS_CC_EFFECT, data1, data2] => match data1 {
            0x02 => {
                decoder.last_hi = data2;
                return Ok(None);
            }
            0x22 => (
                EffectSensor::BeatFxLevelDepthKnob.into(),
                CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
            ),
            _ => {
                return Err(MidiInputDecodeError);
            }
        },
        [status @ (MIDI_STATUS_CC_DECK_ONE | MIDI_STATUS_CC_DECK_TWO), data1, data2] => {
            let deck = midi_status_to_deck(status);
            let (sensor, value) = match data1 {
                0x00 | 0x13 | 0x07 | 0x0f | 0x0b | 0x04 => {
                    decoder.last_hi = data2;
                    return Ok(None);
                }
                0x20 => (
                    DeckSensor::TempoCenterSlider,
                    CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2))
                        .inverse()
                        .into(),
                ),
                0x33 => (
                    DeckSensor::LevelFader,
                    SliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
                ),
                0x21 => (
                    DeckSensor::JogWheelOuterEncoder,
                    StepEncoderInput::from_u7(data2).into(),
                ),
                0x22 => (
                    DeckSensor::JogWheelTopEncoder,
                    StepEncoderInput::from_u7(data2).into(),
                ),
                0x24 => (
                    DeckSensor::TrimSlider,
                    SliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
                ),
                0x27 => (
                    DeckSensor::EqHighCenterSlider,
                    CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
                ),
                0x2b => (
                    DeckSensor::EqMidCenterSlider,
                    CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
                ),
                0x2f => (
                    DeckSensor::EqLowCenterSlider,
                    CenterSliderInput::from_u14(u7_be_to_u14(decoder.last_hi, data2)).into(),
                ),
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            (Sensor::Deck(deck, sensor), value)
        }
        _ => {
            return Err(MidiInputDecodeError);
        }
    };
    Ok(Some((sensor, value)))
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
