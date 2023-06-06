// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;
use strum::{EnumIter, FromRepr, IntoEnumIterator as _};

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
    EqLo,
    EqMid,
    EqHi,
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

#[derive(Debug, Clone, Copy, FromRepr, EnumIter)]
#[repr(u32)]
pub enum Actuator {
    // Button Led
    TabButtonLed,
    // Knob Led
    MonitorLevelKnobLed,
    MonitorMixKnobLed,
    MasterLevelKnobLed,
    // Deck A: Button Led
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
    // Deck A: Knob Led
    DeckAGainKnobLed,
    DeckAEqLoKnobLed,
    DeckAEqMidKnobLed,
    DeckAEqHiKnobLed,
    // Deck B: Button Led
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
    // Deck A: Knob Led
    DeckBGainKnobLed,
    DeckBEqLoKnobLed,
    DeckBEqMidKnobLed,
    DeckBEqHiKnobLed,
}

impl From<Actuator> for ControlIndex {
    fn from(value: Actuator) -> Self {
        ControlIndex::new(value as u32)
    }
}

#[derive(Debug)]
pub struct InvalidControlIndex;

impl TryFrom<ControlIndex> for Actuator {
    type Error = InvalidControlIndex;

    fn try_from(index: ControlIndex) -> Result<Self, Self::Error> {
        Self::from_repr(index.value()).ok_or(InvalidControlIndex)
    }
}

impl From<Actuator> for Led {
    fn from(from: Actuator) -> Self {
        match from {
            Actuator::TabButtonLed => Led::Button(ButtonLed::Tab),
            Actuator::MasterLevelKnobLed => Led::Knob(KnobLed::MasterLevel),
            Actuator::MonitorLevelKnobLed => Led::Knob(KnobLed::MonitorLevel),
            Actuator::MonitorMixKnobLed => Led::Knob(KnobLed::MonitorMix),
            Actuator::DeckACueButtonLed => Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Cue)),
            Actuator::DeckAFxButtonLed => Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Fx)),
            Actuator::DeckAMonitorButtonLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Monitor))
            }
            Actuator::DeckAPlayPauseButtonLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::PlayPause))
            }
            Actuator::DeckAShiftButtonLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Shift))
            }
            Actuator::DeckASyncButtonLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Sync))
            }
            Actuator::DeckATouchStripCenterLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::TouchStripCenter))
            }
            Actuator::DeckATouchStripHotCueCenterLed => Led::Deck(
                Deck::A,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueCenter),
            ),
            Actuator::DeckATouchStripHotCueLeftLed => Led::Deck(
                Deck::A,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueLeft),
            ),
            Actuator::DeckATouchStripHotCueRightLed => Led::Deck(
                Deck::A,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueRight),
            ),
            Actuator::DeckATouchStripLeftLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::TouchStripLeft))
            }
            Actuator::DeckATouchStripLoopCenterLed => Led::Deck(
                Deck::A,
                DeckLed::Button(DeckButtonLed::TouchStripLoopCenter),
            ),
            Actuator::DeckATouchStripLoopLeftLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::TouchStripLoopLeft))
            }
            Actuator::DeckATouchStripLoopRightLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::TouchStripLoopRight))
            }
            Actuator::DeckATouchStripRightLed => {
                Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::TouchStripRight))
            }
            Actuator::DeckAGainKnobLed => Led::Deck(Deck::A, DeckLed::Knob(DeckKnobLed::Gain)),
            Actuator::DeckAEqHiKnobLed => Led::Deck(Deck::A, DeckLed::Knob(DeckKnobLed::EqHi)),
            Actuator::DeckAEqLoKnobLed => Led::Deck(Deck::A, DeckLed::Knob(DeckKnobLed::EqLo)),
            Actuator::DeckAEqMidKnobLed => Led::Deck(Deck::A, DeckLed::Knob(DeckKnobLed::EqMid)),
            Actuator::DeckBCueButtonLed => Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Cue)),
            Actuator::DeckBFxButtonLed => Led::Deck(Deck::A, DeckLed::Button(DeckButtonLed::Fx)),
            Actuator::DeckBMonitorButtonLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::Monitor))
            }
            Actuator::DeckBPlayPauseButtonLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::PlayPause))
            }
            Actuator::DeckBShiftButtonLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::Shift))
            }
            Actuator::DeckBSyncButtonLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::Sync))
            }
            Actuator::DeckBTouchStripCenterLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::TouchStripCenter))
            }
            Actuator::DeckBTouchStripHotCueCenterLed => Led::Deck(
                Deck::B,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueCenter),
            ),
            Actuator::DeckBTouchStripHotCueLeftLed => Led::Deck(
                Deck::B,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueLeft),
            ),
            Actuator::DeckBTouchStripHotCueRightLed => Led::Deck(
                Deck::B,
                DeckLed::Button(DeckButtonLed::TouchStripHotCueRight),
            ),
            Actuator::DeckBTouchStripLeftLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::TouchStripLeft))
            }
            Actuator::DeckBTouchStripLoopCenterLed => Led::Deck(
                Deck::B,
                DeckLed::Button(DeckButtonLed::TouchStripLoopCenter),
            ),
            Actuator::DeckBTouchStripLoopLeftLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::TouchStripLoopLeft))
            }
            Actuator::DeckBTouchStripLoopRightLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::TouchStripLoopRight))
            }
            Actuator::DeckBTouchStripRightLed => {
                Led::Deck(Deck::B, DeckLed::Button(DeckButtonLed::TouchStripRight))
            }
            Actuator::DeckBGainKnobLed => Led::Deck(Deck::B, DeckLed::Knob(DeckKnobLed::Gain)),
            Actuator::DeckBEqHiKnobLed => Led::Deck(Deck::B, DeckLed::Knob(DeckKnobLed::EqHi)),
            Actuator::DeckBEqLoKnobLed => Led::Deck(Deck::B, DeckLed::Knob(DeckKnobLed::EqLo)),
            Actuator::DeckBEqMidKnobLed => Led::Deck(Deck::B, DeckLed::Knob(DeckKnobLed::EqMid)),
        }
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
                        DeckKnobLed::EqHi => MIDI_DECK_EQ_HI_KNOB,
                        DeckKnobLed::EqMid => MIDI_DECK_EQ_MID_KNOB,
                        DeckKnobLed::EqLo => MIDI_DECK_EQ_LO_KNOB,
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
