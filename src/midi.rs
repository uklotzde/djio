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

use crate::{input::TimeStamp, OutputError};

#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub vendor_name: &'static str,
    pub model_name: &'static str,
    pub port_name_prefix: &'static str,
}

impl DeviceDescriptor {
    #[must_use]
    pub fn device_name(&self) -> Cow<'static, str> {
        let Self {
            vendor_name,
            model_name,
            ..
        } = *self;
        debug_assert!(!model_name.is_empty());
        if vendor_name.is_empty() {
            model_name.into()
        } else {
            format!("{vendor_name} {model_name}").into()
        }
    }
}

// Predefined port names of existing DJ controllers for auto-detection.
const DJ_CONTROLLER_DESCRIPTORS: &[DeviceDescriptor] = &[
    crate::devices::pioneer_ddj_400::DEVICE_DESCRIPTOR,
    crate::devices::korg_kaoss_dj::DEVICE_DESCRIPTOR,
];

#[derive(Debug, Error)]
pub enum PortError {
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

// Callbacks for handling MIDI input
pub trait InputHandler: Send {
    /// Invoked before (re-)connecting the port.
    fn connect_midi_input_port(&mut self, device_name: &str, port_name: &str, port: &MidiInputPort);

    /// Invoked for each incoming message.
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]);
}

impl<D> InputHandler for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: InputHandler,
{
    fn connect_midi_input_port(
        &mut self,
        device_name: &str,
        port_name: &str,
        port: &MidiInputPort,
    ) {
        self.deref_mut()
            .connect_midi_input_port(device_name, port_name, port);
    }

    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        self.deref_mut().handle_midi_input(ts, input);
    }
}

#[allow(missing_debug_implementations)]
pub struct MidiDevice<I>
where
    I: InputHandler + 'static,
{
    name: String,
    input_port_name: String,
    input_port: MidiInputPort,
    output_port_name: String,
    output_port: MidiOutputPort,
    input_connection: Option<MidiInputConnection<I>>,
}

impl<I> MidiDevice<I>
where
    I: InputHandler,
{
    #[must_use]
    fn new(name: String, input: (String, MidiInputPort), output: (String, MidiOutputPort)) -> Self {
        let (input_port_name, input_port) = input;
        let (output_port_name, output_port) = output;
        Self {
            name,
            input_port,
            input_port_name,
            output_port,
            output_port_name,
            input_connection: None,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
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
    pub fn is_available<U>(&self, device_manager: &MidiDeviceManager<U>) -> bool
    where
        U: InputHandler,
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

    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn reconnect(
        &mut self,
        new_input_handler: Option<impl FnOnce() -> I>,
        output_connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, PortError> {
        let input_connection = self.input_connection.take();
        debug_assert!(!self.is_connected());
        debug_assert_eq!(input_connection.is_some(), output_connection.is_some());
        let input_connection = self.reconnect_input(input_connection, new_input_handler)?;
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
        new_input_handler: Option<impl FnOnce() -> I>,
    ) -> Result<MidiInputConnection<I>, PortError> {
        let (input, mut input_handler) =
            if let Some((input, input_handler)) = connection.map(MidiInputConnection::close) {
                (input, input_handler)
            } else {
                let Some(new_input_handler) = new_input_handler else {
                    return Err(PortError::Disconnected);
                };
                let input = MidiInput::new(&self.name)?;
                let input_handler = new_input_handler();
                (input, input_handler)
            };
        input_handler.connect_midi_input_port(&self.name, &self.input_port_name, &self.input_port);
        input
            .connect(
                &self.input_port,
                &self.input_port_name,
                move |micros, message, input_handler| {
                    input_handler.handle_midi_input(TimeStamp::from_micros(micros), message);
                },
                input_handler,
            )
            .map_err(Into::into)
    }

    fn reconnect_output(
        &self,
        connection: Option<MidiOutputConnection>,
    ) -> Result<MidiOutputConnection, PortError> {
        let output = match connection.map(MidiOutputConnection::close) {
            Some(output) => output,
            None => MidiOutput::new(&self.name)?,
        };
        output
            .connect(&self.output_port, &self.name)
            .map_err(Into::into)
    }
}

pub type GenericMidiDevice = MidiDevice<Box<dyn InputHandler>>;

#[allow(missing_debug_implementations)]
pub struct MidiDeviceManager<I> {
    input: MidiInput,
    output: MidiOutput,
    _input_handler: PhantomData<I>,
}

impl<I> MidiDeviceManager<I>
where
    I: InputHandler,
{
    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn new() -> Result<Self, midir::InitError> {
        let mut input = MidiInput::new("input port watcher")?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new("output port watcher")?;
        Ok(MidiDeviceManager {
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
    pub fn detect_dj_controllers(&self) -> Vec<(DeviceDescriptor, MidiDevice<I>)> {
        let mut input_ports = self
            .input_ports()
            .into_iter()
            .filter_map(|port| {
                let port_name = self.input.port_name(&port).ok()?;
                let Some(descriptor) = DJ_CONTROLLER_DESCRIPTORS
                    .iter()
                    .find(|descriptor| port_name.starts_with(descriptor.port_name_prefix)) else {
                    log::debug!("Input port \"{port_name}\" does not belong to a DJ controller");
                    return None;
                };
                log::debug!("Detected input port \"{port_name}\" for {descriptor:?}");
                Some((descriptor.port_name_prefix, (descriptor, port_name, port)))
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
                    let device_name = descriptor.device_name().into_owned();
                    log::debug!(
                        "Found DJ controller device \"{device_name}\" (input port: \
                         \"{input_port_name}\", output port: \"{output_port_name}\")"
                    );
                    let device = MidiDevice::new(
                        device_name,
                        (input_port_name, input_port),
                        (output_port_name, output_port),
                    );
                    (descriptor.clone(), device)
                },
            )
            .collect()
    }
}

pub type GenericMidiDeviceManager = MidiDeviceManager<Box<dyn InputHandler>>;
