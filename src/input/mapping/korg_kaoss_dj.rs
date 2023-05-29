// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::input;

use super::{u7_to_center_slider, u7_to_slider};

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Tap,
    Hold,
    TouchPadMode,
}

#[derive(Debug, Clone, Copy)]
pub enum Slider {
    MasterLevelKnob,
    MonitorLevelKnob,
    MonitorMixKnob,
    TouchPadX,
    TouchPadY,
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
    TouchStripMode,
    TouchStripLeft,
    TouchStripCenter,
    TouchStripRight,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckSlider {
    LineFader,
    TouchStrip,
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
    pub fn try_from_midi_message(input: &[u8]) -> Option<Self> {
        let mapped = match input {
            [0x97, 0x1b, data2] => Self::Deck(
                Deck::A,
                DeckInputEvent::Button {
                    ctrl: DeckButton::PlayPause,
                    layer: Layer::Default,
                    input: u7_to_button(*data2),
                },
            ),
            [0x97, 0x2e, data2] => Self::Deck(
                Deck::A,
                DeckInputEvent::Button {
                    ctrl: DeckButton::PlayPause,
                    layer: Layer::Shifted,
                    input: u7_to_button(*data2),
                },
            ),
            [0xb6, 0x17, data2] => Self::CenterSlider {
                ctrl: CenterSlider::CrossFader,
                input: u7_to_center_slider(*data2),
            },
            [0xb6, 0x1e, data2] => Self::StepEncoder {
                ctrl: StepEncoder::BrowseKnob,
                input: u7_to_step_encoder(*data2),
            },
            [0xb6, 0x1f, data2] => Self::StepEncoder {
                ctrl: StepEncoder::ProgramKnob,
                input: u7_to_step_encoder(*data2),
            },
            [0xb7, 0x18, data2] => Self::Deck(
                Deck::A,
                DeckInputEvent::Slider {
                    ctrl: DeckSlider::LineFader,
                    input: u7_to_slider(*data2),
                },
            ),
            [0xb8, 0x18, data2] => Self::Deck(
                Deck::B,
                DeckInputEvent::Slider {
                    ctrl: DeckSlider::LineFader,
                    input: u7_to_slider(*data2),
                },
            ),
            _ => {
                return None;
            }
        };
        Some(mapped)
    }
}
