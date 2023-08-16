// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr};

use super::{
    Deck, CONTROL_INDEX_DECK_A, CONTROL_INDEX_DECK_B, CONTROL_INDEX_DECK_BIT_MASK,
    CONTROL_INDEX_ENUM_BIT_MASK, MIDI_CHANNEL_DECK_A, MIDI_CHANNEL_DECK_B, MIDI_DECK_CUE_BUTTON,
    MIDI_DECK_EQ_HI_KNOB, MIDI_DECK_EQ_LO_KNOB, MIDI_DECK_EQ_MID_KNOB, MIDI_DECK_GAIN_KNOB,
    MIDI_DECK_MONITOR_BUTTON, MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_SYNC_BUTTON,
    MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON, MIDI_DEVICE_DESCRIPTOR, MIDI_MASTER_LEVEL_KNOB,
    MIDI_MONITOR_LEVEL_KNOB, MIDI_MONITOR_MIX_KNOB, MIDI_STATUS_BUTTON_DECK_A,
    MIDI_STATUS_BUTTON_DECK_B, MIDI_STATUS_BUTTON_MAIN, MIDI_STATUS_CC_DECK_A,
    MIDI_STATUS_CC_DECK_B, MIDI_STATUS_CC_MAIN, MIDI_TAP_BUTTON,
};
use crate::{
    ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent, ControlRegister, ControlValue,
    MidiInputConnector, MidiInputDecodeError, SliderEncoderInput, SliderInput, StepEncoderInput,
    TimeStamp,
};

fn u7_to_button(input: u8) -> ButtonInput {
    match input {
        0x00 => ButtonInput::Released,
        0x7f => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum MainSensor {
    BrowseKnobShiftButton, // Encoder knob acts like a button when shifted
    TapButton,
    TapHoldButton,
    TouchPadLowerLeftButton,
    TouchPadLowerRightButton,
    TouchPadModeButton, // 0: X/Y Sliders, 1: 4 Buttons
    TouchPadUpperLeftButton,
    TouchPadUpperRightButton,
    CrossfaderCenterSlider,
    AudiolessMonitorLevelSlider,
    AudiolessMonitorBalanceSlider,
    AudiolessMasterLevelSlider,
    TouchPadXSlider,
    TouchPadYSlider,
    BrowseKnobStepEncoder,
    ProgramKnobStepEncoder,
}

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DeckSensor {
    FxButton,
    LoadButton,
    MonitorButton,
    ShiftButton,
    CueButton,
    CueShiftButton,
    PlayPauseButton,
    PlayPauseShiftButton,
    SyncButton,
    SyncShiftButton,
    TouchStripLeftButton,   // Pitch bend down
    TouchStripCenterButton, // Vinyl mode switch
    TouchStripRightButton,  // Pitch bend up
    TouchStripLoopLeftButton,
    TouchStripLoopCenterButton,
    TouchStripLoopRightButton,
    TouchStripHotCueLeftButton,
    TouchStripHotCueCenterButton,
    TouchStripHotCueRightButton,
    TouchWheelScratchButton,
    VolumeFaderSlider,
    TouchStripSlider,
    GainKnobCenterSlider,
    EqHiKnobCenterSlider,
    EqLoKnobCenterSlider,
    EqMidKnobCenterSlider,
    PitchFaderCenterSlider,
    TouchWheelBendSliderEncoder,
    TouchWheelScratchSliderEncoder,
    TouchWheelSearchSliderEncoder,
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
                ControlIndex::new(deck.control_index_bit_mask() | sensor as u32)
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
            CONTROL_INDEX_DECK_A => Deck::A,
            CONTROL_INDEX_DECK_B => Deck::B,
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

fn midi_status_to_deck(status: u8) -> Deck {
    match status & 0xf {
        MIDI_CHANNEL_DECK_A => Deck::A,
        MIDI_CHANNEL_DECK_B => Deck::B,
        _ => unreachable!("Unexpected MIDI status {status}"),
    }
}

#[allow(clippy::too_many_lines)]
pub fn try_decode_midi_input(
    input: &[u8],
) -> Result<Option<(Sensor, ControlValue)>, MidiInputDecodeError> {
    let decoded = match *input {
        [MIDI_STATUS_BUTTON_MAIN, data1, data2] => {
            let input = u7_to_button(data2);
            let sensor = match data1 {
                0x07 => MainSensor::BrowseKnobShiftButton,
                MIDI_TAP_BUTTON => MainSensor::TapButton,
                0x21 => MainSensor::TapHoldButton,
                0x22 => MainSensor::TouchPadModeButton,
                0x4a => MainSensor::TouchPadUpperLeftButton,
                0x4b => MainSensor::TouchPadUpperRightButton,
                0x4c => MainSensor::TouchPadLowerLeftButton,
                0x4d => MainSensor::TouchPadLowerRightButton,
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            (sensor.into(), input.into())
        }
        [status @ (MIDI_STATUS_BUTTON_DECK_A | MIDI_STATUS_BUTTON_DECK_B), data1, data2] => {
            let input = u7_to_button(data2);
            let deck = midi_status_to_deck(status);
            let sensor = match data1 {
                0x0e => DeckSensor::LoadButton,
                MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON => DeckSensor::TouchStripLoopLeftButton,
                MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON => DeckSensor::TouchStripLoopCenterButton,
                MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON => DeckSensor::TouchStripLoopRightButton,
                MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON => DeckSensor::TouchStripHotCueLeftButton,
                MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON => {
                    DeckSensor::TouchStripHotCueCenterButton
                }
                MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON => DeckSensor::TouchStripHotCueRightButton,
                MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON => DeckSensor::TouchStripLeftButton,
                MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON => DeckSensor::TouchStripCenterButton,
                MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON => DeckSensor::TouchStripRightButton,
                0x18 => DeckSensor::FxButton,
                MIDI_DECK_MONITOR_BUTTON => DeckSensor::MonitorButton,
                0x1a => DeckSensor::ShiftButton,
                MIDI_DECK_PLAYPAUSE_BUTTON => DeckSensor::PlayPauseButton,
                MIDI_DECK_SYNC_BUTTON => DeckSensor::SyncButton,
                MIDI_DECK_CUE_BUTTON => DeckSensor::CueButton,
                0x1f => DeckSensor::TouchWheelScratchButton,
                0x2e => DeckSensor::PlayPauseShiftButton,
                0x2f => DeckSensor::SyncShiftButton,
                0x30 => DeckSensor::CueShiftButton,
                _ => {
                    return Err(MidiInputDecodeError);
                }
            };
            (Sensor::Deck(deck, sensor), input.into())
        }
        [MIDI_STATUS_CC_DECK_B, 0x0c | 0x0d, _data2] => {
            // Filter duplicate touch pad messages for deck B,
            // see the comments in next match expression.
            return Ok(None);
        }
        [status @ (MIDI_STATUS_CC_MAIN | MIDI_STATUS_CC_DECK_A), 0x0c, data2] => {
            // The X/Y coordinates of the touch pad are always sent twice for
            // unknown reasons. According to the documentation they should
            // be sent on the main channel instead of on both deck channels.
            debug_assert_ne!(MIDI_STATUS_CC_MAIN, status);
            debug_assert_eq!(MIDI_STATUS_CC_DECK_A, status);
            let input = SliderInput::from_u7(data2);
            (MainSensor::TouchPadXSlider.into(), input.into())
        }
        [status @ (MIDI_STATUS_CC_MAIN | MIDI_STATUS_CC_DECK_A), 0x0d, data2] => {
            // The X/Y coordinates of the touch pad are always sent twice for
            // unknown reasons. According to the documentation they should
            // be sent on the main channel instead of on both deck channels.
            debug_assert_ne!(MIDI_STATUS_CC_MAIN, status);
            debug_assert_eq!(MIDI_STATUS_CC_DECK_A, status);
            let input = SliderInput::from_u7(data2);
            (MainSensor::TouchPadYSlider.into(), input.into())
        }
        [MIDI_STATUS_CC_MAIN, data1, data2] => match data1 {
            MIDI_MONITOR_LEVEL_KNOB => (
                MainSensor::AudiolessMonitorLevelSlider.into(),
                SliderInput::from_u7(data2).into(),
            ),
            MIDI_MONITOR_MIX_KNOB => (
                MainSensor::AudiolessMonitorBalanceSlider.into(),
                SliderInput::from_u7(data2).into(),
            ),
            MIDI_MASTER_LEVEL_KNOB => (
                MainSensor::AudiolessMasterLevelSlider.into(),
                SliderInput::from_u7(data2).into(),
            ),
            0x17 => (
                MainSensor::CrossfaderCenterSlider.into(),
                CenterSliderInput::from_u7(data2).into(),
            ),
            0x1e => (
                MainSensor::BrowseKnobStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            0x1f => (
                MainSensor::ProgramKnobStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            _ => {
                return Err(MidiInputDecodeError);
            }
        },
        [status @ (MIDI_STATUS_CC_DECK_A | MIDI_STATUS_CC_DECK_B), data1, data2] => {
            let deck = midi_status_to_deck(status);
            let (sensor, value) = match data1 {
                0x0e => (
                    DeckSensor::TouchWheelBendSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                0x0f => (
                    DeckSensor::TouchWheelSearchSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                0x10 => (
                    DeckSensor::TouchWheelScratchSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                0x18 => (
                    DeckSensor::VolumeFaderSlider,
                    SliderInput::from_u7(data2).into(),
                ),
                0x19 => (
                    DeckSensor::PitchFaderCenterSlider,
                    CenterSliderInput::from_u7(data2).inverse().into(),
                ),
                MIDI_DECK_GAIN_KNOB => (
                    DeckSensor::GainKnobCenterSlider,
                    CenterSliderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_EQ_HI_KNOB => (
                    DeckSensor::EqHiKnobCenterSlider,
                    CenterSliderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_EQ_MID_KNOB => (
                    DeckSensor::EqMidKnobCenterSlider,
                    CenterSliderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_EQ_LO_KNOB => (
                    DeckSensor::EqLoKnobCenterSlider,
                    CenterSliderInput::from_u7(data2).into(),
                ),
                0x21 => (
                    DeckSensor::TouchStripSlider,
                    SliderInput::from_u7(data2).into(),
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
    Ok(Some(decoded))
}

pub fn try_decode_midi_input_event(
    ts: TimeStamp,
    input: &[u8],
) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
    let Some((sensor, value)) = try_decode_midi_input(input)? else {
        return Ok(None);
    };
    let input = ControlRegister {
        index: sensor.into(),
        value,
    };
    let event = ControlInputEvent { ts, input };
    Ok(Some(event))
}

#[derive(Debug, Clone, Default)]
pub struct MidiInputEventDecoder;

impl crate::MidiInputEventDecoder for MidiInputEventDecoder {
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
        try_decode_midi_input_event(ts, input)
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
