// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;

use super::Deck;
use crate::{LedOutput, OutputResult};

#[derive(Debug, Clone, Copy)]
pub enum Led {
    MonitorLevelKnob,
    MonitorMixKnob,
    MasterLevelKnob,
    TabButton,
    TabHoldButton,
    HoldButton,
}

#[derive(Debug, Clone, Copy)]
pub enum DeckLed {
    ShiftButton,
    PlayPauseButton,
    SyncButton,
    CueButton,
    MonitorButton,
    FxButton,
    TouchStripLeft,
    TouchStripCenter,
    TouchStripRight,
    TouchStripLoopLeft,
    TouchStripLoopCenter,
    TouchStripLoopRight,
    TouchStripHotCueLeft,
    TouchStripHotCueCenter,
    TouchStripHotCueRight,
    GainKnob,
    LoEqKnob,
    MidEqKnob,
    HiEqKnob,
}

fn led_to_u7(output: LedOutput) -> u8 {
    match output {
        LedOutput::Off => 0x00,
        LedOutput::On => 0x7f,
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

    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn send_led_output(&mut self, led: Led, output: LedOutput) -> OutputResult<()> {
        #[allow(clippy::match_single_binding)] // FIXME
        match led {
            _ => unimplemented!("FIXME: {led:?} -> {output:?}"),
        }
    }

    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn send_deck_led_output(
        &mut self,
        deck: Deck,
        led: DeckLed,
        output: LedOutput,
    ) -> OutputResult<()> {
        let status = match deck {
            Deck::A => 0x97,
            Deck::B => 0x98,
        };
        let data1 = match led {
            DeckLed::ShiftButton => 0x1a,
            DeckLed::PlayPauseButton => 0x1b,
            DeckLed::SyncButton => 0x1d,
            DeckLed::CueButton => 0x1e,
            _ => unimplemented!("FIXME: {led:?}@{deck:?} -> {output:?}"),
        };
        let data2 = led_to_u7(output);
        self.midi_output_connection.send(&[status, data1, data2])?;
        Ok(())
    }
}
