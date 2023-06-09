// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{ControlOutputGateway, ControlRegister, MidiOutputConnection, OutputResult};

#[allow(missing_debug_implementations)]
pub struct OutputGateway<C> {
    midi_output_connection: C,
}

impl<C: MidiOutputConnection> OutputGateway<C> {
    #[must_use]
    pub fn attach(midi_output_connection: C) -> Self {
        Self {
            midi_output_connection,
        }
    }

    #[must_use]
    pub fn detach(self) -> C {
        let Self {
            midi_output_connection,
        } = self;
        midi_output_connection
    }
}

impl<C: MidiOutputConnection> ControlOutputGateway for OutputGateway<C> {
    fn send_output(&mut self, output: &ControlRegister) -> OutputResult<()> {
        let ControlRegister { index, value } = *output;
        let status = ((index.value() >> 7) & 0x7f) as u8;
        let command = (index.value() & 0x7f) as u8;
        let data = (value.to_bits() & 0x7f) as u8;
        self.midi_output_connection
            .send_midi_output(&[status, command, data])
    }
}
