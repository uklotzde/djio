// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "midi")]
pub mod generic_midi;

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

#[cfg(all(feature = "ni-traktor-kontrol-s4mk3", not(target_family = "wasm")))]
pub mod ni_traktor_kontrol_s4mk3;

// Descriptors of supported HID DJ controllers for auto-detection.
#[cfg(all(feature = "hid-controllers", not(target_family = "wasm")))]
pub const HID_DJ_CONTROLLER_DESCRIPTORS: &[&crate::DeviceDescriptor] =
    &[crate::devices::ni_traktor_kontrol_s4mk3::DEVICE_DESCRIPTOR];
