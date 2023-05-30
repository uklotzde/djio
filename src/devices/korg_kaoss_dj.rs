// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{
    input::{self, TimeStamp},
    midi::{DeviceDescriptor, InputHandler},
};

pub const DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    vendor_name: "Korg",
    model_name: "KAOSS DJ",
    port_name_prefix: "KAOSS DJ",
};

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Tap,
    TapHold,
    TouchPadMode, // 0: X/Y Sliders, 1: 4 Buttons
    TouchPadUpperLeft,
    TouchPadUpperRight,
    TouchPadLowerLeft,
    TouchPadLowerRight,
    BrowseEncoderShifted,
}

#[derive(Debug, Clone, Copy)]
pub enum CenterSlider {
    CrossFader,
}

#[derive(Debug, Clone, Copy)]
pub enum StepEncoder {
    BrowseKnob,
    BrowseKnobShifted,
    ProgramKnob,
}

#[derive(Debug, Clone, Copy)]
pub enum Slider {
    TouchPadX,
    TouchPadY,
}

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    /// Left deck
    A,
    /// Right deck
    B,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Layer {
    #[default]
    Default,
    Shifted,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckButton {
    Load,
    Shift,
    PlayPause,
    Sync,
    Cue,
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
pub enum DeckSlider {
    LineFader,
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
        layer: Layer,
        input: input::Button,
    },
    Slider {
        ctrl: Slider,
        input: input::Slider,
    },
    CenterSlider {
        ctrl: CenterSlider,
        input: input::CenterSlider,
    },
    StepEncoder {
        ctrl: StepEncoder,
        input: input::StepEncoder,
    },
    Deck(Deck, DeckInput),
}

#[derive(Debug)]
pub enum DeckInput {
    Button {
        ctrl: DeckButton,
        layer: Layer,
        input: input::Button,
    },
    Slider {
        ctrl: DeckSlider,
        input: input::Slider,
    },
    CenterSlider {
        ctrl: DeckCenterSlider,
        input: input::CenterSlider,
    },
    SliderEncoder {
        ctrl: DeckSliderEncoder,
        input: input::SliderEncoder,
    },
}

fn u7_to_button(input: u8) -> input::Button {
    match input {
        0 => input::Button::Released,
        127 => input::Button::Pressed,
        _ => unreachable!(),
    }
}

fn u7_to_step_encoder(input: u8) -> input::StepEncoder {
    let delta = match input {
        1 => 1,
        127 => -1,
        _ => unreachable!(),
    };
    input::StepEncoder { delta }
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
                        ctrl: Button::BrowseEncoderShifted,
                        layer: Layer::Shifted,
                        input: u7_to_button(*data2),
                    },
                    0x0b => Input::Button {
                        ctrl: Button::Tap,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x21 => Input::Button {
                        ctrl: Button::TapHold,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x22 => Input::Button {
                        ctrl: Button::TouchPadMode,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x4a => Input::Button {
                        ctrl: Button::TouchPadUpperLeft,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x4b => Input::Button {
                        ctrl: Button::TouchPadUpperRight,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x4c => Input::Button {
                        ctrl: Button::TouchPadLowerLeft,
                        layer: Layer::Default,
                        input: u7_to_button(*data2),
                    },
                    0x4d => Input::Button {
                        ctrl: Button::TouchPadLowerRight,
                        layer: Layer::Default,
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
                    0x0e => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Load,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x0f => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x10 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x11 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripLoopRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x12 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x13 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x14 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripHotCueRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x15 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x16 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x17 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchStripRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x18 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Fx,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x19 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Monitor,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1a => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Shift,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1b => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::PlayPause,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1d => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Sync,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1e => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Cue,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1f => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::TouchWheelScratch,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x2e => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::PlayPause,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x2f => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Sync,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x30 => Self::Deck(
                        deck,
                        DeckInput::Button {
                            ctrl: DeckButton::Cue,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
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
                    input: input::u7_to_slider(*data2),
                }
            }
            [0xb6 | 0xb7 | 0xb8, 0x0d, data2] => {
                // See the comment above for the X slider.
                Self::Slider {
                    ctrl: Slider::TouchPadY,
                    input: input::u7_to_slider(*data2),
                }
            }
            [0xb6, data1, data2] => {
                // Global sliders and encoders (MIDI channel 7)
                match *data1 {
                    0x17 => Self::CenterSlider {
                        ctrl: CenterSlider::CrossFader,
                        input: input::u7_to_center_slider(*data2),
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
                    0x0e => Self::Deck(
                        deck,
                        DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelBend,
                            input: input::u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x0f => Self::Deck(
                        deck,
                        DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelSearch,
                            input: input::u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x10 => Self::Deck(
                        deck,
                        DeckInput::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelScratch,
                            input: input::u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x18 => Self::Deck(
                        deck,
                        DeckInput::Slider {
                            ctrl: DeckSlider::LineFader,
                            input: input::u7_to_slider(*data2),
                        },
                    ),
                    0x19 => Self::Deck(
                        deck,
                        DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::PitchFader,
                            input: input::u7_to_center_slider(*data2),
                        },
                    ),
                    0x1a => Self::Deck(
                        deck,
                        DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::GainKnob,
                            input: input::u7_to_center_slider(*data2),
                        },
                    ),
                    0x1b => Self::Deck(
                        deck,
                        DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::HiEqKnob,
                            input: input::u7_to_center_slider(*data2),
                        },
                    ),
                    0x1c => Self::Deck(
                        deck,
                        DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::MidEqKnob,
                            input: input::u7_to_center_slider(*data2),
                        },
                    ),
                    0x1d => Self::Deck(
                        deck,
                        DeckInput::CenterSlider {
                            ctrl: DeckCenterSlider::LoEqKnob,
                            input: input::u7_to_center_slider(*data2),
                        },
                    ),
                    0x21 => Self::Deck(
                        deck,
                        DeckInput::Slider {
                            ctrl: DeckSlider::TouchStrip,
                            input: input::u7_to_slider(*data2),
                        },
                    ),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        Some(mapped)
    }
}

pub type InputEvent = input::Event<Input>;

#[derive(Debug)]
pub struct Gateway<E> {
    emit_input_event: E,
}

impl<E> Gateway<E> {
    pub fn new(emit_input_event: E) -> Self {
        Self { emit_input_event }
    }
}

impl<E> InputHandler for Gateway<E>
where
    E: input::EmitEvent<Input> + Send,
{
    fn connect_midi_input_port(
        &mut self,
        _device_name: &str,
        _port_name: &str,
        _port: &midir::MidiInputPort,
    ) {
    }

    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        let Some(input) = Input::try_from_midi_message(input) else {
            // Silently ignore received MIDI message
            return;
        };
        let event = InputEvent { ts, input };
        self.emit_input_event.emit_event(event);
    }
}
