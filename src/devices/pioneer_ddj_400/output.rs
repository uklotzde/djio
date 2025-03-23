// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use derive_more::From;
use smol_str::format_smolstr;
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator as _};

use super::{
    CONTROL_INDEX_DECK_BIT_MASK, CONTROL_INDEX_DECK_ONE, CONTROL_INDEX_DECK_TWO,
    CONTROL_INDEX_ENUM_BIT_MASK, Deck, MIDI_BEAT_FX, MIDI_COMMAND_NOTE_ON,
    MIDI_DECK_PLAYPAUSE_BUTTON, MIDI_MASTER_CUE, MIDI_STATUS_BUTTON_MAIN,
};
use crate::{
    Control, ControlIndex, ControlOutputGateway, LedOutput, MidiOutputConnection,
    MidiOutputGateway, OutputError, OutputResult,
};

#[derive(Debug, Clone, Copy, From)]
pub enum Led {
    Main(MainLed),
    Deck(Deck, DeckLed),
    // TODO: Performance LEDs
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

const LED_OFF: u8 = 0x00;
const LED_ON: u8 = 0x7f;

const fn led_to_u7(output: LedOutput) -> u8 {
    match output {
        LedOutput::Off => LED_OFF,
        LedOutput::On => LED_ON,
    }
}

/// Deck LED
#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DeckLed {
    PlayPauseButton,
    CueButton,
    BeatSyncButton,
    LoopInButton,
    LoopOutButton,
    ReloopExitButton,
    // -- Mixer section -- //
    HeadphoneCueButton,
}

/// Main LED
#[derive(Debug, Clone, Copy, FromRepr, EnumIter, EnumCount)]
#[repr(u8)]
pub enum MainLed {
    MasterCue,
    BeatFx,
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
            CONTROL_INDEX_DECK_ONE => Deck::One,
            CONTROL_INDEX_DECK_TWO => Deck::Two,
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
            MainLed::MasterCue => (MIDI_STATUS_BUTTON_MAIN, MIDI_MASTER_CUE),
            MainLed::BeatFx => (MIDI_STATUS_BUTTON_MAIN, MIDI_BEAT_FX),
        },
        Led::Deck(deck, led) => {
            let midi_channel = deck.midi_channel();
            let status = MIDI_COMMAND_NOTE_ON | midi_channel;
            let data1 = match led {
                DeckLed::PlayPauseButton => MIDI_DECK_PLAYPAUSE_BUTTON,
                DeckLed::CueButton => 0x0c,
                DeckLed::BeatSyncButton => 0x58,
                DeckLed::LoopInButton => 0x10,
                DeckLed::LoopOutButton => 0x11,
                DeckLed::ReloopExitButton => 0x4D,
                DeckLed::HeadphoneCueButton => 0x54,
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
    // TODO: How to query the initial position of all knobs and faders?
    turn_off_all_leds(midi_output_connection)?;
    Ok(())
}

fn on_detach<C: MidiOutputConnection>(midi_output_connection: &mut C) -> OutputResult<()> {
    turn_off_all_leds(midi_output_connection)?;
    Ok(())
}

fn turn_off_all_leds<C: MidiOutputConnection>(midi_output_connection: &mut C) -> OutputResult<()> {
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
            msg: format_smolstr!("no LED with control index {index}"),
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
