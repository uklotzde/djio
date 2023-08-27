// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::{BoxedMidiOutputConnection, Controller, MidiDeviceDescriptor, MidiOutputGateway};

#[cfg(feature = "controller-thread")]
pub(crate) mod context;

pub trait MidiController: Controller + MidiOutputGateway<BoxedMidiOutputConnection> {
    #[must_use]
    fn midi_device_descriptor(&self) -> &MidiDeviceDescriptor;
}

pub type BoxedMidiController<T> = Box<dyn MidiController<Types = T> + Send + 'static>;
