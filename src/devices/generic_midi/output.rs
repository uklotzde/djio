// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use midir::MidiOutputConnection;

use crate::{ControlRegister, OutputResult};

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

    pub fn send_midi_value(&mut self, output: &ControlRegister) -> OutputResult<()> {
        let ControlRegister { index, value } = *output;
        let status = ((index.value() >> 7) & 0x7f) as u8;
        let command = (index.value() & 0x7f) as u8;
        let data = (value.to_bits() & 0x7f) as u8;
        self.midi_output_connection.send(&[status, command, data])?;
        Ok(())
    }
}
