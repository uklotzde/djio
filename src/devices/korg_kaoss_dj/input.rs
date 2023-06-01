// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive as _;

use super::Deck;
use crate::{
    ButtonInput, CenterSliderInput, ControlIndex, ControlInput, EmitInputEvent,
    MidiDeviceDescriptor, MidiInputHandler, SliderEncoderInput, SliderInput, StepEncoderInput,
    TimeStamp,
};

fn u7_to_button(input: u8) -> ButtonInput {
    match input {
        0 => ButtonInput::Released,
        127 => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

fn u7_to_step_encoder(input: u8) -> StepEncoderInput {
    let delta = match input {
        1 => 1,
        127 => -1,
        _ => unreachable!(),
    };
    StepEncoderInput { delta }
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
    CrossFader,
}

#[derive(Debug, Clone, Copy)]
pub enum StepEncoder {
    BrowseKnob,
    ProgramKnob,
}

#[derive(Debug, Clone, Copy)]
pub enum Slider {
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
    HiEqKnob,
    LoEqKnob,
    MidEqKnob,
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

impl Input {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn try_from_midi_message(input: &[u8]) -> Option<Self> {
        let mapped = match input {
            [0x96, data1, data2] => {
                // Global buttons (MIDI channel 7)
                match data1 {
                    0x07 => Input::Button {
                        ctrl: Button::BrowseKnobShift,
                        input: u7_to_button(*data2),
                    },
                    0x0b => Input::Button {
                        ctrl: Button::Tap,
                        input: u7_to_button(*data2),
                    },
                    0x21 => Input::Button {
                        ctrl: Button::TapHold,
                        input: u7_to_button(*data2),
                    },
                    0x22 => Input::Button {
                        ctrl: Button::TouchPadMode,
                        input: u7_to_button(*data2),
                    },
                    0x4a => Input::Button {
                        ctrl: Button::TouchPadUpperLeft,
                        input: u7_to_button(*data2),
                    },
                    0x4b => Input::Button {
                        ctrl: Button::TouchPadUpperRight,
                        input: u7_to_button(*data2),
                    },
                    0x4c => Input::Button {
                        ctrl: Button::TouchPadLowerLeft,
                        input: u7_to_button(*data2),
                    },
                    0x4d => Input::Button {
                        ctrl: Button::TouchPadLowerRight,
                        input: u7_to_button(*data2),
                    },
                    _ => unreachable!(),
                }
            }
            [status @ (0x97 | 0x98), data1, data2] => {
                // Deck buttons (MIDI channel 8/9)
                let deck = match *status {
                    0x97 => Deck::A,
                    0x98 => Deck::B,
                    _ => unreachable!(),
                };
                match data1 {
                    0x0e => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Load,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x0f => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopLeft,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x10 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopCenter,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x11 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopRight,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x12 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueLeft,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x13 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueCenter,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x14 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueRight,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x15 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripLeft,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x16 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripCenter,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x17 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchStripRight,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x18 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Fx,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x19 => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Monitor,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1a => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::Shift,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1b => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::PlayPause,
                            layer: Layer::Plain,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1d => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Sync,
                            layer: Layer::Plain,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1e => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Cue,
                            layer: Layer::Plain,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1f => Self::Deck {
                        deck,
                        input: DeckInput::Button {
                            ctrl: DeckButton::TouchWheelScratch,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x2e => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::PlayPause,
                            layer: Layer::Shift,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x2f => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Sync,
                            layer: Layer::Shift,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x30 => Self::Deck {
                        deck,
                        input: DeckInput::LayerButton {
                            ctrl: DeckLayerButton::Cue,
                            layer: Layer::Shift,
                            input: u7_to_button(*data2),
                        },
                    },
                    _ => unreachable!(),
                }
            }
            [0xb8, 0x0c, _data2] => {
                // Filter duplicate touch pad messages for deck B,
                // see the comments in next match expression.
                return None;
            }
            [status @ (0xb6 | 0xb7), 0x0c, data2] => {
                // The X/Y coordinates of the touch pad are always sent twice for
                // unknown reasons. According to the documentation they should
                // be sent on channel 7 (0xb6) instead of on channel 8 (0xb7)
                // and channel 9 (0xb8) for both decks.
                debug_assert_ne!(0xb6, *status);
                debug_assert_eq!(0xb7, *status);
                Self::Slider {
                    ctrl: Slider::TouchPadX,
                    input: SliderInput::from_u7(*data2),
                }
            }
            [0xb6 | 0xb7 | 0xb8, 0x0d, data2] => {
                // See the comment above for the X slider.
                Self::Slider {
                    ctrl: Slider::TouchPadY,
                    input: SliderInput::from_u7(*data2),
                }
            }
            [0xb6, data1, data2] => {
                // Global sliders and encoders (MIDI channel 7)
                match *data1 {
                    0x17 => Self::CenterSlider {
                        ctrl: CenterSlider::CrossFader,
                        input: CenterSliderInput::from_u7(*data2),
                    },
                    0x1e => Self::StepEncoder {
                        ctrl: StepEncoder::BrowseKnob,
                        input: u7_to_step_encoder(*data2),
                    },
                    0x1f => Self::StepEncoder {
                        ctrl: StepEncoder::ProgramKnob,
                        input: u7_to_step_encoder(*data2),
                    },
                    _ => unreachable!(),
                }
            }
            [status @ (0xb7 | 0xb8), data1, data2] => {
                // Deck sliders and encoders (MIDI channel 8/9)
                let deck = match *status {
                    0xb7 => Deck::A,
                    0xb8 => Deck::B,
                    _ => unreachable!(),
                };
                match *data1 {
                    0x0e => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelBend,
                            input: SliderEncoderInput::from_u7(*data2),
                        },
                    },
                    0x0f => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelSearch,
                            input: SliderEncoderInput::from_u7(*data2),
                        },
                    },
                    0x10 => Self::Deck {
                        deck,
                        input: DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelScratch,
                            input: SliderEncoderInput::from_u7(*data2),
                        },
                    },
                    0x18 => Self::Deck {
                        deck,
                        input: DeckInput::Slider {
                            ctrl: DeckSlider::LevelFader,
                            input: SliderInput::from_u7(*data2),
                        },
                    },
                    0x19 => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::PitchFader,
                            input: CenterSliderInput::from_u7(*data2),
                        },
                    },
                    0x1a => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::GainKnob,
                            input: CenterSliderInput::from_u7(*data2),
                        },
                    },
                    0x1b => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::HiEqKnob,
                            input: CenterSliderInput::from_u7(*data2),
                        },
                    },
                    0x1c => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::MidEqKnob,
                            input: CenterSliderInput::from_u7(*data2),
                        },
                    },
                    0x1d => Self::Deck {
                        deck,
                        input: DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::LoEqKnob,
                            input: CenterSliderInput::from_u7(*data2),
                        },
                    },
                    0x21 => Self::Deck {
                        deck,
                        input: DeckInput::Slider {
                            ctrl: DeckSlider::TouchStrip,
                            input: SliderInput::from_u7(*data2),
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
    fn connect_midi_input_port(
        &mut self,
        _device_descriptor: &MidiDeviceDescriptor,
        client_name: &str,
        port_name: &str,
        _port: &midir::MidiInputPort,
    ) {
        log::debug!("Device \"{client_name}\" is connected to port \"{port_name}\"");
    }

    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        let Some(input) = Input::try_from_midi_message(input) else {
            log::debug!("[{ts}] Unhandled MIDI input message: {input:x?}");
            return;
        };
        let event = InputEvent { ts, input };
        log::debug!("Emitting {event:?}");
        self.emit_input_event.emit_input_event(event);
    }
}

/// Flattened enumeration of all input sensors
#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
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
    CrossFaderCenterSlider,
    // StepEncoder
    BrowseKnobStepEncoder,
    ProgramKnobStepEncoder,
    // Slider
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
    DeckAHiEqKnobCenterSlider,
    DeckALoEqKnobCenterSlider,
    DeckAMidEqKnobCenterSlider,
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
    DeckBHiEqKnobCenterSlider,
    DeckBLoEqKnobCenterSlider,
    DeckBMidEqKnobCenterSlider,
    DeckBPitchFaderCenterSlider,
}

impl From<Sensor> for ControlIndex {
    fn from(value: Sensor) -> Self {
        ControlIndex::new(value.to_u32().expect("u32"))
    }
}

impl From<Input> for ControlInput {
    #[allow(clippy::too_many_lines)]
    fn from(from: Input) -> Self {
        let (ctrl, input) = match from {
            Input::Button { ctrl, input } => {
                let input = input.into();
                match ctrl {
                    Button::BrowseKnobShift => (Sensor::BrowseKnobShiftButton, input),
                    Button::Tap => (Sensor::TapButton, input),
                    Button::TapHold => (Sensor::TapHoldButton, input),
                    Button::TouchPadMode => (Sensor::TouchPadModeButton, input),
                    Button::TouchPadLowerLeft => (Sensor::TouchPadLowerLeftButton, input),
                    Button::TouchPadLowerRight => (Sensor::TouchPadLowerRightButton, input),
                    Button::TouchPadUpperLeft => (Sensor::TouchPadUpperLeftButton, input),
                    Button::TouchPadUpperRight => (Sensor::TouchPadUpperRightButton, input),
                }
            }
            Input::Slider { ctrl, input } => {
                let input = input.into();
                match ctrl {
                    Slider::TouchPadX => (Sensor::TouchPadXSlider, input),
                    Slider::TouchPadY => (Sensor::TouchPadYSlider, input),
                }
            }
            Input::CenterSlider { ctrl, input } => {
                let input = input.into();
                match ctrl {
                    CenterSlider::CrossFader => (Sensor::CrossFaderCenterSlider, input),
                }
            }
            Input::StepEncoder { ctrl, input } => {
                let input = input.into();
                match ctrl {
                    StepEncoder::BrowseKnob => (Sensor::BrowseKnobStepEncoder, input),
                    StepEncoder::ProgramKnob => (Sensor::ProgramKnobStepEncoder, input),
                }
            }
            Input::Deck { deck, input } => match deck {
                Deck::A => match input {
                    DeckInput::Button { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckButton::Fx => (Sensor::DeckAFxButton, input),
                            DeckButton::Load => (Sensor::DeckALoadButton, input),
                            DeckButton::Monitor => (Sensor::DeckAMonitorButton, input),
                            DeckButton::Shift => (Sensor::DeckAShiftButton, input),
                            DeckButton::TouchStripCenter => {
                                (Sensor::DeckATouchStripCenterButton, input)
                            }
                            DeckButton::TouchStripHotCueCenter => {
                                (Sensor::DeckATouchStripHotCueCenterButton, input)
                            }
                            DeckButton::TouchStripHotCueLeft => {
                                (Sensor::DeckATouchStripHotCueLeftButton, input)
                            }
                            DeckButton::TouchStripHotCueRight => {
                                (Sensor::DeckATouchStripHotCueRightButton, input)
                            }
                            DeckButton::TouchStripLeft => {
                                (Sensor::DeckATouchStripLeftButton, input)
                            }
                            DeckButton::TouchStripLoopCenter => {
                                (Sensor::DeckATouchStripLoopCenterButton, input)
                            }
                            DeckButton::TouchStripLoopLeft => {
                                (Sensor::DeckATouchStripLoopLeftButton, input)
                            }
                            DeckButton::TouchStripLoopRight => {
                                (Sensor::DeckATouchStripLoopRightButton, input)
                            }
                            DeckButton::TouchStripRight => {
                                (Sensor::DeckATouchStripRightButton, input)
                            }
                            DeckButton::TouchWheelScratch => {
                                (Sensor::DeckATouchWheelScratchButton, input)
                            }
                        }
                    }
                    DeckInput::LayerButton { ctrl, layer, input } => {
                        let input = input.into();
                        match (ctrl, layer) {
                            (DeckLayerButton::Cue, Layer::Plain) => (Sensor::DeckACueButton, input),
                            (DeckLayerButton::Cue, Layer::Shift) => {
                                (Sensor::DeckACueShiftButton, input)
                            }
                            (DeckLayerButton::PlayPause, Layer::Plain) => {
                                (Sensor::DeckAPlayPauseButton, input)
                            }
                            (DeckLayerButton::PlayPause, Layer::Shift) => {
                                (Sensor::DeckAPlayPauseShiftButton, input)
                            }
                            (DeckLayerButton::Sync, Layer::Plain) => {
                                (Sensor::DeckASyncButton, input)
                            }
                            (DeckLayerButton::Sync, Layer::Shift) => {
                                (Sensor::DeckASyncShiftButton, input)
                            }
                        }
                    }
                    DeckInput::Slider { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckSlider::LevelFader => (Sensor::DeckALevelFaderSlider, input),
                            DeckSlider::TouchStrip => (Sensor::DeckATouchStripSlider, input),
                        }
                    }
                    DeckInput::CenterSlider { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckCenterSlider::GainKnob => {
                                (Sensor::DeckAGainKnobCenterSlider, input)
                            }
                            DeckCenterSlider::HiEqKnob => {
                                (Sensor::DeckAHiEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::LoEqKnob => {
                                (Sensor::DeckALoEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::MidEqKnob => {
                                (Sensor::DeckAMidEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::PitchFader => {
                                (Sensor::DeckAPitchFaderCenterSlider, input)
                            }
                        }
                    }
                    DeckInput::SliderEncoder { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckSliderEncoder::TouchWheelBend => {
                                (Sensor::DeckATouchWheelBendSliderEncoder, input)
                            }
                            DeckSliderEncoder::TouchWheelScratch => {
                                (Sensor::DeckATouchWheelScratchSliderEncoder, input)
                            }
                            DeckSliderEncoder::TouchWheelSearch => {
                                (Sensor::DeckATouchWheelSearchSliderEncoder, input)
                            }
                        }
                    }
                },
                Deck::B => match input {
                    DeckInput::Button { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckButton::Fx => (Sensor::DeckBFxButton, input),
                            DeckButton::Load => (Sensor::DeckBLoadButton, input),
                            DeckButton::Monitor => (Sensor::DeckBMonitorButton, input),
                            DeckButton::Shift => (Sensor::DeckBShiftButton, input),
                            DeckButton::TouchStripCenter => {
                                (Sensor::DeckBTouchStripCenterButton, input)
                            }
                            DeckButton::TouchStripHotCueCenter => {
                                (Sensor::DeckBTouchStripHotCueCenterButton, input)
                            }
                            DeckButton::TouchStripHotCueLeft => {
                                (Sensor::DeckBTouchStripHotCueLeftButton, input)
                            }
                            DeckButton::TouchStripHotCueRight => {
                                (Sensor::DeckBTouchStripHotCueRightButton, input)
                            }
                            DeckButton::TouchStripLeft => {
                                (Sensor::DeckBTouchStripLeftButton, input)
                            }
                            DeckButton::TouchStripLoopCenter => {
                                (Sensor::DeckBTouchStripLoopCenterButton, input)
                            }
                            DeckButton::TouchStripLoopLeft => {
                                (Sensor::DeckBTouchStripLoopLeftButton, input)
                            }
                            DeckButton::TouchStripLoopRight => {
                                (Sensor::DeckBTouchStripLoopRightButton, input)
                            }
                            DeckButton::TouchStripRight => {
                                (Sensor::DeckBTouchStripRightButton, input)
                            }
                            DeckButton::TouchWheelScratch => {
                                (Sensor::DeckBTouchWheelScratchButton, input)
                            }
                        }
                    }
                    DeckInput::LayerButton { ctrl, layer, input } => {
                        let input = input.into();
                        match (ctrl, layer) {
                            (DeckLayerButton::Cue, Layer::Plain) => (Sensor::DeckBCueButton, input),
                            (DeckLayerButton::Cue, Layer::Shift) => {
                                (Sensor::DeckBCueShiftButton, input)
                            }
                            (DeckLayerButton::PlayPause, Layer::Plain) => {
                                (Sensor::DeckBPlayPauseButton, input)
                            }
                            (DeckLayerButton::PlayPause, Layer::Shift) => {
                                (Sensor::DeckBPlayPauseShiftButton, input)
                            }
                            (DeckLayerButton::Sync, Layer::Plain) => {
                                (Sensor::DeckBSyncButton, input)
                            }
                            (DeckLayerButton::Sync, Layer::Shift) => {
                                (Sensor::DeckBSyncShiftButton, input)
                            }
                        }
                    }
                    DeckInput::Slider { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckSlider::LevelFader => (Sensor::DeckBLevelFaderSlider, input),
                            DeckSlider::TouchStrip => (Sensor::DeckBTouchStripSlider, input),
                        }
                    }
                    DeckInput::CenterSlider { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckCenterSlider::GainKnob => {
                                (Sensor::DeckBGainKnobCenterSlider, input)
                            }
                            DeckCenterSlider::HiEqKnob => {
                                (Sensor::DeckBHiEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::LoEqKnob => {
                                (Sensor::DeckBLoEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::MidEqKnob => {
                                (Sensor::DeckBMidEqKnobCenterSlider, input)
                            }
                            DeckCenterSlider::PitchFader => {
                                (Sensor::DeckBPitchFaderCenterSlider, input)
                            }
                        }
                    }
                    DeckInput::SliderEncoder { ctrl, input } => {
                        let input = input.into();
                        match ctrl {
                            DeckSliderEncoder::TouchWheelBend => {
                                (Sensor::DeckBTouchWheelBendSliderEncoder, input)
                            }
                            DeckSliderEncoder::TouchWheelScratch => {
                                (Sensor::DeckBTouchWheelScratchSliderEncoder, input)
                            }
                            DeckSliderEncoder::TouchWheelSearch => {
                                (Sensor::DeckBTouchWheelSearchSliderEncoder, input)
                            }
                        }
                    }
                },
            },
        };
        Self {
            index: ctrl.into(),
            input,
        }
    }
}
