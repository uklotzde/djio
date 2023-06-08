// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr};

use super::{
    Deck, MIDI_BROWSE_KNOB, MIDI_BROWSE_KNOB_SHIFT_BUTTON, MIDI_CHANNEL_DECK_A,
    MIDI_CHANNEL_DECK_B, MIDI_CROSSFADER, MIDI_DECK_CUE_BUTTON, MIDI_DECK_CUE_SHIFT_BUTTON,
    MIDI_DECK_EQ_HI_KNOB, MIDI_DECK_EQ_LO_KNOB, MIDI_DECK_EQ_MID_KNOB, MIDI_DECK_FX_BUTTON,
    MIDI_DECK_GAIN_KNOB, MIDI_DECK_LEVEL_FADER, MIDI_DECK_LOAD_BUTTON, MIDI_DECK_MONITOR_BUTTON,
    MIDI_DECK_PITCH_FADER, MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_PLAYPAUSE_SHIFT_BUTTON,
    MIDI_DECK_SHIFT_BUTTON, MIDI_DECK_SYNC_BUTTON, MIDI_DECK_SYNC_SHIFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP, MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
    MIDI_DECK_TOUCHWHEEL_BEND, MIDI_DECK_TOUCHWHEEL_SCRATCH, MIDI_DECK_TOUCHWHEEL_SCRATCH_BUTTON,
    MIDI_DECK_TOUCHWHEEL_SEARCH, MIDI_MASTER_LEVEL_KNOB, MIDI_MONITOR_LEVEL_KNOB,
    MIDI_MONITOR_MIX_KNOB, MIDI_PROGRAM_KNOB, MIDI_STATUS_BUTTON, MIDI_STATUS_BUTTON_DECK_A,
    MIDI_STATUS_BUTTON_DECK_B, MIDI_STATUS_CC, MIDI_STATUS_CC_DECK_A, MIDI_STATUS_CC_DECK_B,
    MIDI_TAP_BUTTON, MIDI_TAP_HOLD_BUTTON, MIDI_TOUCHPAD_LOWER_LEFT_BUTTON,
    MIDI_TOUCHPAD_LOWER_RIGHT_BUTTON, MIDI_TOUCHPAD_MODE_BUTTON, MIDI_TOUCHPAD_UPPER_LEFT_BUTTON,
    MIDI_TOUCHPAD_UPPER_RIGHT_BUTTON, MIDI_TOUCHPAD_X, MIDI_TOUCHPAD_Y,
};
use crate::{
    ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent, ControlRegister, Input,
    SliderEncoderInput, SliderInput, StepEncoderInput, TimeStamp,
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
    LevelFaderSlider,
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

const SENSOR_CONTROL_INDEX_DECK_A: u32 = 0x0100;
const SENSOR_CONTROL_INDEX_DECK_B: u32 = 0x0200;

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
                    Deck::A => SENSOR_CONTROL_INDEX_DECK_A,
                    Deck::B => SENSOR_CONTROL_INDEX_DECK_B,
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
pub struct InvalidSensorControlIndex;

impl TryFrom<ControlIndex> for Sensor {
    type Error = InvalidSensorControlIndex;

    fn try_from(from: ControlIndex) -> Result<Self, Self::Error> {
        const DECK_BIT_MASK: u32 = SENSOR_CONTROL_INDEX_DECK_A | SENSOR_CONTROL_INDEX_DECK_B;
        const ENUM_BIT_MASK: u32 = (1 << DECK_BIT_MASK.trailing_zeros()) - 1;
        debug_assert!(ENUM_BIT_MASK <= u8::MAX.into());
        let value = from.value();
        let enum_index = (value & ENUM_BIT_MASK) as u8;
        let deck = match value & DECK_BIT_MASK {
            SENSOR_CONTROL_INDEX_DECK_A => Deck::A,
            SENSOR_CONTROL_INDEX_DECK_B => Deck::B,
            DECK_BIT_MASK => return Err(InvalidSensorControlIndex),
            _ => {
                return MainSensor::from_repr(enum_index)
                    .map(Sensor::Main)
                    .ok_or(InvalidSensorControlIndex);
            }
        };
        DeckSensor::from_repr(enum_index)
            .map(|sensor| Sensor::Deck(deck, sensor))
            .ok_or(InvalidSensorControlIndex)
    }
}

fn midi_status_to_deck(status: u8) -> Deck {
    match status & 0xf {
        MIDI_CHANNEL_DECK_A => Deck::A,
        MIDI_CHANNEL_DECK_B => Deck::B,
        _ => unreachable!("Unexpected MIDI status {status}"),
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn try_decode_midi_input(input: &[u8]) -> Option<(Sensor, Input)> {
    let decoded = match *input {
        [MIDI_STATUS_BUTTON, data1, data2] => {
            let input = u7_to_button(data2);
            let sensor = match data1 {
                MIDI_BROWSE_KNOB_SHIFT_BUTTON => MainSensor::BrowseKnobShiftButton,
                MIDI_TAP_BUTTON => MainSensor::TapButton,
                MIDI_TAP_HOLD_BUTTON => MainSensor::TapHoldButton,
                MIDI_TOUCHPAD_MODE_BUTTON => MainSensor::TouchPadModeButton,
                MIDI_TOUCHPAD_UPPER_LEFT_BUTTON => MainSensor::TouchPadUpperLeftButton,
                MIDI_TOUCHPAD_UPPER_RIGHT_BUTTON => MainSensor::TouchPadUpperRightButton,
                MIDI_TOUCHPAD_LOWER_LEFT_BUTTON => MainSensor::TouchPadLowerLeftButton,
                MIDI_TOUCHPAD_LOWER_RIGHT_BUTTON => MainSensor::TouchPadLowerRightButton,
                _ => {
                    return None;
                }
            };
            (sensor.into(), input.into())
        }
        [status @ (MIDI_STATUS_BUTTON_DECK_A | MIDI_STATUS_BUTTON_DECK_B), data1, data2] => {
            let input = u7_to_button(data2);
            let deck = midi_status_to_deck(status);
            let sensor = match data1 {
                MIDI_DECK_LOAD_BUTTON => DeckSensor::LoadButton,
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
                MIDI_DECK_FX_BUTTON => DeckSensor::FxButton,
                MIDI_DECK_MONITOR_BUTTON => DeckSensor::MonitorButton,
                MIDI_DECK_SHIFT_BUTTON => DeckSensor::ShiftButton,
                MIDI_DECK_PLAYPAUSE_BUTTON => DeckSensor::PlayPauseButton,
                MIDI_DECK_SYNC_BUTTON => DeckSensor::SyncButton,
                MIDI_DECK_CUE_BUTTON => DeckSensor::CueButton,
                MIDI_DECK_TOUCHWHEEL_SCRATCH_BUTTON => DeckSensor::TouchWheelScratchButton,
                MIDI_DECK_PLAYPAUSE_SHIFT_BUTTON => DeckSensor::PlayPauseShiftButton,
                MIDI_DECK_SYNC_SHIFT_BUTTON => DeckSensor::SyncShiftButton,
                MIDI_DECK_CUE_SHIFT_BUTTON => DeckSensor::CueShiftButton,
                _ => {
                    return None;
                }
            };
            (Sensor::Deck(deck, sensor), input.into())
        }
        [MIDI_STATUS_CC_DECK_B, MIDI_TOUCHPAD_X | MIDI_TOUCHPAD_Y, _data2] => {
            // Filter duplicate touch pad messages for deck B,
            // see the comments in next match expression.
            return None;
        }
        [status @ (MIDI_STATUS_CC | MIDI_STATUS_CC_DECK_A), MIDI_TOUCHPAD_X, data2] => {
            // The X/Y coordinates of the touch pad are always sent twice for
            // unknown reasons. According to the documentation they should
            // be sent on the main channel instead of on both deck channels.
            debug_assert_ne!(MIDI_STATUS_CC, status);
            debug_assert_eq!(MIDI_STATUS_CC_DECK_A, status);
            let input = SliderInput::from_u7(data2);
            (MainSensor::TouchPadXSlider.into(), input.into())
        }
        [status @ (MIDI_STATUS_CC | MIDI_STATUS_CC_DECK_A), MIDI_TOUCHPAD_Y, data2] => {
            // The X/Y coordinates of the touch pad are always sent twice for
            // unknown reasons. According to the documentation they should
            // be sent on the main channel instead of on both deck channels.
            debug_assert_ne!(MIDI_STATUS_CC, status);
            debug_assert_eq!(MIDI_STATUS_CC_DECK_A, status);
            let input = SliderInput::from_u7(data2);
            (MainSensor::TouchPadYSlider.into(), input.into())
        }
        [MIDI_STATUS_CC, data1, data2] => match data1 {
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
            MIDI_CROSSFADER => (
                MainSensor::CrossfaderCenterSlider.into(),
                CenterSliderInput::from_u7(data2).into(),
            ),
            MIDI_BROWSE_KNOB => (
                MainSensor::BrowseKnobStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            MIDI_PROGRAM_KNOB => (
                MainSensor::ProgramKnobStepEncoder.into(),
                StepEncoderInput::from_u7(data2).into(),
            ),
            _ => {
                return None;
            }
        },
        [status @ (MIDI_STATUS_CC_DECK_A | MIDI_STATUS_CC_DECK_B), data1, data2] => {
            let deck = midi_status_to_deck(status);
            let (sensor, input) = match data1 {
                MIDI_DECK_TOUCHWHEEL_BEND => (
                    DeckSensor::TouchWheelBendSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_TOUCHWHEEL_SEARCH => (
                    DeckSensor::TouchWheelSearchSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_TOUCHWHEEL_SCRATCH => (
                    DeckSensor::TouchWheelScratchSliderEncoder,
                    SliderEncoderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_LEVEL_FADER => (
                    DeckSensor::LevelFaderSlider,
                    SliderInput::from_u7(data2).into(),
                ),
                MIDI_DECK_PITCH_FADER => (
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
                MIDI_DECK_TOUCHSTRIP => (
                    DeckSensor::TouchStripSlider,
                    SliderInput::from_u7(data2).into(),
                ),
                _ => {
                    return None;
                }
            };
            (Sensor::Deck(deck, sensor), input)
        }
        _ => {
            return None;
        }
    };
    Some(decoded)
}

#[must_use]
pub fn try_decode_midi_message(ts: TimeStamp, input: &[u8]) -> Option<ControlInputEvent> {
    let Some((sensor, input)) = try_decode_midi_input(input) else {
        return None;
    };
    let input = ControlRegister {
        index: sensor.into(),
        value: input.into(),
    };
    let event = ControlInputEvent { ts, input };
    Some(event)
}
