// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "denon-dj-mc6000mk2")]
pub mod denon_dj_mc6000mk2;

#[cfg(feature = "korg-kaoss-dj")]
pub mod korg_kaoss_dj;

#[cfg(feature = "pioneer-ddj-400")]
pub mod pioneer_ddj_400;

// Descriptors of supported MIDI DJ controllers for auto-detection.
#[cfg(feature = "midi-controllers")]
pub const MIDI_DJ_CONTROLLER_DESCRIPTORS: &[&crate::MidiDeviceDescriptor] = &[
    crate::devices::denon_dj_mc6000mk2::MIDI_DEVICE_DESCRIPTOR,
    crate::devices::korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR,
    crate::devices::pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR,
];

#[cfg(feature = "ni-traktor-kontrol-s4mk3")]
pub mod ni_traktor_kontrol_s4mk3;

// Descriptors of supported HID DJ controllers for auto-detection.
#[cfg(feature = "hid-controllers")]
pub const HID_DJ_CONTROLLER_DESCRIPTORS: &[&crate::DeviceDescriptor] =
    &[crate::devices::ni_traktor_kontrol_s4mk3::DEVICE_DESCRIPTOR];
