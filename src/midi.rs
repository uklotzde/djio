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
    MidiOutput, MidiOutputConnection, MidiOutputPort, MidiOutputPorts,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DjControllerDescriptor {
    pub vendor_name: &'static str,
    pub model_name: &'static str,
    pub port_name_prefix: &'static str,
}

impl DjControllerDescriptor {
    fn device_name(&self) -> Cow<'static, str> {
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
const DJ_CONTROLLER_DESCRIPTORS: &[DjControllerDescriptor] = &[
    DjControllerDescriptor {
        vendor_name: "Pioneer",
        model_name: "DDJ-400",
        port_name_prefix: "DDJ-400",
    },
    DjControllerDescriptor {
        vendor_name: "Korg",
        model_name: "KAOSS DJ",
        port_name_prefix: "KAOSS DJ",
    },
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

// Callbacks for handling MIDI input
pub trait MidiInputHandler: Send {
    /// Invoked before (re-)connecting the port.
    fn connect_midi_input_port(&mut self, device_name: &str, port_name: &str, port: &MidiInputPort);

    /// Invoked for each incoming message.
    fn handle_midi_input(&mut self, ts: u64, data: &[u8]);
}

impl<D> MidiInputHandler for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: MidiInputHandler,
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

    fn handle_midi_input(&mut self, ts: u64, data: &[u8]) {
        self.deref_mut().handle_midi_input(ts, data);
    }
}

#[allow(missing_debug_implementations)]
pub struct MidiDevice<InputHandler>
where
    InputHandler: MidiInputHandler + 'static,
{
    name: String,
    input_port_name: String,
    input_port: MidiInputPort,
    output_port_name: String,
    output_port: MidiOutputPort,
    connection: Option<(MidiInputConnection<InputHandler>, MidiOutputConnection)>,
}

impl<InputHandler> MidiDevice<InputHandler>
where
    InputHandler: MidiInputHandler,
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
            connection: None,
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
    pub fn is_available<T>(&self, device_manager: &MidiDeviceManager<T>) -> bool
    where
        T: MidiInputHandler,
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
        self.connection.is_some()
    }

    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn reconnect(
        &mut self,
        new_input_handler: Option<impl FnOnce() -> InputHandler>,
    ) -> Result<(), PortError> {
        let (input_conn, output_conn) = self
            .connection
            .take()
            .map_or((None, None), |(input_conn, output_conn)| {
                (Some(input_conn), Some(output_conn))
            });
        debug_assert!(!self.is_connected());
        let input_conn = self.reconnect_input(input_conn, new_input_handler)?;
        let output_conn = self.reconnect_output(output_conn)?;
        self.connection = Some((input_conn, output_conn));
        debug_assert!(self.is_connected());
        Ok(())
    }

    pub fn disconnect(&mut self) {
        let Some((input_conn, output_conn)) = self
            .connection
            .take()
             else {
                return;
             };
        input_conn.close();
        output_conn.close();
        debug_assert!(!self.is_connected());
    }

    fn reconnect_input(
        &mut self,
        connection: Option<MidiInputConnection<InputHandler>>,
        new_input_handler: Option<impl FnOnce() -> InputHandler>,
    ) -> Result<MidiInputConnection<InputHandler>, PortError> {
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
                move |stamp, message, input_handler| {
                    input_handler.handle_midi_input(stamp, message);
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

pub type GenericMidiDevice = MidiDevice<Box<dyn MidiInputHandler>>;

#[allow(missing_debug_implementations)]
pub struct MidiDeviceManager<InputHandler> {
    input: MidiInput,
    output: MidiOutput,
    _input_handler: PhantomData<InputHandler>,
}

impl<InputHandler> MidiDeviceManager<InputHandler>
where
    InputHandler: MidiInputHandler,
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
    pub fn detect_dj_controllers(&self) -> Vec<(DjControllerDescriptor, MidiDevice<InputHandler>)> {
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

pub type GenericMidiDeviceManager = MidiDeviceManager<Box<dyn MidiInputHandler>>;
