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
    midi::MidiPortDescriptor, ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent,
    ControlRegister, EmitInputEvent, MidiDeviceDescriptor, MidiInputConnector, MidiInputHandler,
    SliderEncoderInput, SliderInput, StepEncoderInput, TimeStamp,
};

fn u7_to_button(input: u8) -> ButtonInput {
    match input {
        0x00 => ButtonInput::Released,
        0x7f => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    BrowseKnobShift, // Encoder knob acts like a button when shifted
    Tap,
    TapHold,
    TouchPadLowerLeft,
    TouchPadLowerRight,
    TouchPadMode, // 0: X/Y Sliders, 1: 4 Buttons
    TouchPadUpperLeft,
    TouchPadUpperRight,
}

#[derive(Debug, Clone, Copy)]
pub enum CenterSlider {
    Crossfader,
}

#[derive(Debug, Clone, Copy)]
pub enum StepEncoder {
    BrowseKnob,
    ProgramKnob,
}

#[derive(Debug, Clone, Copy)]
pub enum Slider {
    AudiolessMonitorLevel,
    AudiolessMonitorMix,
    AudiolessMasterLevel,
    TouchPadX,
    TouchPadY,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Layer {
    #[default]
    Plain,
    Shift,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckButton {
    Fx,
    Load,
    Monitor,
    Shift,
    TouchStripLeft,   // Pitch bend down
    TouchStripCenter, // Vinyl mode switch
    TouchStripRight,  // Pitch bend up
    TouchStripLoopLeft,
    TouchStripLoopCenter,
    TouchStripLoopRight,
    TouchStripHotCueLeft,
    TouchStripHotCueCenter,
    TouchStripHotCueRight,
    TouchWheelScratch,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckLayerButton {
    Cue,
    PlayPause,
    Sync,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckSlider {
    LevelFader,
    TouchStrip,
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
pub enum DeckSliderEncoder {
    TouchWheelBend,
    TouchWheelScratch,
    TouchWheelSearch,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckCenterSlider {
    GainKnob,
    EqHiKnob,
    EqLoKnob,
    EqMidKnob,
    PitchFader,
}

#[derive(Debug)]
pub enum Input {
    Button {
        ctrl: Button,
        input: ButtonInput,
    },
    Slider {
        ctrl: Slider,
        input: SliderInput,
    },
    CenterSlider {
        ctrl: CenterSlider,
        input: CenterSliderInput,
    },
    StepEncoder {
        ctrl: StepEncoder,
        input: StepEncoderInput,
    },
    Deck {
        deck: Deck,
        input: DeckInput,
    },
}

#[derive(Debug)]
pub enum DeckInput {
    Button {
        ctrl: DeckButton,
        input: ButtonInput,
    },
    LayerButton {
        ctrl: DeckLayerButton,
        layer: Layer,
        input: ButtonInput,
    },
    Slider {
        ctrl: DeckSlider,
        input: SliderInput,
    },
    CenterSlider {
        ctrl: DeckCenterSlider,
        input: CenterSliderInput,
    },
    SliderEncoder {
        ctrl: DeckSliderEncoder,
        input: SliderEncoderInput,
    },
}

fn midi_status_to_deck(status: u8) -> Deck {
    match status & 0xf {
        MIDI_CHANNEL_DECK_A => Deck::A,
        MIDI_CHANNEL_DECK_B => Deck::B,
        _ => unreachable!("Unexpected MIDI status {status}"),
    }
}

impl Input {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn try_from_midi_input(input: &[u8]) -> Option<Self> {
        let mapped = match *input {
            [MIDI_STATUS_BUTTON, data1, data2] => match data1 {
                MIDI_BROWSE_KNOB_SHIFT_BUTTON => Input::Button {
                    ctrl: Button::BrowseKnobShift,
                    input: u7_to_button(data2),
                },
                MIDI_TAP_BUTTON => Input::Button {
                    ctrl: Button::Tap,
                    input: u7_to_button(data2),
                },
                MIDI_TAP_HOLD_BUTTON => Input::Button {
                    ctrl: Button::TapHold,
                    input: u7_to_button(data2),
                },
                MIDI_TOUCHPAD_MODE_BUTTON => Input::Button {
                    ctrl: Button::TouchPadMode,
                    input: u7_to_button(data2),
                },
                MIDI_TOUCHPAD_UPPER_LEFT_BUTTON => Input::Button {
                    ctrl: Button::TouchPadUpperLeft,
                    input: u7_to_button(data2),
                },
                MIDI_TOUCHPAD_UPPER_RIGHT_BUTTON => Input::Button {
                    ctrl: Button::TouchPadUpperRight,
                    input: u7_to_button(data2),
                },
                MIDI_TOUCHPAD_LOWER_LEFT_BUTTON => Input::Button {
                    ctrl: Button::TouchPadLowerLeft,
                    input: u7_to_button(data2),
                },
                MIDI_TOUCHPAD_LOWER_RIGHT_BUTTON => Input::Button {
                    ctrl: Button::TouchPadLowerRight,
                    input: u7_to_button(data2),
                },
                _ => unreachable!(),
            },
            [status @ (MIDI_STATUS_BUTTON_DECK_A | MIDI_STATUS_BUTTON_DECK_B), data1, data2] => {
                let deck = midi_status_to_deck(status);
                match data1 {
                    MIDI_DECK_LOAD_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Load,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopLeft,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopCenter,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopRight,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueLeft,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueCenter,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueRight,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLeft,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripCenter,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripRight,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_FX_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Fx,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_MONITOR_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Monitor,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_SHIFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Shift,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_PLAYPAUSE_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::PlayPause,
                            layer: Layer::Plain,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_SYNC_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Sync,
                            layer: Layer::Plain,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_CUE_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Cue,
                            layer: Layer::Plain,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_TOUCHWHEEL_SCRATCH_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchWheelScratch,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_PLAYPAUSE_SHIFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::PlayPause,
                            layer: Layer::Shift,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_SYNC_SHIFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Sync,
                            layer: Layer::Shift,
                            input: u7_to_button(data2),
                        },
                    },
                    MIDI_DECK_CUE_SHIFT_BUTTON => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Cue,
                            layer: Layer::Shift,
                            input: u7_to_button(data2),
                        },
                    },
                    _ => unreachable!(),
                }
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
                Self::Slider {
                    ctrl: Slider::TouchPadX,
                    input: SliderInput::from_u7(data2),
                }
            }
            [status @ (MIDI_STATUS_CC | MIDI_STATUS_CC_DECK_A), MIDI_TOUCHPAD_Y, data2] => {
                // The X/Y coordinates of the touch pad are always sent twice for
                // unknown reasons. According to the documentation they should
                // be sent on the main channel instead of on both deck channels.
                debug_assert_ne!(MIDI_STATUS_CC, status);
                debug_assert_eq!(MIDI_STATUS_CC_DECK_A, status);
                Self::Slider {
                    ctrl: Slider::TouchPadY,
                    input: SliderInput::from_u7(data2),
                }
            }
            [MIDI_STATUS_CC, data1, data2] => match data1 {
                MIDI_MONITOR_LEVEL_KNOB => Self::Slider {
                    ctrl: Slider::AudiolessMonitorLevel,
                    input: SliderInput::from_u7(data2),
                },
                MIDI_MONITOR_MIX_KNOB => Self::Slider {
                    ctrl: Slider::AudiolessMonitorMix,
                    input: SliderInput::from_u7(data2),
                },
                MIDI_MASTER_LEVEL_KNOB => Self::Slider {
                    ctrl: Slider::AudiolessMasterLevel,
                    input: SliderInput::from_u7(data2),
                },
                MIDI_CROSSFADER => Self::CenterSlider {
                    ctrl: CenterSlider::Crossfader,
                    input: CenterSliderInput::from_u7(data2),
                },
                MIDI_BROWSE_KNOB => Self::StepEncoder {
                    ctrl: StepEncoder::BrowseKnob,
                    input: StepEncoderInput::from_u7(data2),
                },
                MIDI_PROGRAM_KNOB => Self::StepEncoder {
                    ctrl: StepEncoder::ProgramKnob,
                    input: StepEncoderInput::from_u7(data2),
                },
                _ => unreachable!(),
            },
            [status @ (MIDI_STATUS_CC_DECK_A | MIDI_STATUS_CC_DECK_B), data1, data2] => {
                let deck = midi_status_to_deck(status);
                match data1 {
                    MIDI_DECK_TOUCHWHEEL_BEND => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelBend,
                            input: SliderEncoderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_TOUCHWHEEL_SEARCH => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelSearch,
                            input: SliderEncoderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_TOUCHWHEEL_SCRATCH => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelScratch,
                            input: SliderEncoderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_LEVEL_FADER => Self::Deck {
                        deck,
                        input: DeckInput::Slider {
                            ctrl: DeckSlider::LevelFader,
                            input: SliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_PITCH_FADER => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::PitchFader,
                            input: CenterSliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_GAIN_KNOB => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::GainKnob,
                            input: CenterSliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_EQ_HI_KNOB => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::EqHiKnob,
                            input: CenterSliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_EQ_MID_KNOB => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::EqMidKnob,
                            input: CenterSliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_EQ_LO_KNOB => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::EqLoKnob,
                            input: CenterSliderInput::from_u7(data2),
                        },
                    },
                    MIDI_DECK_TOUCHSTRIP => Self::Deck {
                        deck,
                        input: DeckInput::Slider {
                            ctrl: DeckSlider::TouchStrip,
                            input: SliderInput::from_u7(data2),
                        },
                    },
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        Some(mapped)
    }
}

pub type InputEvent = crate::InputEvent<Input>;

impl From<InputEvent> for ControlInputEvent {
    fn from(from: InputEvent) -> Self {
        let InputEvent { ts, input } = from;
        Self {
            ts,
            input: input.into(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct InputGateway<E> {
    emit_input_event: E,
}

impl<E> InputGateway<E> {
    #[must_use]
    pub fn attach(emit_input_event: E) -> Self {
        Self { emit_input_event }
    }

    #[must_use]
    pub fn detach(self) -> E {
        let Self { emit_input_event } = self;
        emit_input_event
    }
}

impl<E> MidiInputHandler for InputGateway<E>
where
    E: EmitInputEvent<Input> + Send,
{
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        let Some(input) = Input::try_from_midi_input(input) else {
            return false;
        };
        let event = InputEvent { ts, input };
        log::debug!("Emitting {event:?}");
        self.emit_input_event.emit_input_event(event);
        true
    }
}

#[must_use]
pub fn try_decode_midi_input(ts: TimeStamp, input: &[u8]) -> Option<ControlInputEvent> {
    let Some(input) = Input::try_from_midi_input(input) else {
        log::debug!("[{ts}] Cannot decode MIDI input: {input:x?}");
        return None;
    };
    let event = InputEvent { ts, input }.into();
    Some(event)
}

impl<E> MidiInputConnector for InputGateway<E>
where
    E: Send,
{
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        port: &MidiPortDescriptor,
    ) {
        log::debug!("Device \"{device:?}\" is connected to port \"{port:?}\"");
    }
}

/// Flattened enumeration of all input sensors
#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u32)]
pub enum Sensor {
    // Button
    BrowseKnobShiftButton,
    TapButton,
    TapHoldButton,
    TouchPadModeButton,
    TouchPadLowerLeftButton,
    TouchPadLowerRightButton,
    TouchPadUpperLeftButton,
    TouchPadUpperRightButton,
    // CenterSlider
    CrossfaderCenterSlider,
    // StepEncoder
    BrowseKnobStepEncoder,
    ProgramKnobStepEncoder,
    // Slider
    AudiolessMonitorLevel,
    AudiolessMonitorMix,
    AudiolessMasterLevel,
    TouchPadXSlider,
    TouchPadYSlider,
    // Deck A: Button
    DeckAFxButton,
    DeckALoadButton,
    DeckAMonitorButton,
    DeckAShiftButton,
    DeckATouchStripCenterButton,
    DeckATouchStripHotCueCenterButton,
    DeckATouchStripHotCueLeftButton,
    DeckATouchStripHotCueRightButton,
    DeckATouchStripLeftButton,
    DeckATouchStripLoopCenterButton,
    DeckATouchStripLoopLeftButton,
    DeckATouchStripLoopRightButton,
    DeckATouchStripRightButton,
    DeckATouchWheelScratchButton,
    // Deck A: LayerButton
    DeckACueButton,
    DeckACueShiftButton,
    DeckAPlayPauseButton,
    DeckAPlayPauseShiftButton,
    DeckASyncButton,
    DeckASyncShiftButton,
    // Deck A: Slider
    DeckALevelFaderSlider,
    DeckATouchStripSlider,
    // Deck A: SliderEncoder
    DeckATouchWheelBendSliderEncoder,
    DeckATouchWheelScratchSliderEncoder,
    DeckATouchWheelSearchSliderEncoder,
    // Deck A: CenterSlider
    DeckAGainKnobCenterSlider,
    DeckAEqHiKnobCenterSlider,
    DeckAEqLoKnobCenterSlider,
    DeckAEqMidKnobCenterSlider,
    DeckAPitchFaderCenterSlider,
    // Deck B: Button
    DeckBFxButton,
    DeckBLoadButton,
    DeckBMonitorButton,
    DeckBShiftButton,
    DeckBTouchStripLeftButton,
    DeckBTouchStripCenterButton,
    DeckBTouchStripRightButton,
    DeckBTouchStripLoopLeftButton,
    DeckBTouchStripLoopCenterButton,
    DeckBTouchStripLoopRightButton,
    DeckBTouchStripHotCueLeftButton,
    DeckBTouchStripHotCueCenterButton,
    DeckBTouchStripHotCueRightButton,
    DeckBTouchWheelScratchButton,
    // Deck B: LayerButton
    DeckBCueButton,
    DeckBCueShiftButton,
    DeckBPlayPauseButton,
    DeckBPlayPauseShiftButton,
    DeckBSyncButton,
    DeckBSyncShiftButton,
    // Deck B: Slider
    DeckBLevelFaderSlider,
    DeckBTouchStripSlider,
    // Deck B: SliderEncoder
    DeckBTouchWheelBendSliderEncoder,
    DeckBTouchWheelScratchSliderEncoder,
    DeckBTouchWheelSearchSliderEncoder,
    // Deck B: CenterSlider
    DeckBGainKnobCenterSlider,
    DeckBEqHiKnobCenterSlider,
    DeckBEqLoKnobCenterSlider,
    DeckBEqMidKnobCenterSlider,
    DeckBPitchFaderCenterSlider,
}

impl From<Sensor> for ControlIndex {
    fn from(value: Sensor) -> Self {
        ControlIndex::new(value as u32)
    }
}

#[derive(Debug)]
pub struct InvalidControlIndex;

impl TryFrom<ControlIndex> for Sensor {
    type Error = InvalidControlIndex;

    fn try_from(index: ControlIndex) -> Result<Self, Self::Error> {
        Self::from_repr(index.value()).ok_or(InvalidControlIndex)
    }
}

impl From<Input> for ControlRegister {
    #[allow(clippy::too_many_lines)]
    fn from(from: Input) -> Self {
        let (sensor, value) = match from {
            Input::Button { ctrl, input } => {
                let input = input.into();
                let sensor = match ctrl {
                    Button::BrowseKnobShift => Sensor::BrowseKnobShiftButton,
                    Button::Tap => Sensor::TapButton,
                    Button::TapHold => Sensor::TapHoldButton,
                    Button::TouchPadMode => Sensor::TouchPadModeButton,
                    Button::TouchPadLowerLeft => Sensor::TouchPadLowerLeftButton,
                    Button::TouchPadLowerRight => Sensor::TouchPadLowerRightButton,
                    Button::TouchPadUpperLeft => Sensor::TouchPadUpperLeftButton,
                    Button::TouchPadUpperRight => Sensor::TouchPadUpperRightButton,
                };
                (sensor, input)
            }
            Input::Slider { ctrl, input } => {
                let input = input.into();
                let sensor = match ctrl {
                    Slider::AudiolessMonitorLevel => Sensor::AudiolessMonitorLevel,
                    Slider::AudiolessMonitorMix => Sensor::AudiolessMonitorMix,
                    Slider::AudiolessMasterLevel => Sensor::AudiolessMasterLevel,
                    Slider::TouchPadX => Sensor::TouchPadXSlider,
                    Slider::TouchPadY => Sensor::TouchPadYSlider,
                };
                (sensor, input)
            }
            Input::CenterSlider { ctrl, input } => {
                let input = input.into();
                let sensor = match ctrl {
                    CenterSlider::Crossfader => Sensor::CrossfaderCenterSlider,
                };
                (sensor, input)
            }
            Input::StepEncoder { ctrl, input } => {
                let input = input.into();
                let sensor = match ctrl {
                    StepEncoder::BrowseKnob => Sensor::BrowseKnobStepEncoder,
                    StepEncoder::ProgramKnob => Sensor::ProgramKnobStepEncoder,
                };
                (sensor, input)
            }
            Input::Deck { deck, input } => match deck {
                Deck::A => match input {
                    DeckInput::Button { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckButton::Fx => Sensor::DeckAFxButton,
                            DeckButton::Load => Sensor::DeckALoadButton,
                            DeckButton::Monitor => Sensor::DeckAMonitorButton,
                            DeckButton::Shift => Sensor::DeckAShiftButton,
                            DeckButton::TouchStripCenter => Sensor::DeckATouchStripCenterButton,
                            DeckButton::TouchStripHotCueCenter => {
                                Sensor::DeckATouchStripHotCueCenterButton
                            }
                            DeckButton::TouchStripHotCueLeft => {
                                Sensor::DeckATouchStripHotCueLeftButton
                            }
                            DeckButton::TouchStripHotCueRight => {
                                Sensor::DeckATouchStripHotCueRightButton
                            }
                            DeckButton::TouchStripLeft => Sensor::DeckATouchStripLeftButton,
                            DeckButton::TouchStripLoopCenter => {
                                Sensor::DeckATouchStripLoopCenterButton
                            }
                            DeckButton::TouchStripLoopLeft => Sensor::DeckATouchStripLoopLeftButton,
                            DeckButton::TouchStripLoopRight => {
                                Sensor::DeckATouchStripLoopRightButton
                            }
                            DeckButton::TouchStripRight => Sensor::DeckATouchStripRightButton,
                            DeckButton::TouchWheelScratch => Sensor::DeckATouchWheelScratchButton,
                        };
                        (sensor, input)
                    }
                    DeckInput::LayerButton { ctrl, layer, input } => {
                        let input = input.into();
                        let sensor = match (ctrl, layer) {
                            (DeckLayerButton::Cue, Layer::Plain) => Sensor::DeckACueButton,
                            (DeckLayerButton::Cue, Layer::Shift) => Sensor::DeckACueShiftButton,
                            (DeckLayerButton::PlayPause, Layer::Plain) => {
                                Sensor::DeckAPlayPauseButton
                            }
                            (DeckLayerButton::PlayPause, Layer::Shift) => {
                                Sensor::DeckAPlayPauseShiftButton
                            }
                            (DeckLayerButton::Sync, Layer::Plain) => Sensor::DeckASyncButton,
                            (DeckLayerButton::Sync, Layer::Shift) => Sensor::DeckASyncShiftButton,
                        };
                        (sensor, input)
                    }
                    DeckInput::Slider { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckSlider::LevelFader => Sensor::DeckALevelFaderSlider,
                            DeckSlider::TouchStrip => Sensor::DeckATouchStripSlider,
                        };
                        (sensor, input)
                    }
                    DeckInput::CenterSlider { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckCenterSlider::GainKnob => Sensor::DeckAGainKnobCenterSlider,
                            DeckCenterSlider::EqHiKnob => Sensor::DeckAEqHiKnobCenterSlider,
                            DeckCenterSlider::EqLoKnob => Sensor::DeckAEqLoKnobCenterSlider,
                            DeckCenterSlider::EqMidKnob => Sensor::DeckAEqMidKnobCenterSlider,
                            DeckCenterSlider::PitchFader => Sensor::DeckAPitchFaderCenterSlider,
                        };
                        (sensor, input)
                    }
                    DeckInput::SliderEncoder { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckSliderEncoder::TouchWheelBend => {
                                Sensor::DeckATouchWheelBendSliderEncoder
                            }
                            DeckSliderEncoder::TouchWheelScratch => {
                                Sensor::DeckATouchWheelScratchSliderEncoder
                            }
                            DeckSliderEncoder::TouchWheelSearch => {
                                Sensor::DeckATouchWheelSearchSliderEncoder
                            }
                        };
                        (sensor, input)
                    }
                },
                Deck::B => match input {
                    DeckInput::Button { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckButton::Fx => Sensor::DeckBFxButton,
                            DeckButton::Load => Sensor::DeckBLoadButton,
                            DeckButton::Monitor => Sensor::DeckBMonitorButton,
                            DeckButton::Shift => Sensor::DeckBShiftButton,
                            DeckButton::TouchStripCenter => Sensor::DeckBTouchStripCenterButton,
                            DeckButton::TouchStripHotCueCenter => {
                                Sensor::DeckBTouchStripHotCueCenterButton
                            }
                            DeckButton::TouchStripHotCueLeft => {
                                Sensor::DeckBTouchStripHotCueLeftButton
                            }
                            DeckButton::TouchStripHotCueRight => {
                                Sensor::DeckBTouchStripHotCueRightButton
                            }
                            DeckButton::TouchStripLeft => Sensor::DeckBTouchStripLeftButton,
                            DeckButton::TouchStripLoopCenter => {
                                Sensor::DeckBTouchStripLoopCenterButton
                            }
                            DeckButton::TouchStripLoopLeft => Sensor::DeckBTouchStripLoopLeftButton,
                            DeckButton::TouchStripLoopRight => {
                                Sensor::DeckBTouchStripLoopRightButton
                            }
                            DeckButton::TouchStripRight => Sensor::DeckBTouchStripRightButton,
                            DeckButton::TouchWheelScratch => Sensor::DeckBTouchWheelScratchButton,
                        };
                        (sensor, input)
                    }
                    DeckInput::LayerButton { ctrl, layer, input } => {
                        let input = input.into();
                        let sensor = match (ctrl, layer) {
                            (DeckLayerButton::Cue, Layer::Plain) => Sensor::DeckBCueButton,
                            (DeckLayerButton::Cue, Layer::Shift) => Sensor::DeckBCueShiftButton,
                            (DeckLayerButton::PlayPause, Layer::Plain) => {
                                Sensor::DeckBPlayPauseButton
                            }
                            (DeckLayerButton::PlayPause, Layer::Shift) => {
                                Sensor::DeckBPlayPauseShiftButton
                            }
                            (DeckLayerButton::Sync, Layer::Plain) => Sensor::DeckBSyncButton,
                            (DeckLayerButton::Sync, Layer::Shift) => Sensor::DeckBSyncShiftButton,
                        };
                        (sensor, input)
                    }
                    DeckInput::Slider { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckSlider::LevelFader => Sensor::DeckBLevelFaderSlider,
                            DeckSlider::TouchStrip => Sensor::DeckBTouchStripSlider,
                        };
                        (sensor, input)
                    }
                    DeckInput::CenterSlider { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckCenterSlider::GainKnob => Sensor::DeckBGainKnobCenterSlider,
                            DeckCenterSlider::EqHiKnob => Sensor::DeckBEqHiKnobCenterSlider,
                            DeckCenterSlider::EqLoKnob => Sensor::DeckBEqLoKnobCenterSlider,
                            DeckCenterSlider::EqMidKnob => Sensor::DeckBEqMidKnobCenterSlider,
                            DeckCenterSlider::PitchFader => Sensor::DeckBPitchFaderCenterSlider,
                        };
                        (sensor, input)
                    }
                    DeckInput::SliderEncoder { ctrl, input } => {
                        let input = input.into();
                        let sensor = match ctrl {
                            DeckSliderEncoder::TouchWheelBend => {
                                Sensor::DeckBTouchWheelBendSliderEncoder
                            }
                            DeckSliderEncoder::TouchWheelScratch => {
                                Sensor::DeckBTouchWheelScratchSliderEncoder
                            }
                            DeckSliderEncoder::TouchWheelSearch => {
                                Sensor::DeckBTouchWheelSearchSliderEncoder
                            }
                        };
                        (sensor, input)
                    }
                },
            },
        };
        Self {
            index: sensor.into(),
            value,
        }
    }
}
