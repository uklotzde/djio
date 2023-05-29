// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::input;

use super::{u7_to_center_slider, u7_to_slider, u7_to_slider_encoder};

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Tap,
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
pub enum InputEvent {
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
    Deck(Deck, DeckInputEvent),
}

#[derive(Debug)]
pub enum DeckInputEvent {
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

impl InputEvent {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn try_from_midi_message(input: &[u8]) -> Option<Self> {
        let mapped = match input {
            [0x96, 0x07, data2] => InputEvent::Button {
                ctrl: Button::BrowseEncoderShifted,
                layer: Layer::Shifted,
                input: u7_to_button(*data2),
            },
            [0x96, 0x0b, data2] => InputEvent::Button {
                ctrl: Button::Tap,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [0x96, 0x22, data2] => InputEvent::Button {
                ctrl: Button::TouchPadMode,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [0x96, 0x4a, data2] => InputEvent::Button {
                ctrl: Button::TouchPadUpperLeft,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [0x96, 0x4b, data2] => InputEvent::Button {
                ctrl: Button::TouchPadUpperRight,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [0x96, 0x4c, data2] => InputEvent::Button {
                ctrl: Button::TouchPadLowerLeft,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [0x96, 0x4d, data2] => InputEvent::Button {
                ctrl: Button::TouchPadLowerRight,
                layer: Layer::Default,
                input: u7_to_button(*data2),
            },
            [status @ (0x97 | 0x98), data1, data2] => {
                // Deck buttons
                let deck = match *status {
                    0x97 => Deck::A,
                    0x98 => Deck::B,
                    _ => unreachable!(),
                };
                match data1 {
                    0x0e => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Load,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x0f => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripLoopLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x10 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripLoopCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x11 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripLoopRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x12 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripHotCueLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x13 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripHotCueCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x14 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripHotCueRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x15 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripLeft,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x16 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripCenter,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x17 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchStripRight,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x18 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Fx,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x19 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Monitor,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1a => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Shift,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1b => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::PlayPause,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1d => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Sync,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1e => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Cue,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x1f => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::TouchWheelScratch,
                            layer: Layer::Default,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x2e => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::PlayPause,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x2f => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Sync,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
                    0x30 => Self::Deck(
                        deck,
                        DeckInputEvent::Button {
                            ctrl: DeckButton::Cue,
                            layer: Layer::Shifted,
                            input: u7_to_button(*data2),
                        },
                    ),
                    _ => unreachable!(),
                }
            }
            [0xb6, data1, data2] => {
                // Sliders and encoders
                match *data1 {
                    0x17 => Self::CenterSlider {
                        ctrl: CenterSlider::CrossFader,
                        input: u7_to_center_slider(*data2),
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
            [0xb7 | 0xb8, 0x0c, data2] => {
                // The X/Y coordinates of the touchpad are always sent twice for
                // unknown reasons. They are probably intended to be handled
                // independently by both decks? Two messages with the same
                // 7-bit value are received subsequently.
                // TODO: Ignore one of the values. But forwarding them twice is
                // only slightly inefficient, so why bother. The receiver of the
                // event will deduplicate them anyway as needed.
                Self::Slider {
                    ctrl: Slider::TouchPadX,
                    input: u7_to_slider(*data2),
                }
            }
            [0xb7 | 0xb8, 0x0d, data2] => {
                // See the comment above for the X slider
                Self::Slider {
                    ctrl: Slider::TouchPadY,
                    input: u7_to_slider(*data2),
                }
            }
            [status @ (0xb7 | 0xb8), data1, data2] => {
                // Deck sliders and slider encoders
                let deck = match *status {
                    0xb7 => Deck::A,
                    0xb8 => Deck::B,
                    _ => unreachable!(),
                };
                match *data1 {
                    0x0e => Self::Deck(
                        deck,
                        DeckInputEvent::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelBend,
                            input: u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x0f => Self::Deck(
                        deck,
                        DeckInputEvent::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelSearch,
                            input: u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x10 => Self::Deck(
                        deck,
                        DeckInputEvent::SliderEncoder {
                            ctrl: DeckSliderEncoder::TouchWheelScratch,
                            input: u7_to_slider_encoder(*data2),
                        },
                    ),
                    0x18 => Self::Deck(
                        deck,
                        DeckInputEvent::Slider {
                            ctrl: DeckSlider::LineFader,
                            input: u7_to_slider(*data2),
                        },
                    ),
                    0x19 => Self::Deck(
                        deck,
                        DeckInputEvent::CenterSlider {
                            ctrl: DeckCenterSlider::PitchFader,
                            input: u7_to_center_slider(*data2),
                        },
                    ),
                    0x1a => Self::Deck(
                        deck,
                        DeckInputEvent::CenterSlider {
                            ctrl: DeckCenterSlider::GainKnob,
                            input: u7_to_center_slider(*data2),
                        },
                    ),
                    0x1b => Self::Deck(
                        deck,
                        DeckInputEvent::CenterSlider {
                            ctrl: DeckCenterSlider::HiEqKnob,
                            input: u7_to_center_slider(*data2),
                        },
                    ),
                    0x1c => Self::Deck(
                        deck,
                        DeckInputEvent::CenterSlider {
                            ctrl: DeckCenterSlider::MidEqKnob,
                            input: u7_to_center_slider(*data2),
                        },
                    ),
                    0x1d => Self::Deck(
                        deck,
                        DeckInputEvent::CenterSlider {
                            ctrl: DeckCenterSlider::LoEqKnob,
                            input: u7_to_center_slider(*data2),
                        },
                    ),
                    0x21 => Self::Deck(
                        deck,
                        DeckInputEvent::Slider {
                            ctrl: DeckSlider::TouchStrip,
                            input: u7_to_slider(*data2),
                        },
                    ),
                    _ => unreachable!(),
                }
            }
            _ => {
                return None;
            }
        };
        Some(mapped)
    }
}
