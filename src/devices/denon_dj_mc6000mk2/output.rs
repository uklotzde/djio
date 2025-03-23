// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{Control, ControlOutputGateway, MidiOutputConnection, MidiOutputGateway, OutputResult};

#[expect(missing_debug_implementations)]
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

impl<C: MidiOutputConnection> ControlOutputGateway for OutputGateway<C> {
    fn send_output(&mut self, output: &Control) -> OutputResult<()> {
        unimplemented!("TODO: Send MIDI output message for {output:?}");
    }
}

impl<C: MidiOutputConnection> MidiOutputGateway<C> for OutputGateway<C> {
    fn attach_midi_output_connection(
        &mut self,
        midi_output_connection: &mut Option<C>,
    ) -> OutputResult<()> {
        assert!(self.midi_output_connection.is_none());
        assert!(midi_output_connection.is_some());
        self.midi_output_connection = midi_output_connection.take();
        Ok(())
    }

    fn detach_midi_output_connection(&mut self) -> Option<C> {
        self.midi_output_connection.take()
    }
}
