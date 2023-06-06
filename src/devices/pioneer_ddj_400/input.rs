// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use super::Deck;
use crate::{
    u7_be_to_u14, ButtonInput, CenterSliderInput, EmitInputEvent, MidiDeviceDescriptor,
    MidiInputReceiver, MidirInputConnector, SliderInput, TimeStamp,
};

pub type InputEvent = crate::InputEvent<Input>;

/// One half of a 14-bit value.
///
/// TODO: Combine 14-bit values from two 7-bit values in `InputGateway`
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

#[derive(Debug)]
pub enum DeckInput {
    Button {
        ctrl: DeckButton,
        input: ButtonInput,
    },
    PitchFader(CenterSliderInput),
    PitchFaderRaw(HalfU14),
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
    Crossfader(CenterSliderInput),
    CrossfaderRaw(HalfU14),
    VolumeFader(MixerChannel, SliderInput),
    VolumeFaderRaw(MixerChannel, HalfU14),
}

#[derive(Debug, Clone, Copy)]
pub enum MixerChannel {
    /// It's labeled with `1` on the controller
    One,
    /// It's labeled with `2` on the controller
    Two,
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
            [0xB6, 0x3F, value] => Self::Mixer(MixerInput::CrossfaderRaw(HalfU14::Lo(*value))),
            [0xB6, 0x1F, value] => Self::Mixer(MixerInput::CrossfaderRaw(HalfU14::Hi(*value))),

            // Volume faders
            [0xB0, 0x33, value] => Self::Mixer(MixerInput::VolumeFaderRaw(
                MixerChannel::One,
                HalfU14::Lo(*value),
            )),
            [0xB0, 0x13, value] => Self::Mixer(MixerInput::VolumeFaderRaw(
                MixerChannel::One,
                HalfU14::Hi(*value),
            )),
            [0xB1, 0x33, value] => Self::Mixer(MixerInput::VolumeFaderRaw(
                MixerChannel::Two,
                HalfU14::Lo(*value),
            )),
            [0xB1, 0x13, value] => Self::Mixer(MixerInput::VolumeFaderRaw(
                MixerChannel::Two,
                HalfU14::Hi(*value),
            )),

            // Pitch faders
            [0xB0, 0x00, value] => {
                Self::Deck(Deck::Left, DeckInput::PitchFaderRaw(HalfU14::Hi(*value)))
            }
            [0xB0, 0x20, value] => {
                Self::Deck(Deck::Left, DeckInput::PitchFaderRaw(HalfU14::Lo(*value)))
            }
            [0xB1, 0x00, value] => {
                Self::Deck(Deck::Right, DeckInput::PitchFaderRaw(HalfU14::Hi(*value)))
            }
            [0xB1, 0x20, value] => {
                Self::Deck(Deck::Right, DeckInput::PitchFaderRaw(HalfU14::Lo(*value)))
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

#[derive(Debug)]
pub struct InputGateway<E> {
    crossfader: Fader,
    volume_fader_ch_1: Fader,
    volume_fader_ch_2: Fader,
    pitch_fader_left: Fader,
    pitch_fader_right: Fader,
    emit_input_event: E,
}

#[derive(Debug, Default)]
struct Fader {
    hi: u8,
    lo: u8,
}

impl<E> InputGateway<E> {
    #[must_use]
    pub fn attach(emit_input_event: E) -> Self {
        Self {
            emit_input_event,
            crossfader: Default::default(),
            volume_fader_ch_1: Default::default(),
            volume_fader_ch_2: Default::default(),
            pitch_fader_left: Default::default(),
            pitch_fader_right: Default::default(),
        }
    }

    #[must_use]
    pub fn detach(self) -> E {
        let Self {
            emit_input_event, ..
        } = self;
        emit_input_event
    }
}

impl<E> MidiInputReceiver for InputGateway<E>
where
    E: EmitInputEvent<Input> + Send,
{
    fn recv_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        let Some(input) = Input::try_from_midi_message(input) else {
            log::debug!("[{ts}] Unhandled MIDI input message: {input:x?}");
            return;
        };
        let input = match input {
            Input::Mixer(ev) => match ev {
                MixerInput::CrossfaderRaw(half_u14) => {
                    match half_u14 {
                        HalfU14::Lo(val) => {
                            self.crossfader.lo = val;
                        }
                        HalfU14::Hi(val) => {
                            self.crossfader.hi = val;
                        }
                    }
                    let center_slider = CenterSliderInput::from_u14(u7_be_to_u14(
                        self.crossfader.hi,
                        self.crossfader.lo,
                    ));
                    Input::Mixer(MixerInput::Crossfader(center_slider))
                }
                MixerInput::VolumeFaderRaw(channel, half_u14) => {
                    let fader = match channel {
                        MixerChannel::One => &mut self.volume_fader_ch_1,
                        MixerChannel::Two => &mut self.volume_fader_ch_2,
                    };
                    match half_u14 {
                        HalfU14::Lo(val) => {
                            fader.lo = val;
                        }
                        HalfU14::Hi(val) => {
                            fader.hi = val;
                        }
                    }
                    let slider = SliderInput::from_u14(u7_be_to_u14(fader.hi, fader.lo));
                    Input::Mixer(MixerInput::VolumeFader(channel, slider))
                }
                _ => Input::Mixer(ev),
            },
            Input::Deck(deck, input) => match input {
                DeckInput::PitchFaderRaw(half_u14) => {
                    let fader = match deck {
                        Deck::Left => &mut self.pitch_fader_left,
                        Deck::Right => &mut self.pitch_fader_right,
                    };
                    match half_u14 {
                        HalfU14::Lo(val) => {
                            fader.lo = val;
                        }
                        HalfU14::Hi(val) => {
                            fader.hi = val;
                        }
                    }
                    let slider = CenterSliderInput::from_u14(u7_be_to_u14(fader.hi, fader.lo));
                    Input::Deck(deck, DeckInput::PitchFader(slider))
                }
                _ => Input::Deck(deck, input),
            },
        };
        let event = InputEvent { ts, input };
        log::debug!("Emitting {event:?}");
        self.emit_input_event.emit_input_event(event);
    }
}

impl<E> MidirInputConnector for InputGateway<E>
where
    E: Send,
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
}
