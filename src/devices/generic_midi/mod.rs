// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use crate::DeviceDescriptor;

mod input;
pub use self::input::{try_decode_midi_input, try_decode_midi_input_event, MidiInputEventDecoder};

mod output;
pub use self::output::OutputGateway;

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &DeviceDescriptor {
    vendor_name: Cow::Borrowed("Unknown"),
    product_name: Cow::Borrowed("Generic MIDI"),
    audio_interface: None,
};
