// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator as _};

use super::{
    Deck, CONTROL_INDEX_DECK_A, CONTROL_INDEX_DECK_B, CONTROL_INDEX_DECK_BIT_MASK,
    CONTROL_INDEX_ENUM_BIT_MASK, MIDI_DECK_CUE_BUTTON, MIDI_DECK_EQ_HI_KNOB, MIDI_DECK_EQ_LO_KNOB,
    MIDI_DECK_EQ_MID_KNOB, MIDI_DECK_GAIN_KNOB, MIDI_DECK_MONITOR_BUTTON,
    MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_SYNC_BUTTON, MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
    MIDI_MASTER_LEVEL_KNOB, MIDI_MONITOR_LEVEL_KNOB, MIDI_MONITOR_MIX_KNOB,
    MIDI_STATUS_BUTTON_DECK_A, MIDI_STATUS_BUTTON_DECK_B, MIDI_STATUS_BUTTON_MAIN,
    MIDI_STATUS_CC_DECK_A, MIDI_STATUS_CC_DECK_B, MIDI_STATUS_CC_MAIN, MIDI_TAP_BUTTON,
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

#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum MainLed {
    TabButton,
    MonitorLevelKnob,
    MonitorBalanceKnob,
    MasterLevelKnob,
}

impl MainLed {
    #[must_use]
    pub const fn is_knob(self) -> bool {
        !matches!(self, Self::TabButton)
    }
}

/// Deck LED
///
/// Special cases:
/// - The Shift button LED cannot be controlled.
/// - The Fx button LED can only be toggled, not set to a desired on/off state.
#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DeckLed {
    PlayPauseButton,
    SyncButton,
    CueButton,
    MonitorButton,
    TouchStripLeftButton,
    TouchStripCenterButton,
    TouchStripRightButton,
    TouchStripLoopLeftButton,
    TouchStripLoopCenterButton,
    TouchStripLoopRightButton,
    TouchStripHotCueLeftButton,
    TouchStripHotCueCenterButton,
    TouchStripHotCueRightButton,
    GainKnob,
    EqLoKnob,
    EqMidKnob,
    EqHiKnob,
}

impl DeckLed {
    #[must_use]
    pub const fn is_knob(self) -> bool {
        matches!(
            self,
            Self::GainKnob | Self::EqHiKnob | Self::EqLoKnob | Self::EqMidKnob
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Led {
    Main(MainLed),
    Deck(Deck, DeckLed),
}

impl Led {
    #[must_use]
    pub const fn deck(self) -> Option<Deck> {
        match self {
            Self::Main(_) => None,
            Self::Deck(deck, _) => Some(deck),
        }
    }

    #[must_use]
    pub const fn to_control_index(self) -> ControlIndex {
        match self {
            Self::Main(led) => ControlIndex::new(led as u32),
            Self::Deck(deck, led) => {
                let deck_bit = match deck {
                    Deck::A => CONTROL_INDEX_DECK_A,
                    Deck::B => CONTROL_INDEX_DECK_B,
                };
                ControlIndex::new(deck_bit | led as u32)
            }
        }
    }
}

impl From<MainLed> for Led {
    fn from(from: MainLed) -> Self {
        Self::Main(from)
    }
}

impl From<Led> for ControlIndex {
    fn from(from: Led) -> Self {
        from.to_control_index()
    }
}

#[derive(Debug)]
pub struct InvalidOutputControlIndex;

impl TryFrom<ControlIndex> for Led {
    type Error = InvalidOutputControlIndex;

    fn try_from(from: ControlIndex) -> Result<Self, Self::Error> {
        let value = from.value();
        debug_assert!(CONTROL_INDEX_ENUM_BIT_MASK <= u8::MAX.into());
        let enum_index = (value & CONTROL_INDEX_ENUM_BIT_MASK) as u8;
        let deck = match value & CONTROL_INDEX_DECK_BIT_MASK {
            CONTROL_INDEX_DECK_A => Deck::A,
            CONTROL_INDEX_DECK_B => Deck::B,
            CONTROL_INDEX_DECK_BIT_MASK => return Err(InvalidOutputControlIndex),
            _ => {
                return MainLed::from_repr(enum_index)
                    .map(Led::Main)
                    .ok_or(InvalidOutputControlIndex);
            }
        };
        DeckLed::from_repr(enum_index)
            .map(|led| Led::Deck(deck, led))
            .ok_or(InvalidOutputControlIndex)
    }
}

#[allow(missing_debug_implementations)]
pub struct OutputGateway {
    midi_output_connection: MidiOutputConnection,
}

impl OutputGateway {
    pub fn attach(mut midi_output_connection: MidiOutputConnection) -> OutputResult<Self> {
        // MIDI SysEx message for querying the initial position of all knobs and faders
        const MIDI_STATUS_SYSEX: &[u8] = &[
            0xf0, 0x42, 0x40, 0x00, 0x01, 0x28, 0x00, 0x1f, 0x70, 0x01, 0xf7,
        ];
        midi_output_connection.send(MIDI_STATUS_SYSEX)?;
        let mut gateway = Self {
            midi_output_connection,
        };
        gateway.reset_all_leds()?;
        Ok(gateway)
    }

    #[must_use]
    pub fn detach(self) -> MidiOutputConnection {
        let Self {
            midi_output_connection,
        } = self;
        midi_output_connection
    }

    fn reset_all_leds(&mut self) -> OutputResult<()> {
        for led in MainLed::iter() {
            let output = if led.is_knob() {
                LedOutput::On
            } else {
                LedOutput::Off
            };
            self.send_led_output(led.into(), output)?;
        }
        for deck in Deck::iter() {
            for led in DeckLed::iter() {
                let output = if led.is_knob() {
                    LedOutput::On
                } else {
                    LedOutput::Off
                };
                self.send_led_output(Led::Deck(deck, led), output)?;
            }
        }
        Ok(())
    }

    pub fn send_led_output(&mut self, led: Led, output: LedOutput) -> OutputResult<()> {
        let (status, data1) = match led {
            Led::Main(led) => match led {
                MainLed::TabButton => (MIDI_TAP_BUTTON, MIDI_STATUS_BUTTON_MAIN),
                MainLed::MonitorLevelKnob => (MIDI_MONITOR_LEVEL_KNOB, MIDI_STATUS_CC_MAIN),
                MainLed::MonitorBalanceKnob => (MIDI_MONITOR_MIX_KNOB, MIDI_STATUS_CC_MAIN),
                MainLed::MasterLevelKnob => (MIDI_MASTER_LEVEL_KNOB, MIDI_STATUS_CC_MAIN),
            },
            Led::Deck(deck, led) => {
                let status = match (deck, led.is_knob()) {
                    (Deck::A, false) => MIDI_STATUS_BUTTON_DECK_A,
                    (Deck::A, true) => MIDI_STATUS_CC_DECK_A,
                    (Deck::B, false) => MIDI_STATUS_BUTTON_DECK_B,
                    (Deck::B, true) => MIDI_STATUS_CC_DECK_B,
                };
                let data1 = match led {
                    DeckLed::MonitorButton => MIDI_DECK_MONITOR_BUTTON,
                    DeckLed::PlayPauseButton => MIDI_DECK_PLAYPAUSE_BUTTON,
                    DeckLed::SyncButton => MIDI_DECK_SYNC_BUTTON,
                    DeckLed::CueButton => MIDI_DECK_CUE_BUTTON,
                    DeckLed::TouchStripCenterButton => MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
                    DeckLed::TouchStripHotCueCenterButton => {
                        MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON
                    }
                    DeckLed::TouchStripHotCueLeftButton => MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
                    DeckLed::TouchStripHotCueRightButton => {
                        MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON
                    }
                    DeckLed::TouchStripLeftButton => MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
                    DeckLed::TouchStripLoopCenterButton => MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON,
                    DeckLed::TouchStripLoopLeftButton => MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
                    DeckLed::TouchStripLoopRightButton => MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON,
                    DeckLed::TouchStripRightButton => MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
                    DeckLed::GainKnob => MIDI_DECK_GAIN_KNOB,
                    DeckLed::EqHiKnob => MIDI_DECK_EQ_HI_KNOB,
                    DeckLed::EqMidKnob => MIDI_DECK_EQ_MID_KNOB,
                    DeckLed::EqLoKnob => MIDI_DECK_EQ_LO_KNOB,
                };
                (status, data1)
            }
        };
        let data2 = led_to_u7(output);
        self.midi_output_connection.send(&[status, data1, data2])?;
        Ok(())
    }
}
