// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr};

use super::{Deck, Side};
use crate::{
    devices::denon_dj_mc6000mk2::{
        MIDI_CMD_CC, MIDI_CMD_NOTE_OFF, MIDI_CMD_NOTE_ON, MIDI_DECK_CUE_BUTTON,
        MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_SYNC_BUTTON,
    },
    u7_be_to_u14, ButtonInput, CenterSliderInput, ControlIndex, ControlInputEvent, ControlRegister,
    EmitInputEvent, MidiDeviceDescriptor, MidiInputReceiver, MidirInputConnector,
    SliderEncoderInput, SliderInput, StepEncoderInput, TimeStamp,
};

fn midi_status_to_deck_cmd(status: u8) -> (Deck, u8) {
    let cmd = status & 0xf;
    let deck = match status & 0x3 {
        0x0 => Deck::One,
        0x1 => Deck::Three,
        0x2 => Deck::Two,
        0x3 => Deck::Four,
        _ => unreachable!(),
    };
    (deck, cmd)
}

// Unused
// fn deck_cmd_to_midi_status(deck: Deck, cmd: u8) -> u8 {
//     debug_assert_eq!(0x0, cmd & 0x3);
//     let channel = match deck {
//         Deck::One => 0x0,
//         Deck::Three => 0x1,
//         Deck::Two => 0x2,
//         Deck::Four => 0x3,
//     };
//     cmd | channel
// }

fn midi_value_to_button(data2: u8) -> ButtonInput {
    match data2 {
        0x00 => ButtonInput::Released,
        0x40 => ButtonInput::Pressed,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub enum Input {
    Crossfader(CenterSliderInput),
    BrowseKnob(StepEncoderInput),
    Side { side: Side, input: SideInput },
    Deck { deck: Deck, input: DeckInput },
}

#[derive(Debug)]
pub enum SideInput {
    ShiftButton(ButtonInput),
    PitchFader(CenterSliderInput),
    Efx1Knob(SliderInput),
    Efx2Knob(SliderInput),
    Efx3Knob(SliderInput),
}

#[derive(Debug)]
pub enum DeckInput {
    CueButton(ButtonInput),
    PlayPauseButton(ButtonInput),
    SyncButton(ButtonInput),
    LevelFader(SliderInput),
    JogWheelBend(SliderEncoderInput),
    JogWheelScratch(SliderEncoderInput),
    GainKnob(CenterSliderInput),
    EqHiKnob(SliderInput),
    EqLoKnob(SliderInput),
    EqMidKnob(SliderInput),
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

impl<E> MidiInputReceiver for InputGateway<E>
where
    E: EmitInputEvent<Input> + Send,
{
    #[allow(clippy::too_many_lines)]
    fn recv_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        let [status, data1, data2] = *input else {
            log::error!("[{ts}] Unexpected MIDI input message: {input:x?}");
            return;
        };
        let (deck, cmd) = midi_status_to_deck_cmd(status);
        let input = match cmd {
            MIDI_CMD_NOTE_OFF | MIDI_CMD_NOTE_ON => {
                let input = midi_value_to_button(data2);
                debug_assert_eq!(cmd == MIDI_CMD_NOTE_ON, input == ButtonInput::Pressed);
                debug_assert_eq!(cmd == MIDI_CMD_NOTE_OFF, input == ButtonInput::Released);
                match data1 {
                    0x60 | 0x61 => {
                        let side = deck.side();
                        Input::Side {
                            side,
                            input: SideInput::ShiftButton(input),
                        }
                    }
                    MIDI_DECK_CUE_BUTTON => Input::Deck {
                        deck,
                        input: DeckInput::CueButton(input),
                    },
                    MIDI_DECK_PLAYPAUSE_BUTTON => Input::Deck {
                        deck,
                        input: DeckInput::PlayPauseButton(input),
                    },
                    MIDI_DECK_SYNC_BUTTON => Input::Deck {
                        deck,
                        input: DeckInput::SyncButton(input),
                    },
                    _ => {
                        log::error!("[{ts}] Unhandled MIDI input message: {input:x?}");
                        return;
                    }
                }
            }
            MIDI_CMD_CC => match data1 {
                0x01 | 0x07 | 0x0c | 0x11 => {
                    let input = CenterSliderInput::from_u7(data2);
                    Input::Deck {
                        deck,
                        input: DeckInput::GainKnob(input),
                    }
                }
                0x02 | 0x08 | 0x0d | 0x12 => {
                    let input = SliderInput::from_u7(data2);
                    Input::Deck {
                        deck,
                        input: DeckInput::EqHiKnob(input),
                    }
                }
                0x03 | 0x09 | 0x0e | 0x13 => {
                    let input = SliderInput::from_u7(data2);
                    Input::Deck {
                        deck,
                        input: DeckInput::EqMidKnob(input),
                    }
                }
                0x04 | 0x0a | 0x0f | 0x14 => {
                    let input = SliderInput::from_u7(data2);
                    Input::Deck {
                        deck,
                        input: DeckInput::EqLoKnob(input),
                    }
                }
                0x05 | 0x0b | 0x10 | 0x15 => {
                    let input = SliderInput::from_u7(data2);
                    Input::Deck {
                        deck,
                        input: DeckInput::LevelFader(input),
                    }
                }
                0x16 | 0x17 => {
                    let input = CenterSliderInput::from_u7(data2);
                    Input::Crossfader(input)
                }
                0x51 => {
                    let input = SliderEncoderInput::from_u7(data2).inverse();
                    Input::Deck {
                        deck,
                        input: DeckInput::JogWheelBend(input),
                    }
                }
                0x52 => {
                    let input = SliderEncoderInput::from_u7(data2).inverse();
                    Input::Deck {
                        deck,
                        input: DeckInput::JogWheelScratch(input),
                    }
                }
                0x54 => {
                    let input = StepEncoderInput::from_u7(data2);
                    Input::BrowseKnob(input)
                }
                0x55 => {
                    let side = deck.side();
                    let input = SliderInput::from_u7(data2);
                    Input::Side {
                        side,
                        input: SideInput::Efx1Knob(input),
                    }
                }
                0x56 => {
                    let side = deck.side();
                    let input = SliderInput::from_u7(data2);
                    Input::Side {
                        side,
                        input: SideInput::Efx2Knob(input),
                    }
                }
                0x57 => {
                    let side = deck.side();
                    let input = SliderInput::from_u7(data2);
                    Input::Side {
                        side,
                        input: SideInput::Efx3Knob(input),
                    }
                }
                _ => {
                    log::error!("[{ts}] Unhandled MIDI input message: {input:x?}");
                    return;
                }
            },
            0xe0 => {
                let input = CenterSliderInput::from_u14(u7_be_to_u14(data2, data1)).inverse();
                Input::Side {
                    side: deck.side(),
                    input: SideInput::PitchFader(input),
                }
            }
            _ => {
                log::error!("[{ts}] Unhandled MIDI input message: {input:x?}");
                return;
            }
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
        unimplemented!("TODO: Convert input {from:?} into ControlRegister")
        // let (sensor, value) = match from {
        //     // TODO
        // };
        // Self {
        //     index: sensor.into(),
        //     value,
        // }
    }
}
