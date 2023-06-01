// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive as _;

use super::Deck;
use crate::{ControlIndex, LedOutput, OutputResult};

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

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
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
    DeckALoEqKnobLed,
    DeckAMidEqKnobLed,
    DeckAHiEqKnobLed,
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
    DeckBLoEqKnobLed,
    DeckBMidEqKnobLed,
    DeckBHiEqKnobLed,
}

impl From<Actuator> for ControlIndex {
    fn from(value: Actuator) -> Self {
        ControlIndex::new(value.to_u32().expect("u32"))
    }
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
