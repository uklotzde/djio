// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
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
    ControlInputEvent, DeviceDescriptor, InputEventReceiver, OutputError, PortIndex, TimeStamp,
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

pub trait MidirInputConnector: Send {
    /// Invoked before (re-)connecting the port.
    fn connect_midi_input_port(
        &mut self,
        device_descriptor: &MidiDeviceDescriptor,
        client_name: &str,
        port_name: &str,
        port: &MidiInputPort,
    );
}

pub trait MidiInputDecoder {
    /// Invoked for each incoming message.
    fn try_decode_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> Option<ControlInputEvent>;
}

impl<F> MidiInputDecoder for F
where
    F: FnMut(TimeStamp, &[u8]) -> Option<ControlInputEvent>,
{
    fn try_decode_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> Option<ControlInputEvent> {
        self(ts, input)
    }
}

/// Passive callback for receiving MIDI input messages
pub trait MidiInputReceiver: Send {
    /// Invoked for each incoming message.
    fn recv_midi_input(&mut self, ts: TimeStamp, input: &[u8]);
}

impl<D> MidiInputReceiver for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: MidiInputReceiver,
{
    fn recv_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        self.deref_mut().recv_midi_input(ts, input);
    }
}

#[allow(missing_debug_implementations)]
pub struct MidiInputPortConnector {
    pub device_descriptor: MidiDeviceDescriptor,
    pub decoder: Box<dyn MidiInputDecoder>,
}

#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct MidiInputEventGateway<E> {
    event_receiver: E,
    next_port_index: PortIndex,
    port_connectors: HashMap<PortIndex, MidiInputPortConnector>,
}

#[allow(missing_debug_implementations)]
pub struct MidiInputPortConnectError(MidiInputPortConnector);

impl<E> MidiInputEventGateway<E>
where
    E: InputEventReceiver,
{
    #[must_use]
    pub fn new(event_receiver: E) -> Self {
        Self {
            event_receiver,
            next_port_index: PortIndex::FIRST,
            port_connectors: HashMap::new(),
        }
    }

    pub fn connect_port(
        &mut self,
        connector: MidiInputPortConnector,
    ) -> Result<PortIndex, MidiInputPortConnectError> {
        let port_index = self.next_port_index;
        if self.is_port_connected(port_index) {
            return Err(MidiInputPortConnectError(connector));
        }
        self.port_connectors.insert(port_index, connector);
        self.next_port_index = port_index.next();
        Ok(port_index)
    }

    pub fn disconnect_port(&mut self, port_index: PortIndex) -> Option<MidiInputPortConnector> {
        self.port_connectors.remove(&port_index)
    }

    #[must_use]
    pub fn is_port_connected(&self, port_index: PortIndex) -> bool {
        self.port_connectors.contains_key(&port_index)
    }

    pub fn recv_midi_input(&mut self, port_index: PortIndex, ts: TimeStamp, input: &[u8]) -> bool {
        let Some(port_connector) = self.port_connectors.get_mut(&port_index) else {
            log::warn!("[{ts}] Discarding MIDI input {input:x?} from disconnected port {port_index}");
            return false;
        };
        let Some(event) = port_connector.decoder.try_decode_midi_input(ts, input) else {
            log::debug!("[{ts}] Discarding undecoded MIDI input {input:x?} from port {port_index}");
            return false;
        };
        self.event_receiver.recv_input_events(port_index, &[event]);
        true
    }
}

impl<D> MidirInputConnector for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: MidirInputConnector,
{
    fn connect_midi_input_port(
        &mut self,
        device_descriptor: &MidiDeviceDescriptor,
        client_name: &str,
        port_name: &str,
        port: &MidiInputPort,
    ) {
        self.deref_mut()
            .connect_midi_input_port(device_descriptor, client_name, port_name, port);
    }
}

/// MIDI device driven by [`midir`].
#[allow(missing_debug_implementations)]
pub struct MidirDevice<I>
where
    I: MidiInputReceiver + MidirInputConnector + 'static,
{
    descriptor: MidiDeviceDescriptor,
    input_port_name: String,
    input_port: MidiInputPort,
    output_port_name: String,
    output_port: MidiOutputPort,
    input_connection: Option<MidiInputConnection<I>>,
}

impl<I> MidirDevice<I>
where
    I: MidiDevice,
{
    #[must_use]
    fn new(
        descriptor: MidiDeviceDescriptor,
        input: (String, MidiInputPort),
        output: (String, MidiOutputPort),
    ) -> Self {
        let (input_port_name, input_port) = input;
        let (output_port_name, output_port) = output;
        Self {
            descriptor,
            input_port,
            input_port_name,
            output_port,
            output_port_name,
            input_connection: None,
        }
    }

    #[must_use]
    pub fn descriptor(&self) -> &MidiDeviceDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn input_port_name(&self) -> &str {
        &self.input_port_name
    }

    #[must_use]
    pub fn output_port_name(&self) -> &str {
        &self.output_port_name
    }

    #[must_use]
    pub fn is_available<U>(&self, device_manager: &MidirDeviceManager<U>) -> bool
    where
        U: MidiInputReceiver + MidirInputConnector,
    {
        device_manager
            .filter_input_ports_by_name(|port_name| port_name == self.input_port_name)
            .next()
            .is_some()
            && device_manager
                .filter_output_ports_by_name(|port_name| port_name == self.output_port_name)
                .next()
                .is_some()
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.input_connection.is_some()
    }

    pub fn reconnect(
        &mut self,
        new_input_receiver: Option<impl FnOnce() -> I>,
        output_connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, MidiPortError> {
        let input_connection = self.input_connection.take();
        debug_assert!(!self.is_connected());
        debug_assert_eq!(input_connection.is_some(), output_connection.is_some());
        let input_connection = self.reconnect_input(input_connection, new_input_receiver)?;
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

    fn reconnect_input(
        &mut self,
        connection: Option<MidiInputConnection<I>>,
        new_input_receiver: Option<impl FnOnce() -> I>,
    ) -> Result<MidiInputConnection<I>, MidiPortError> {
        let client_name = self.descriptor.device.name();
        let (input, mut input_rx) =
            if let Some((input, input_rx)) = connection.map(MidiInputConnection::close) {
                (input, input_rx)
            } else {
                let Some(new_input_receiver) = new_input_receiver else {
                    return Err(MidiPortError::Disconnected);
                };
                let input = MidiInput::new(&client_name)?;
                let input_rx = new_input_receiver();
                (input, input_rx)
            };
        input_rx.connect_midi_input_port(
            &self.descriptor,
            &client_name,
            &self.input_port_name,
            &self.input_port,
        );
        input
            .connect(
                &self.input_port,
                &self.input_port_name,
                move |micros, input, input_rx| {
                    let ts = TimeStamp::from_micros(micros);
                    log::debug!("[{ts}] Received MIDI input: {input:0x?}");
                    input_rx.recv_midi_input(ts, input);
                },
                input_rx,
            )
            .map_err(Into::into)
    }

    fn reconnect_output(
        &self,
        connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, MidiPortError> {
        let client_name = self.descriptor.device.name();
        let output = match connection.map(MidiOutputConnection::close) {
            Some(output) => output,
            None => MidiOutput::new(&client_name)?,
        };
        output
            .connect(&self.output_port, &client_name)
            .map_err(Into::into)
    }
}

pub trait MidiDevice: MidiInputReceiver + MidirInputConnector {}

impl<I> MidiDevice for I where I: MidiInputReceiver + MidirInputConnector {}

pub type GenericMidiDevice = MidirDevice<Box<dyn MidiDevice>>;

/// Identifies and connects [`MidirDevice`]s.
#[allow(missing_debug_implementations)]
pub struct MidirDeviceManager<I> {
    input: MidiInput,
    output: MidiOutput,
    _input_rx: PhantomData<I>,
}

impl<I> MidirDeviceManager<I>
where
    I: MidiInputReceiver + MidirInputConnector,
{
    pub fn new() -> Result<Self, midir::InitError> {
        let mut input = MidiInput::new("input port watcher")?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new("output port watcher")?;
        Ok(MidirDeviceManager {
            input,
            output,
            _input_rx: PhantomData,
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
    ) -> Vec<(MidiDeviceDescriptor, MidirDevice<I>)> {
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
                    let device = MidirDevice::new(
                        descriptor.clone(),
                        (input_port_name, input_port),
                        (output_port_name, output_port),
                    );
                    (descriptor.clone(), device)
                },
            )
            .collect()
    }
}

pub type GenericMidirDeviceManager = MidirDeviceManager<Box<dyn MidiDevice>>;
