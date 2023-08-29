// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator as _};

use super::{
    Deck, CONTROL_INDEX_DECK_A, CONTROL_INDEX_DECK_B, CONTROL_INDEX_DECK_BIT_MASK,
    CONTROL_INDEX_ENUM_BIT_MASK, MIDI_COMMAND_CC, MIDI_COMMAND_NOTE_ON, MIDI_DECK_CUE_BUTTON,
    MIDI_DECK_EQ_HI_KNOB, MIDI_DECK_EQ_LO_KNOB, MIDI_DECK_EQ_MID_KNOB, MIDI_DECK_GAIN_KNOB,
    MIDI_DECK_MONITOR_BUTTON, MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_DECK_SHIFT_BUTTON,
    MIDI_DECK_SYNC_BUTTON, MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_CENTER_BUTTON, MIDI_DECK_TOUCHSTRIP_LOOP_LEFT_BUTTON,
    MIDI_DECK_TOUCHSTRIP_LOOP_RIGHT_BUTTON, MIDI_DECK_TOUCHSTRIP_RIGHT_BUTTON,
    MIDI_MASTER_LEVEL_KNOB, MIDI_MONITOR_LEVEL_KNOB, MIDI_MONITOR_MIX_KNOB,
    MIDI_STATUS_BUTTON_MAIN, MIDI_STATUS_CC_MAIN, MIDI_TAP_BUTTON,
};
use crate::{
    Control, ControlIndex, ControlOutputGateway, LedOutput, MidiOutputConnection,
    MidiOutputGateway, OutputError, OutputResult,
};

const LED_OFF: u8 = 0x00;
const LED_ON: u8 = 0x7f;

const fn led_to_u7(output: LedOutput) -> u8 {
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
    MonitorButton,
    ShiftButton,
    PlayPauseButton,
    SyncButton,
    CueButton,
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
            Self::Deck(deck, led) => ControlIndex::new(deck.control_index_bit_mask() | led as u32),
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

#[must_use]
pub const fn led_output_into_midi_message(led: Led, output: LedOutput) -> [u8; 3] {
    let (status, data1) = match led {
        Led::Main(led) => match led {
            MainLed::TabButton => (MIDI_STATUS_BUTTON_MAIN, MIDI_TAP_BUTTON),
            MainLed::MonitorLevelKnob => (MIDI_STATUS_CC_MAIN, MIDI_MONITOR_LEVEL_KNOB),
            MainLed::MonitorBalanceKnob => (MIDI_STATUS_CC_MAIN, MIDI_MONITOR_MIX_KNOB),
            MainLed::MasterLevelKnob => (MIDI_STATUS_CC_MAIN, MIDI_MASTER_LEVEL_KNOB),
        },
        Led::Deck(deck, led) => {
            let midi_channel = deck.midi_channel();
            let status = if led.is_knob() {
                MIDI_COMMAND_CC | midi_channel
            } else {
                MIDI_COMMAND_NOTE_ON | midi_channel
            };
            let data1 = match led {
                DeckLed::MonitorButton => MIDI_DECK_MONITOR_BUTTON,
                DeckLed::ShiftButton => MIDI_DECK_SHIFT_BUTTON,
                DeckLed::PlayPauseButton => MIDI_DECK_PLAYPAUSE_BUTTON,
                DeckLed::SyncButton => MIDI_DECK_SYNC_BUTTON,
                DeckLed::CueButton => MIDI_DECK_CUE_BUTTON,
                DeckLed::TouchStripCenterButton => MIDI_DECK_TOUCHSTRIP_CENTER_BUTTON,
                DeckLed::TouchStripHotCueCenterButton => MIDI_DECK_TOUCHSTRIP_HOTCUE_CENTER_BUTTON,
                DeckLed::TouchStripHotCueLeftButton => MIDI_DECK_TOUCHSTRIP_HOTCUE_LEFT_BUTTON,
                DeckLed::TouchStripHotCueRightButton => MIDI_DECK_TOUCHSTRIP_HOTCUE_RIGHT_BUTTON,
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
    [status, data1, data2]
}

fn send_led_output<C: MidiOutputConnection>(
    midi_output_connection: &mut C,
    led: Led,
    output: LedOutput,
) -> OutputResult<()> {
    midi_output_connection.send_midi_output(&led_output_into_midi_message(led, output))
}

fn on_attach<C: MidiOutputConnection>(midi_output_connection: &mut C) -> OutputResult<()> {
    // MIDI SysEx message for querying the initial position of all knobs and faders
    const MIDI_STATUS_SYSEX: &[u8] = &[
        0xf0, 0x42, 0x40, 0x00, 0x01, 0x28, 0x00, 0x1f, 0x70, 0x01, 0xf7,
    ];
    midi_output_connection.send_midi_output(MIDI_STATUS_SYSEX)?;
    for led in MainLed::iter() {
        let output = if led.is_knob() {
            LedOutput::On
        } else {
            LedOutput::Off
        };
        send_led_output(midi_output_connection, led.into(), output)?;
    }
    for deck in Deck::iter() {
        for led in DeckLed::iter() {
            let output = if led.is_knob() {
                LedOutput::On
            } else {
                LedOutput::Off
            };
            send_led_output(midi_output_connection, Led::Deck(deck, led), output)?;
        }
    }
    Ok(())
}

fn on_detach<C: MidiOutputConnection>(midi_output_connection: &mut C) -> OutputResult<()> {
    for led in MainLed::iter() {
        send_led_output(midi_output_connection, led.into(), LedOutput::Off)?;
    }
    for deck in Deck::iter() {
        for led in DeckLed::iter() {
            send_led_output(midi_output_connection, Led::Deck(deck, led), LedOutput::Off)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
#[allow(missing_debug_implementations)]
pub struct OutputGateway<C> {
    midi_output_connection: Option<C>,
}

impl<C> Default for OutputGateway<C> {
    fn default() -> Self {
        Self {
            midi_output_connection: None,
        }
    }
}

impl<C: MidiOutputConnection> OutputGateway<C> {
    pub fn send_led_output(&mut self, led: Led, output: LedOutput) -> OutputResult<()> {
        let Some(midi_output_connection) = &mut self.midi_output_connection else {
            return Err(OutputError::Disconnected);
        };
        send_led_output(midi_output_connection, led, output)
    }
}

impl<C: MidiOutputConnection> ControlOutputGateway for OutputGateway<C> {
    fn send_output(&mut self, output: &Control) -> OutputResult<()> {
        let Control { index, value } = *output;
        let led = Led::try_from(index).map_err(|InvalidOutputControlIndex| OutputError::Send {
            msg: format!("No LED with control index {index}").into(),
        })?;
        self.send_led_output(led, value.into())
    }
}

impl<C: MidiOutputConnection> MidiOutputGateway<C> for OutputGateway<C> {
    fn attach_midi_output_connection(
        &mut self,
        midi_output_connection: &mut Option<C>,
    ) -> OutputResult<()> {
        assert!(self.midi_output_connection.is_none());
        assert!(midi_output_connection.is_some());
        // Initialize the hardware
        on_attach(midi_output_connection.as_mut().expect("Some"))?;
        // Finally take ownership
        self.midi_output_connection = midi_output_connection.take();
        Ok(())
    }

    fn detach_midi_output_connection(&mut self) -> Option<C> {
        // Release ownership
        let mut midi_output_connection = self.midi_output_connection.take()?;
        // Reset the hardware
        if let Err(err) = on_detach(&mut midi_output_connection) {
            log::warn!("Failed reset MIDI hardware on detach: {err}");
        }
        Some(midi_output_connection)
    }
}
