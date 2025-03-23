// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use smol_str::SmolStr;

use crate::DeviceDescriptor;

mod input;
pub use self::input::{MidiInputEventDecoder, try_decode_midi_input, try_decode_midi_input_event};

mod output;
pub use self::output::OutputGateway;

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &DeviceDescriptor {
    vendor_name: SmolStr::new_static("Unknown"),
    product_name: SmolStr::new_static("Generic MIDI"),
    audio_interface: None,
};
