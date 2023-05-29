// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::input;

#[derive(Debug)]
pub enum InputEvent {
    Deck(Deck, DeckInputEvent),
    Mixer(MixerInputEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    Left,
    Right,
}

#[derive(Debug)]
pub enum DeckInputEvent {
    Button {
        ctrl: DeckButton,
        input: input::Button,
    },
    PitchFader(input::HalfU14),
    JogWheel(WheelDirection),
}

#[derive(Debug, Clone, Copy)]
pub enum DeckButton {
    PlayPause,
    Cue,
}

#[derive(Debug, Clone, Copy)]
pub enum WheelDirection {
    Rev,
    Fwd,
}

#[derive(Debug)]
pub enum MixerInputEvent {
    Crossfader(input::HalfU14),
    VolumeFader(MixerChannel, input::HalfU14),
}

#[derive(Debug, Clone, Copy)]
pub enum MixerChannel {
    Left,
    Right,
}

fn u7_to_button(input: u8) -> input::Button {
    match input {
        0 => input::Button::Released,
        127 => input::Button::Pressed,
        _ => unreachable!(),
    }
}

impl InputEvent {
    #[must_use]
    pub fn try_from_midi_message(input: &[u8]) -> Option<Self> {
        let mapped = match input {
            // Play/Pause
            [0x90, 0xB, state] => Self::Deck(
                Deck::Left,
                DeckInputEvent::Button {
                    ctrl: DeckButton::PlayPause,
                    input: u7_to_button(*state),
                },
            ),
            [0x91, 0xB, state] => Self::Deck(
                Deck::Right,
                DeckInputEvent::Button {
                    ctrl: DeckButton::PlayPause,
                    input: u7_to_button(*state),
                },
            ),

            // CUE
            [0x90, 0xC, state] => Self::Deck(
                Deck::Left,
                DeckInputEvent::Button {
                    ctrl: DeckButton::Cue,
                    input: u7_to_button(*state),
                },
            ),
            [0x91, 0xC, state] => Self::Deck(
                Deck::Right,
                DeckInputEvent::Button {
                    ctrl: DeckButton::Cue,
                    input: u7_to_button(*state),
                },
            ),

            // Cross fader
            [0xB6, 0x3F, value] => {
                Self::Mixer(MixerInputEvent::Crossfader(input::HalfU14::Lo(*value)))
            }
            [0xB6, 0x1F, value] => {
                Self::Mixer(MixerInputEvent::Crossfader(input::HalfU14::Hi(*value)))
            }

            // Volume faders
            [0xB0, 0x33, value] => Self::Mixer(MixerInputEvent::VolumeFader(
                MixerChannel::Left,
                input::HalfU14::Lo(*value),
            )),
            [0xB0, 0x13, value] => Self::Mixer(MixerInputEvent::VolumeFader(
                MixerChannel::Left,
                input::HalfU14::Hi(*value),
            )),
            [0xB1, 0x33, value] => Self::Mixer(MixerInputEvent::VolumeFader(
                MixerChannel::Right,
                input::HalfU14::Lo(*value),
            )),
            [0xB1, 0x13, value] => Self::Mixer(MixerInputEvent::VolumeFader(
                MixerChannel::Right,
                input::HalfU14::Hi(*value),
            )),

            // Pitch faders
            [0xB0, 0x00, value] => Self::Deck(
                Deck::Left,
                DeckInputEvent::PitchFader(input::HalfU14::Hi(*value)),
            ),
            [0xB0, 0x20, value] => Self::Deck(
                Deck::Left,
                DeckInputEvent::PitchFader(input::HalfU14::Lo(*value)),
            ),
            [0xB1, 0x00, value] => Self::Deck(
                Deck::Right,
                DeckInputEvent::PitchFader(input::HalfU14::Hi(*value)),
            ),
            [0xB1, 0x20, value] => Self::Deck(
                Deck::Right,
                DeckInputEvent::PitchFader(input::HalfU14::Lo(*value)),
            ),

            // Jog wheel
            [0xB1, 0x21, 0x3F] => {
                Self::Deck(Deck::Right, DeckInputEvent::JogWheel(WheelDirection::Rev))
            }
            [0xB1, 0x21, 0x41] => {
                Self::Deck(Deck::Right, DeckInputEvent::JogWheel(WheelDirection::Fwd))
            }
            [0xB0, 0x21, 0x3F] => {
                Self::Deck(Deck::Left, DeckInputEvent::JogWheel(WheelDirection::Rev))
            }
            [0xB0, 0x21, 0x41] => {
                Self::Deck(Deck::Left, DeckInputEvent::JogWheel(WheelDirection::Fwd))
            }

            _ => {
                return None;
            }
        };
        Some(mapped)
    }
}
