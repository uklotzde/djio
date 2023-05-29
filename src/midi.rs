// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use midir::{
    ConnectError, Ignore, InitError, MidiInput, MidiInputConnection, MidiInputPort, MidiInputPorts,
    MidiOutput, MidiOutputConnection, MidiOutputPort, MidiOutputPorts,
};
use thiserror::Error;

// Predefined port names of existing DJ controllers for auto-detection.
//
// Should be extended as needed, preferably keeping the entries in lexicographical order.
const DJ_CONTROLLER_PORT_NAME_PREFIXES: &[&str] = &["DDJ-400", "KAOSS DJ"];

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
    fn connect_midi_input_port(&mut self, port_name: &str, port: &MidiInputPort);

    /// Invoked for each incoming message.
    fn handle_midi_input(&mut self, ts: u64, data: &[u8]);
}

impl<D> MidiInputHandler for D
where
    D: DerefMut + Send,
    <D as Deref>::Target: MidiInputHandler,
{
    fn connect_midi_input_port(&mut self, port_name: &str, port: &MidiInputPort) {
        self.deref_mut().connect_midi_input_port(port_name, port);
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
    port_name: String,
    input_port: MidiInputPort,
    output_port: MidiOutputPort,
    connection: Option<(MidiInputConnection<InputHandler>, MidiOutputConnection)>,
}

impl<InputHandler> MidiDevice<InputHandler>
where
    InputHandler: MidiInputHandler,
{
    #[must_use]
    pub fn new(port_name: String, input_port: MidiInputPort, output_port: MidiOutputPort) -> Self {
        Self {
            port_name,
            input_port,
            output_port,
            connection: None,
        }
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn is_available<T>(&self, device_manager: &MidiDeviceManager<T>) -> bool
    where
        T: MidiInputHandler,
    {
        device_manager
            .filter_input_ports_by_name(|port_name| port_name == self.port_name())
            .next()
            .is_some()
            && device_manager
                .filter_output_ports_by_name(|port_name| port_name == self.port_name())
                .next()
                .is_some()
    }

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
                let input = MidiInput::new(&self.port_name)?;
                let input_handler = new_input_handler();
                (input, input_handler)
            };
        input_handler.connect_midi_input_port(&self.port_name, &self.input_port);
        input
            .connect(
                &self.input_port,
                &self.port_name,
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
            None => MidiOutput::new(&self.port_name)?,
        };
        output
            .connect(&self.output_port, &self.port_name)
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

    pub fn input_ports(&self) -> MidiInputPorts {
        self.input.ports()
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

    pub fn output_ports(&self) -> MidiOutputPorts {
        self.output.ports()
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

    pub fn devices(&self) -> impl Iterator<Item = MidiDevice<InputHandler>> + '_ {
        self.input
            .ports()
            .into_iter()
            .filter_map(move |input_port| {
                let port_name = self.input.port_name(&input_port).ok()?;
                let output_port = {
                    let mut matching_output_port = None;
                    for output_port in self.output.ports() {
                        let Some(output_port_name) = self.output.port_name(&output_port).ok() else {
                            continue;
                        };
                        if output_port_name != port_name {
                            continue;
                        }
                        matching_output_port = Some(output_port);
                        break;
                    }
                    matching_output_port
                }?;
                let device = MidiDevice::new(port_name, input_port, output_port);
                println!(
                    "Found MIDI device {port_name}",
                    port_name = device.port_name()
                );
                Some(device)
            })
    }

    pub fn dj_controllers(&self) -> impl Iterator<Item = MidiDevice<InputHandler>> + '_ {
        self.devices().filter(|device| {
            DJ_CONTROLLER_PORT_NAME_PREFIXES
                .iter()
                .any(|prefix| device.port_name().starts_with(*prefix))
        })
    }
}

pub type GenericMidiDeviceManager = MidiDeviceManager<Box<dyn MidiInputHandler>>;
