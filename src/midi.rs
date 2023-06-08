// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    borrow::Cow,
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use midir::{
    ConnectError, Ignore, InitError, MidiInput, MidiInputConnection, MidiInputPort, MidiInputPorts,
    MidiOutput, MidiOutputConnection, MidiOutputPort, MidiOutputPorts, SendError,
};
use thiserror::Error;

use crate::{
    ControlInputEvent, ControlInputEventSink, DeviceDescriptor, OutputError, PortIndex,
    PortIndexGenerator, TimeStamp,
};

/// MIDI-related, extended [`DeviceDescriptor`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiDeviceDescriptor {
    pub device: DeviceDescriptor,
    pub port_name_prefix: &'static str,
}

#[derive(Debug, Error)]
pub enum MidiPortError {
    #[error("disconnected")]
    Disconnected,
    #[error(transparent)]
    Init(#[from] InitError),
    #[error(transparent)]
    ConnectInput(#[from] ConnectError<MidiInput>),
    #[error(transparent)]
    ConnectOutput(#[from] ConnectError<MidiOutput>),
}

impl From<SendError> for OutputError {
    fn from(err: SendError) -> Self {
        OutputError::Send {
            msg: err.to_string().into(),
        }
    }
}

pub trait MidiInputConnector: Send {
    /// Invoked before (re-)connecting the port.
    fn connect_midi_input_port(&mut self, device: &MidiDeviceDescriptor, port: &MidiPortDescriptor);
}

#[derive(Debug)]
pub struct MidiInputDecodeError;

pub trait MidiInputDecoder {
    /// Invoked for each incoming message.
    fn try_decode_midi_input(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError>;
}

impl<F> MidiInputDecoder for F
where
    F: FnMut(TimeStamp, &[u8]) -> Result<Option<ControlInputEvent>, MidiInputDecodeError>,
{
    fn try_decode_midi_input(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<ControlInputEvent>, MidiInputDecodeError> {
        self(ts, input)
    }
}

/// Passive callback for sinking MIDI input messages
pub trait MidiInputHandler: Send {
    /// Invoked for each incoming message.
    ///
    /// Returns `true` if the message has been accepted and handled
    /// or `false` otherwise.
    #[must_use]
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool;
}

impl<D> MidiInputHandler for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: MidiInputHandler,
{
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        self.deref_mut().handle_midi_input(ts, input)
    }
}

#[derive(Debug)]
pub struct MidiControlInputEventSink<D, E> {
    pub decoder: D,
    pub event_sink: E,
}

impl<D, E> MidiInputHandler for MidiControlInputEventSink<D, E>
where
    D: MidiInputDecoder + Send,
    E: ControlInputEventSink + Send,
{
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        match self.decoder.try_decode_midi_input(ts, input) {
            Ok(Some(event)) => {
                self.event_sink.sink_input_events(&[event]);
                true
            }
            Ok(None) => {
                log::debug!("[{ts}] No event for decoded MIDI input {input:x?}",);
                true
            }
            Err(MidiInputDecodeError) => {
                log::warn!("[{ts}] Failed to decode MIDI input {input:x?}");
                false
            }
        }
    }
}

pub trait MidiDeviceConnector {
    fn new_midi_input_receiver(
        &mut self,
        device_descriptor: &MidiDeviceDescriptor,
        _port_name: &str,
    ) -> Box<dyn MidiDevice>;
}

impl<D> MidiInputConnector for D
where
    D: DerefMut + Send,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPortDescriptor {
    pub index: PortIndex,
    pub name: Cow<'static, str>,
}

#[allow(missing_debug_implementations)]
pub struct MidirInputPort {
    pub descriptor: MidiPortDescriptor,
    pub port: MidiInputPort,
}

#[allow(missing_debug_implementations)]
pub struct MidirOutputPort {
    pub descriptor: MidiPortDescriptor,
    pub port: MidiOutputPort,
}

/// MIDI device driven by [`midir`].
#[allow(missing_debug_implementations)]
pub struct MidirDevice<I>
where
    I: MidiDevice + 'static,
{
    descriptor: MidiDeviceDescriptor,
    input_port: MidirInputPort,
    output_port: MidirOutputPort,
    input_connection: Option<MidiInputConnection<I>>,
}

impl<D> MidirDevice<D>
where
    D: MidiDevice,
{
    #[must_use]
    fn new(
        descriptor: MidiDeviceDescriptor,
        input_port: MidirInputPort,
        output_port: MidirOutputPort,
    ) -> Self {
        Self {
            descriptor,
            input_port,
            output_port,
            input_connection: None,
        }
    }

    #[must_use]
    pub fn descriptor(&self) -> &MidiDeviceDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn input_port(&self) -> &MidirInputPort {
        &self.input_port
    }

    #[must_use]
    pub fn output_port(&self) -> &MidirOutputPort {
        &self.output_port
    }

    #[must_use]
    pub fn is_available<U>(&self, device_manager: &MidirDeviceManager<U>) -> bool
    where
        U: MidiDevice,
    {
        device_manager
            .filter_input_ports_by_name(|port_name| port_name == self.input_port.descriptor.name)
            .next()
            .is_some()
            && device_manager
                .filter_output_ports_by_name(|port_name| {
                    port_name == self.output_port.descriptor.name
                })
                .next()
                .is_some()
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.input_connection.is_some()
    }

    pub fn reconnect<F>(
        &mut self,
        new_midi_device: Option<F>,
        output_connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, MidiPortError>
    where
        F: NewMidiDevice<MidiDevice = D>,
    {
        let input_connection = self.input_connection.take();
        debug_assert!(!self.is_connected());
        debug_assert_eq!(input_connection.is_some(), output_connection.is_some());
        let input_connection = self.reconnect_input(input_connection, new_midi_device)?;
        let output_connection = self.reconnect_output(output_connection)?;
        self.input_connection = Some(input_connection);
        debug_assert!(self.is_connected());
        Ok(output_connection)
    }

    pub fn disconnect(&mut self) {
        let Some(input_connection) = self
            .input_connection
            .take()
             else {
                return;
             };
        input_connection.close();
        debug_assert!(!self.is_connected());
    }

    fn reconnect_input<F>(
        &mut self,
        connection: Option<MidiInputConnection<D>>,
        new_midi_device: Option<F>,
    ) -> Result<MidiInputConnection<D>, MidiPortError>
    where
        F: NewMidiDevice<MidiDevice = D>,
    {
        let port_name = &self.input_port.descriptor.name;
        let (input, mut device) =
            if let Some((input, device)) = connection.map(MidiInputConnection::close) {
                (input, device)
            } else {
                let Some(mut new_midi_device) = new_midi_device else {
                    return Err(MidiPortError::Disconnected);
                };
                let input = MidiInput::new(port_name)?;
                let device =
                    new_midi_device.new_midi_device(&self.descriptor, &self.input_port.descriptor);
                (input, device)
            };
        device.connect_midi_input_port(&self.descriptor, &self.input_port.descriptor);
        input
            .connect(
                &self.input_port.port,
                port_name,
                move |micros, input, input_handler| {
                    let ts = TimeStamp::from_micros(micros);
                    log::debug!("[{ts}] Received MIDI input: {input:0x?}");
                    if !input_handler.handle_midi_input(ts, input) {
                        log::warn!("[{ts}] Unhandled MIDI input {input:x?}");
                    }
                },
                device,
            )
            .map_err(Into::into)
    }

    fn reconnect_output(
        &self,
        connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, MidiPortError> {
        let port_name = &self.output_port.descriptor.name;
        let output = match connection.map(MidiOutputConnection::close) {
            Some(output) => output,
            None => MidiOutput::new(port_name)?,
        };
        output
            .connect(&self.output_port.port, port_name)
            .map_err(Into::into)
    }
}

pub trait MidiDevice: MidiInputHandler + MidiInputConnector {}

impl<D> MidiDevice for D where D: MidiInputHandler + MidiInputConnector {}

pub type GenericMidirDevice = MidirDevice<Box<dyn MidiDevice>>;

pub trait NewMidiDevice {
    type MidiDevice: self::MidiDevice;

    fn new_midi_device(
        &mut self,
        device: &MidiDeviceDescriptor,
        input_port: &MidiPortDescriptor,
    ) -> Self::MidiDevice;
}

/// Identifies and connects [`MidirDevice`]s.
#[allow(missing_debug_implementations)]
pub struct MidirDeviceManager<I> {
    input: MidiInput,
    output: MidiOutput,
    _input_handler: PhantomData<I>,
}

impl<D> MidirDeviceManager<D>
where
    D: MidiDevice,
{
    pub fn new() -> Result<Self, midir::InitError> {
        let mut input = MidiInput::new("input port watcher")?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new("output port watcher")?;
        Ok(MidirDeviceManager {
            input,
            output,
            _input_handler: PhantomData,
        })
    }

    #[must_use]
    pub fn input_ports(&self) -> MidiInputPorts {
        self.input.ports()
    }

    #[must_use]
    pub fn output_ports(&self) -> MidiOutputPorts {
        self.output.ports()
    }

    pub fn filter_input_ports_by_name<'a>(
        &'a self,
        mut filter_port_name: impl FnMut(&str) -> bool + 'a,
    ) -> impl Iterator<Item = MidiInputPort> + 'a {
        self.input_ports().into_iter().filter(move |port| {
            self.input
                .port_name(port)
                .map_or(false, |port_name| filter_port_name(&port_name))
        })
    }

    pub fn filter_output_ports_by_name<'a>(
        &'a self,
        mut filter_port_name: impl FnMut(&str) -> bool + 'a,
    ) -> impl Iterator<Item = MidiOutputPort> + 'a {
        self.output_ports().into_iter().filter(move |port| {
            self.output
                .port_name(port)
                .map_or(false, |port_name| filter_port_name(&port_name))
        })
    }

    #[must_use]
    pub fn detect_dj_controllers(
        &self,
        device_descriptors: &[&MidiDeviceDescriptor],
        port_index_generator: &PortIndexGenerator,
    ) -> Vec<(MidiDeviceDescriptor, MidirDevice<D>)> {
        let mut input_ports = self
            .input_ports()
            .into_iter()
            .filter_map(|port| {
                let port_name = self.input.port_name(&port).ok()?;
                let Some(device_descriptor) = device_descriptors
                    .iter()
                    .copied()
                    .find(|device_descriptor| port_name.starts_with(device_descriptor.port_name_prefix)) else {
                    log::debug!("Input port \"{port_name}\" does not belong to a DJ controller");
                    return None;
                };
                log::debug!("Detected input port \"{port_name}\" for {device_descriptor:?}");
                Some((device_descriptor.port_name_prefix, (device_descriptor, port_name, port)))
            })
            .collect::<HashMap<_, _>>();
        let mut output_ports = self
            .output_ports()
            .into_iter()
            .filter_map(|port| {
                let port_name = self.output.port_name(&port).ok()?;
                let Some(port_name_prefix) = input_ports
                    .keys()
                    .copied()
                    .find(|port_name_prefix| port_name.starts_with(port_name_prefix)) else {
                        log::debug!("Output port \"{port_name}\" does not belong to a DJ controller");
                        return None;
                    };
                log::debug!("Detected output port \"{port_name}\" for DJ controller \"{port_name_prefix}\"");
                Some((port_name_prefix, (port_name, port)))
            })
            .collect::<HashMap<_, _>>();
        input_ports.retain(|key, _| output_ports.contains_key(key));
        debug_assert_eq!(input_ports.len(), output_ports.len());
        input_ports
            .into_iter()
            .map(
                |(port_name_prefix, (descriptor, input_port_name, input_port))| {
                    let (output_port_name, output_port) =
                        output_ports.remove(port_name_prefix).expect("Some");
                    log::debug!(
                        "Found DJ controller device \"{device_name}\" (input port: \
                         \"{input_port_name}\", output port: \"{output_port_name}\")",
                        device_name = descriptor.device.name()
                    );
                    let input_port = MidirInputPort {
                        descriptor: MidiPortDescriptor {
                            index: port_index_generator.next(),
                            name: input_port_name.into(),
                        },
                        port: input_port,
                    };
                    let output_port = MidirOutputPort {
                        descriptor: MidiPortDescriptor {
                            index: port_index_generator.next(),
                            name: output_port_name.into(),
                        },
                        port: output_port,
                    };
                    let device = MidirDevice::new(descriptor.clone(), input_port, output_port);
                    (descriptor.clone(), device)
                },
            )
            .collect()
    }
}

pub type GenericMidirDeviceManager = MidirDeviceManager<Box<dyn MidiDevice>>;
