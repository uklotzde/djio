// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::MidiDeviceDescriptor;

#[cfg(feature = "midi")]
pub mod denon_dj_mc6000mk2;

#[cfg(feature = "midi")]
pub mod korg_kaoss_dj;

#[cfg(feature = "midi")]
pub mod pioneer_ddj_400;

// Predefined port names of existing MIDI DJ controllers for auto-detection.
#[cfg(feature = "midi")]
pub const MIDI_DJ_CONTROLLER_DESCRIPTORS: &[&MidiDeviceDescriptor] = &[
    crate::devices::denon_dj_mc6000mk2::MIDI_DEVICE_DESCRIPTOR,
    crate::devices::korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR,
    crate::devices::pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR,
];

#[cfg(feature = "hid")]
pub mod ni_traktor_s4mk3;
