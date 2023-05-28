// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::marker::PhantomData;

use midir::{
    ConnectError, Ignore, InitError, MidiInput, MidiInputConnection, MidiInputPort, MidiOutput,
    MidiOutputConnection, MidiOutputPort,
};
use thiserror::Error;

const DJ_CONTROLLER_PORT_NAME_PREFIXES: &[&str] = &["KAOSS DJ:KAOSS DJ KAOSS DJ _ SOUND"];

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

pub trait MidiInputHandler {
    /// Invoked before (re-)connecting the port.
    fn connect_midi_input_port(&mut self, port_name: &str, port: &MidiInputPort);

    /// Invoked for each incoming message.
    fn handle_midi_input(&mut self, ts: u64, data: &[u8]);
}

#[allow(missing_debug_implementations)]
pub struct MidiDevice<InputHandler>
where
    InputHandler: MidiInputHandler + 'static,
{
    port_name: String,
    input_port: MidiInputPort,
    output_port: MidiOutputPort,
    input_connection: Option<MidiInputConnection<InputHandler>>,
    output_connection: Option<MidiOutputConnection>,
}

impl<InputHandler> MidiDevice<InputHandler>
where
    InputHandler: MidiInputHandler + Send,
{
    #[must_use]
    pub fn new(port_name: String, input_port: MidiInputPort, output_port: MidiOutputPort) -> Self {
        Self {
            port_name,
            input_port,
            output_port,
            input_connection: None,
            output_connection: None,
        }
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    #[allow(clippy::missing_errors_doc)] // FIXME
    pub fn reconnect(
        &mut self,
        new_input_handler: Option<impl FnOnce() -> InputHandler>,
    ) -> Result<(), PortError> {
        self.reconnect_input(new_input_handler)?;
        self.reconnect_output()?;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.disconnect_input();
        self.disconnect_output();
    }

    fn reconnect_input(
        &mut self,
        new_input_handler: Option<impl FnOnce() -> InputHandler>,
    ) -> Result<(), PortError> {
        let (input, mut input_handler) =
            if let Some((input, input_handler)) = self.disconnect_input() {
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
        let input_connection = input.connect(
            &self.input_port,
            &self.port_name,
            move |stamp, message, input_handler| input_handler.handle_midi_input(stamp, message),
            input_handler,
        )?;
        self.input_connection = Some(input_connection);
        Ok(())
    }

    fn disconnect_input(&mut self) -> Option<(MidiInput, InputHandler)> {
        self.input_connection.take().map(MidiInputConnection::close)
    }

    fn reconnect_output(&mut self) -> Result<(), PortError> {
        let output = match self.disconnect_output() {
            Some(output) => output,
            None => MidiOutput::new(&self.port_name)?,
        };
        let output_connection = output.connect(&self.output_port, &self.port_name)?;
        self.output_connection = Some(output_connection);
        Ok(())
    }

    fn disconnect_output(&mut self) -> Option<MidiOutput> {
        self.output_connection
            .take()
            .map(MidiOutputConnection::close)
    }
}

#[allow(missing_debug_implementations)]
pub struct MidiDeviceManager<InputHandler> {
    input: MidiInput,
    output: MidiOutput,
    _input_handler: PhantomData<InputHandler>,
}

impl<InputHandler> MidiDeviceManager<InputHandler>
where
    InputHandler: MidiInputHandler + Send,
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

    fn input_port(&self, port_name: &str) -> Option<MidiInputPort> {
        self.input.ports().into_iter().find(|port| {
            self.input
                .port_name(port)
                .map_or(false, |name| name == port_name)
        })
    }

    fn output_port(&self, port_name: &str) -> Option<MidiOutputPort> {
        self.output.ports().into_iter().find(|port| {
            self.output
                .port_name(port)
                .map_or(false, |name| name == port_name)
        })
    }

    pub fn is_connected(&self, port_name: &str) -> bool {
        self.input_port(port_name).is_some() && self.output_port(port_name).is_some()
    }
}
