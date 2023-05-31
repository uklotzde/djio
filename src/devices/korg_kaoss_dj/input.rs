// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use super::Deck;
use crate::{
    input::TimeStamp,
    midi::{DeviceDescriptor, InputHandler},
    ButtonInput, CenterSliderInput, EmitInputEvent, SliderEncoderInput, SliderInput,
    StepEncoderInput,
};

pub const DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    vendor_name: "Korg",
    model_name: "KAOSS DJ",
    port_name_prefix: "KAOSS DJ",
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
    Tap,
    TapHold,
    TouchPadMode, // 0: X/Y Sliders, 1: 4 Buttons
    TouchPadUpperLeft,
    TouchPadUpperRight,
    TouchPadLowerLeft,
    TouchPadLowerRight,
    BrowseKnobShifted, // Encoder knob acts like a button when shifted
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
    Shifted,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckButton {
    Load,
    Shift,
    Monitor,
    Fx,
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
pub enum DeckButtonLayered {
    PlayPause,
    Sync,
    Cue,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckSlider {
    LevelFader,
    TouchStrip,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckSliderEncoder {
    TouchWheelBend,
    TouchWheelScratch,
    TouchWheelSearch,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckCenterSlider {
    PitchFader,
    GainKnob,
    LoEqKnob,
    MidEqKnob,
    HiEqKnob,
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
    ButtonLayered {
        layer: Layer,
        ctrl: DeckButtonLayered,
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
                        ctrl: Button::BrowseKnobShifted,
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
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Plain,
                            ctrl: DeckButtonLayered::PlayPause,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1d => Self::Deck {
                        deck,
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Plain,
                            ctrl: DeckButtonLayered::Sync,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x1e => Self::Deck {
                        deck,
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Plain,
                            ctrl: DeckButtonLayered::Cue,
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
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Shifted,
                            ctrl: DeckButtonLayered::PlayPause,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x2f => Self::Deck {
                        deck,
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Shifted,
                            ctrl: DeckButtonLayered::Sync,
                            input: u7_to_button(*data2),
                        },
                    },
                    0x30 => Self::Deck {
                        deck,
                        input: DeckInput::ButtonLayered {
                            layer: Layer::Shifted,
                            ctrl: DeckButtonLayered::Cue,
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

impl<E> InputHandler for InputGateway<E>
where
    E: EmitInputEvent<Input> + Send,
{
    fn connect_midi_input_port(
        &mut self,
        device_name: &str,
        port_name: &str,
        _port: &midir::MidiInputPort,
    ) {
        log::debug!("Device \"{device_name}\" is connected to port \"{port_name}\"");
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
