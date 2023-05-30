// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{
    input::{EmitEvent, TimeStamp},
    midi::{DeviceDescriptor, InputHandler},
    ButtonInput,
};

pub const DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    vendor_name: "Pioneer",
    model_name: "DDJ-400",
    port_name_prefix: "DDJ-400",
};

/// One half of a 14-bit value.
///
/// TODO: Combine 14-bit values from two 7-bit values in `Gateway`
/// and remove `pub`.
#[derive(Debug, Clone, Copy)]
pub enum HalfU14 {
    Hi(u8),
    Lo(u8),
}

#[derive(Debug)]
pub enum Input {
    Deck(Deck, DeckInput),
    Mixer(MixerInput),
}

#[derive(Debug, Clone, Copy)]
pub enum Deck {
    Left,
    Right,
}

#[derive(Debug)]
pub enum DeckInput {
    Button {
        ctrl: DeckButton,
        input: ButtonInput,
    },
    PitchFader(HalfU14),
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
pub enum MixerInput {
    Crossfader(HalfU14),
    VolumeFader(MixerChannel, HalfU14),
}

#[derive(Debug, Clone, Copy)]
pub enum MixerChannel {
    Left,
    Right,
}

fn u7_to_button(input: u8) -> ButtonInput {
    match input {
        0 => ButtonInput::Released,
        127 => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

impl Input {
    #[must_use]
    pub fn try_from_midi_message(input: &[u8]) -> Option<Self> {
        let mapped = match input {
            // Play/Pause
            [0x90, 0xB, state] => Self::Deck(
                Deck::Left,
                DeckInput::Button {
                    ctrl: DeckButton::PlayPause,
                    input: u7_to_button(*state),
                },
            ),
            [0x91, 0xB, state] => Self::Deck(
                Deck::Right,
                DeckInput::Button {
                    ctrl: DeckButton::PlayPause,
                    input: u7_to_button(*state),
                },
            ),

            // CUE
            [0x90, 0xC, state] => Self::Deck(
                Deck::Left,
                DeckInput::Button {
                    ctrl: DeckButton::Cue,
                    input: u7_to_button(*state),
                },
            ),
            [0x91, 0xC, state] => Self::Deck(
                Deck::Right,
                DeckInput::Button {
                    ctrl: DeckButton::Cue,
                    input: u7_to_button(*state),
                },
            ),

            // Cross fader
            [0xB6, 0x3F, value] => Self::Mixer(MixerInput::Crossfader(HalfU14::Lo(*value))),
            [0xB6, 0x1F, value] => Self::Mixer(MixerInput::Crossfader(HalfU14::Hi(*value))),

            // Volume faders
            [0xB0, 0x33, value] => Self::Mixer(MixerInput::VolumeFader(
                MixerChannel::Left,
                HalfU14::Lo(*value),
            )),
            [0xB0, 0x13, value] => Self::Mixer(MixerInput::VolumeFader(
                MixerChannel::Left,
                HalfU14::Hi(*value),
            )),
            [0xB1, 0x33, value] => Self::Mixer(MixerInput::VolumeFader(
                MixerChannel::Right,
                HalfU14::Lo(*value),
            )),
            [0xB1, 0x13, value] => Self::Mixer(MixerInput::VolumeFader(
                MixerChannel::Right,
                HalfU14::Hi(*value),
            )),

            // Pitch faders
            [0xB0, 0x00, value] => {
                Self::Deck(Deck::Left, DeckInput::PitchFader(HalfU14::Hi(*value)))
            }
            [0xB0, 0x20, value] => {
                Self::Deck(Deck::Left, DeckInput::PitchFader(HalfU14::Lo(*value)))
            }
            [0xB1, 0x00, value] => {
                Self::Deck(Deck::Right, DeckInput::PitchFader(HalfU14::Hi(*value)))
            }
            [0xB1, 0x20, value] => {
                Self::Deck(Deck::Right, DeckInput::PitchFader(HalfU14::Lo(*value)))
            }

            // Jog wheel
            [0xB1, 0x21, 0x3F] => Self::Deck(Deck::Right, DeckInput::JogWheel(WheelDirection::Rev)),
            [0xB1, 0x21, 0x41] => Self::Deck(Deck::Right, DeckInput::JogWheel(WheelDirection::Fwd)),
            [0xB0, 0x21, 0x3F] => Self::Deck(Deck::Left, DeckInput::JogWheel(WheelDirection::Rev)),
            [0xB0, 0x21, 0x41] => Self::Deck(Deck::Left, DeckInput::JogWheel(WheelDirection::Fwd)),

            _ => {
                return None;
            }
        };
        Some(mapped)
    }
}

pub type InputEvent = crate::input::Event<Input>;

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
    E: EmitEvent<Input> + Send,
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
