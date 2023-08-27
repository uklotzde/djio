// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{
    Control, ControlOutputGateway, MidiOutputConnection, MidiOutputGateway, OutputError,
    OutputResult,
};

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

impl<C: MidiOutputConnection> ControlOutputGateway for OutputGateway<C> {
    fn send_output(&mut self, output: &Control) -> OutputResult<()> {
        let Some(midi_output_connection) = &mut self.midi_output_connection else {
            return Err(OutputError::Disconnected);
        };
        let Control { index, value } = *output;
        let status = ((index.value() >> 7) & 0x7f) as u8;
        let command = (index.value() & 0x7f) as u8;
        let data = (value.to_bits() & 0x7f) as u8;
        midi_output_connection.send_midi_output(&[status, command, data])
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
