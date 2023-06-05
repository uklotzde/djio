// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive as _;

use super::{
    Deck, MIDI_DECK_CUE_BUTTON, MIDI_DECK_EQ_HI_KNOB, MIDI_DECK_EQ_LO_KNOB, MIDI_DECK_EQ_MID_KNOB,
    MIDI_DECK_FX_BUTTON, MIDI_DECK_GAIN_KNOB, MIDI_DECK_MONITOR_BUTTON, MIDI_DECK_PLAYPAUSE_BUTTON,
    MIDI_DECK_SHIFT_BUTTON, MIDI_DECK_SYNC_BUTTON, MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
    MIDI_MASTER_LEVEL_KNOB, MIDI_MONITOR_LEVEL_KNOB, MIDI_MONITOR_MIX_KNOB, MIDI_STATUS_BUTTON,
    MIDI_STATUS_BUTTON_DECK_A, MIDI_STATUS_BUTTON_DECK_B, MIDI_STATUS_CC, MIDI_STATUS_CC_DECK_A,
    MIDI_STATUS_CC_DECK_B, MIDI_TAP_BUTTON,
};
use crate::{ControlIndex, LedOutput, OutputResult};

const LED_OFF: u8 = 0x00;
const LED_ON: u8 = 0x7f;

fn led_to_u7(output: LedOutput) -> u8 {
    match output {
        LedOutput::Off => LED_OFF,
        LedOutput::On => LED_ON,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonLed {
    Tab,
}

#[derive(Debug, Clone, Copy)]
pub enum KnobLed {
    MonitorLevel,
    MonitorMix,
    MasterLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckButtonLed {
    Shift,
    PlayPause,
    Sync,
    Cue,
    Monitor,
    Fx,
    TouchStripLeft,
    TouchStripCenter,
    TouchStripRight,
    TouchStripLoopLeft,
    TouchStripLoopCenter,
    TouchStripLoopRight,
    TouchStripHotCueLeft,
    TouchStripHotCueCenter,
    TouchStripHotCueRight,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckKnobLed {
    Gain,
    LoEq,
    MidEq,
    HiEq,
}

#[derive(Debug, Clone, Copy)]
pub enum Led {
    Button(ButtonLed),
    Knob(KnobLed),
    Deck(Deck, DeckLed),
}

impl From<ButtonLed> for Led {
    fn from(from: ButtonLed) -> Self {
        Self::Button(from)
    }
}

impl From<KnobLed> for Led {
    fn from(from: KnobLed) -> Self {
        Self::Knob(from)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeckLed {
    Button(DeckButtonLed),
    Knob(DeckKnobLed),
}

impl From<DeckButtonLed> for DeckLed {
    fn from(from: DeckButtonLed) -> Self {
        Self::Button(from)
    }
}

impl From<DeckKnobLed> for DeckLed {
    fn from(from: DeckKnobLed) -> Self {
        Self::Knob(from)
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum Actuator {
    // Led
    MonitorLevelKnobLed,
    MonitorMixKnobLed,
    MasterLevelKnobLed,
    TabButtonLed,
    TabHoldButtonLed,
    HoldButtonLed,
    // Deck A: Led
    DeckAShiftButtonLed,
    DeckAPlayPauseButtonLed,
    DeckASyncButtonLed,
    DeckACueButtonLed,
    DeckAMonitorButtonLed,
    DeckAFxButtonLed,
    DeckATouchStripLeftLed,
    DeckATouchStripCenterLed,
    DeckATouchStripRightLed,
    DeckATouchStripLoopLeftLed,
    DeckATouchStripLoopCenterLed,
    DeckATouchStripLoopRightLed,
    DeckATouchStripHotCueLeftLed,
    DeckATouchStripHotCueCenterLed,
    DeckATouchStripHotCueRightLed,
    DeckAGainKnobLed,
    DeckAEqLoKnobLed,
    DeckAEqMidKnobLed,
    DeckAEqHiKnobLed,
    // Deck B: Led
    DeckBShiftButtonLed,
    DeckBPlayPauseButtonLed,
    DeckBSyncButtonLed,
    DeckBCueButtonLed,
    DeckBMonitorButtonLed,
    DeckBFxButtonLed,
    DeckBTouchStripLeftLed,
    DeckBTouchStripCenterLed,
    DeckBTouchStripRightLed,
    DeckBTouchStripLoopLeftLed,
    DeckBTouchStripLoopCenterLed,
    DeckBTouchStripLoopRightLed,
    DeckBTouchStripHotCueLeftLed,
    DeckBTouchStripHotCueCenterLed,
    DeckBTouchStripHotCueRightLed,
    DeckBGainKnobLed,
    DeckBEqLoKnobLed,
    DeckBEqMidKnobLed,
    DeckBEqHiKnobLed,
}

impl From<Actuator> for ControlIndex {
    fn from(value: Actuator) -> Self {
        ControlIndex::new(value.to_u32().expect("u32"))
    }
}

#[allow(missing_debug_implementations)]
pub struct OutputGateway {
    midi_output_connection: MidiOutputConnection,
}

impl OutputGateway {
    #[must_use]
    pub fn attach(midi_output_connection: MidiOutputConnection) -> Self {
        Self {
            midi_output_connection,
        }
    }

    #[must_use]
    pub fn detach(self) -> MidiOutputConnection {
        let Self {
            midi_output_connection,
        } = self;
        midi_output_connection
    }

    pub fn send_led_output(&mut self, led: Led, output: LedOutput) -> OutputResult<()> {
        let (status, data1) = match led {
            Led::Button(led) => {
                let status = MIDI_STATUS_BUTTON;
                let data1 = match led {
                    ButtonLed::Tab => MIDI_TAP_BUTTON,
                };
                (status, data1)
            }
            Led::Knob(led) => {
                let status = MIDI_STATUS_CC;
                let data1 = match led {
                    KnobLed::MonitorLevel => MIDI_MONITOR_LEVEL_KNOB,
                    KnobLed::MonitorMix => MIDI_MONITOR_MIX_KNOB,
                    KnobLed::MasterLevel => MIDI_MASTER_LEVEL_KNOB,
                };
                (status, data1)
            }
            Led::Deck(deck, led) => match led {
                DeckLed::Button(led) => {
                    let status = match deck {
                        Deck::A => MIDI_STATUS_BUTTON_DECK_A,
                        Deck::B => MIDI_STATUS_BUTTON_DECK_B,
                    };
                    let data1 = match led {
                        DeckButtonLed::Fx => MIDI_DECK_FX_BUTTON,
                        DeckButtonLed::Monitor => MIDI_DECK_MONITOR_BUTTON,
                        DeckButtonLed::Shift => MIDI_DECK_SHIFT_BUTTON,
                        DeckButtonLed::PlayPause => MIDI_DECK_PLAYPAUSE_BUTTON,
                        DeckButtonLed::Sync => MIDI_DECK_SYNC_BUTTON,
                        DeckButtonLed::Cue => MIDI_DECK_CUE_BUTTON,
                        DeckButtonLed::TouchStripCenter => MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
                        DeckButtonLed::TouchStripHotCueCenter => {
                            MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON
                        }
                        DeckButtonLed::TouchStripHotCueLeft => {
                            MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON
                        }
                        DeckButtonLed::TouchStripHotCueRight => {
                            MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON
                        }
                        DeckButtonLed::TouchStripLeft => MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
                        DeckButtonLed::TouchStripLoopCenter => {
                            MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON
                        }
                        DeckButtonLed::TouchStripLoopLeft => MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
                        DeckButtonLed::TouchStripLoopRight => {
                            MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON
                        }
                        DeckButtonLed::TouchStripRight => MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
                    };
                    (status, data1)
                }
                DeckLed::Knob(led) => {
                    let status = match deck {
                        Deck::A => MIDI_STATUS_CC_DECK_A,
                        Deck::B => MIDI_STATUS_CC_DECK_B,
                    };
                    let data1 = match led {
                        DeckKnobLed::Gain => MIDI_DECK_GAIN_KNOB,
                        DeckKnobLed::HiEq => MIDI_DECK_EQ_HI_KNOB,
                        DeckKnobLed::MidEq => MIDI_DECK_EQ_MID_KNOB,
                        DeckKnobLed::LoEq => MIDI_DECK_EQ_LO_KNOB,
                    };
                    (status, data1)
                }
            },
        };
        let data2 = led_to_u7(output);
        self.midi_output_connection.send(&[status, data1, data2])?;
        Ok(())
    }
}
