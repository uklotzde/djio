// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::ops::{Deref, DerefMut};

use smol_str::SmolStr;

use crate::{
    ControlInputEvent, ControlInputEventSink, ControlOutputGateway, DeviceDescriptor, OutputResult,
    PortIndex, TimeStamp,
};

#[cfg(feature = "midir")]
pub(crate) mod midir;

const MIDI_OUTPUT_SYSTEM_RESET: &[u8] = &[0xff];

/// MIDI-related, extended [`DeviceDescriptor`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiDeviceDescriptor {
    pub device: DeviceDescriptor,
    pub port_name_prefix: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPortDescriptor {
    pub index: PortIndex,
    pub name: SmolStr,
}

pub trait MidiInputConnector {
    /// Invoked before (re-)connecting the input port.
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        input_port: &MidiPortDescriptor,
    );
}

impl<D> MidiInputConnector for D
where
    D: DerefMut + ?Sized,
    <D as Deref>::Target: MidiInputConnector,
{
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        port: &MidiPortDescriptor,
    ) {
        self.deref_mut().connect_midi_input_port(device, port);
    }
}

#[derive(Debug)]
pub struct MidiInputDecodeError;

/// Decode and map received MIDI messages into [`ControlInputEvent`]s.
pub trait MidiInputEventDecoder {
    /// Decode the next MIDI message
    ///
    /// Not each successfully decoded MIDI input might result in an event,
    /// i.e. returning `Ok(None)` is not an error.
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError>;
}

impl<F> MidiInputEventDecoder for F
where
    F: FnMut(TimeStamp, &[u8]) -> Result<Option<ControlInputEvent>, MidiInputDecodeError>,
{
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
        self(ts, input)
    }
}

/// Passive callback for sinking MIDI input messages
pub trait MidiInputHandler {
    /// Invoked for each incoming message.
    ///
    /// Returns `true` if the message has been accepted and handled
    /// or `false` otherwise.
    #[must_use]
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool;
}

impl<D> MidiInputHandler for D
where
    D: DerefMut + ?Sized,
    <D as Deref>::Target: MidiInputHandler,
{
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        self.deref_mut().handle_midi_input(ts, input)
    }
}

pub fn consume_midi_input_event<D, E>(
    ts: TimeStamp,
    input: &[u8],
    decoder: &mut D,
    event_sink: &mut E,
) -> bool
where
    D: MidiInputEventDecoder + ?Sized,
    E: ControlInputEventSink + ?Sized,
{
    match decoder.try_decode_midi_input_event(ts, input) {
        Ok(Some(event)) => {
            event_sink.sink_control_input_events(&[event]);
            true
        }
        Ok(None) => true,
        Err(MidiInputDecodeError) => {
            log::warn!("Failed to decode MIDI input: {ts} {input:x?}");
            false
        }
    }
}

pub trait MidiOutputConnection {
    fn send_midi_output(&mut self, output: &[u8]) -> OutputResult<()>;

    fn send_midi_system_reset(&mut self) -> OutputResult<()> {
        self.send_midi_output(MIDI_OUTPUT_SYSTEM_RESET)
    }
}

pub type BoxedMidiOutputConnection = Box<dyn MidiOutputConnection + Send + 'static>;

impl<T> MidiOutputConnection for Box<T>
where
    T: MidiOutputConnection + ?Sized,
{
    fn send_midi_output(&mut self, output: &[u8]) -> OutputResult<()> {
        self.as_mut().send_midi_output(output)
    }
}

pub trait MidiInputGateway: MidiInputConnector + MidiInputHandler {}

impl<D> MidiInputGateway for D where D: MidiInputConnector + MidiInputHandler {}

pub trait NewMidiInputGateway {
    type MidiInputGateway: self::MidiInputGateway;

    fn new_midi_input_gateway(
        &self,
        device: &MidiDeviceDescriptor,
        input_port: &MidiPortDescriptor,
    ) -> Self::MidiInputGateway;
}

pub trait MidiOutputGateway<C> {
    /// Attach a MIDI output connection
    ///
    /// Consumes the given connection on success.
    ///
    /// # Panics
    ///
    /// Panics if a connection has already been attached or if `midi_output_connection`
    /// is `None`.
    fn attach_midi_output_connection(
        &mut self,
        midi_output_connection: &mut Option<C>,
    ) -> OutputResult<()>;

    /// Attach a MIDI output connection
    ///
    /// Returns `None` if no connection has been attached or if the connection
    /// has already been detached.
    fn detach_midi_output_connection(&mut self) -> Option<C>;
}

pub trait MidiControlOutputGateway<C>: ControlOutputGateway + MidiOutputGateway<C> {}

impl<T, C> MidiControlOutputGateway<C> for T where T: ControlOutputGateway + MidiOutputGateway<C> {}
