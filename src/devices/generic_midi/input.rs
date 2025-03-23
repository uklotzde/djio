// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{
    Control, ControlIndex, ControlInputEvent, ControlValue, MidiInputConnector,
    MidiInputDecodeError, TimeStamp,
};

pub fn try_decode_midi_input(input: &[u8]) -> Result<Option<Control>, MidiInputDecodeError> {
    let [status, command, value] = *input else {
        return Err(MidiInputDecodeError);
    };
    let index = ControlIndex::new((u32::from(status) << 7) | u32::from(command));
    let value = ControlValue::from_bits(u32::from(value));
    let decoded = Control { index, value };
    Ok(Some(decoded))
}

pub fn try_decode_midi_input_event(
    ts: TimeStamp,
    input: &[u8],
) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
    let input = try_decode_midi_input(input)?;
    Ok(input.map(|input| ControlInputEvent { ts, input }))
}

#[derive(Debug, Clone, Default)]
pub struct MidiInputEventDecoder;

impl crate::MidiInputEventDecoder for MidiInputEventDecoder {
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
        try_decode_midi_input_event(ts, input)
    }
}

impl MidiInputConnector for MidiInputEventDecoder {
    fn connect_midi_input_port(
        &mut self,
        _device: &crate::MidiDeviceDescriptor,
        _input_port: &crate::MidiPortDescriptor,
    ) {
    }
}
