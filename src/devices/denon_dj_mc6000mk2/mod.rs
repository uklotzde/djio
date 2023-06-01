// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::borrow::Cow;

use crate::{DeviceDescriptor, MidiDeviceDescriptor};

pub const MIDI_DEVICE_DESCRIPTOR: &MidiDeviceDescriptor = &MidiDeviceDescriptor {
    device: DeviceDescriptor {
        vendor_name: Cow::Borrowed("Denon DJ"),
        product_name: Cow::Borrowed("MC6000MK2"),
    },
    port_name_prefix: "MC6000MK2",
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &MIDI_DEVICE_DESCRIPTOR.device;
