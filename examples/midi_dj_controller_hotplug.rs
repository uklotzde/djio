// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    io::{stdin, stdout, Write as _},
    time::Duration,
};

use djio::{
    devices::{denon_dj_mc6000mk2, korg_kaoss_dj, pioneer_ddj_400, MIDI_DJ_CONTROLLER_DESCRIPTORS},
    ControlInputEventSink, EmitInputEvent, GenericMidirDeviceManager, LedOutput, MidiDevice,
    MidiDeviceDescriptor, MidiInputConnector, MidiInputHandler, MidiPortDescriptor, MidirDevice,
    PortIndex, PortIndexGenerator, TimeStamp,
};
use midir::MidiOutputConnection;

#[derive(Debug, Clone, Default)]
struct LogMidiInput {
    connected: Option<(MidiDeviceDescriptor, MidiPortDescriptor)>,
}

impl MidiInputHandler for LogMidiInput {
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        let Some((device, port)) = &self.connected else {
            return false;
        };
        if device == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            if let Some(input) = korg_kaoss_dj::Input::try_from_midi_input(input) {
                println!("{port:?} @ {ts}: {input:?})");
                return true;
            }
        }
        if device == pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR {
            if let Some(input) = pioneer_ddj_400::Input::try_from_midi_input(input) {
                println!("{port:?} @ {ts}: {input:?})");
                return true;
            }
        }
        println!(
            "{port:?} @ {ts}: {input:x?} (len = {input_len})",
            input_len = input.len()
        );
        true
    }
}

impl MidiInputConnector for LogMidiInput {
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        port: &MidiPortDescriptor,
    ) {
        log::info!("Device \"{device:?}\" is connected to port \"{port:?}\"");
        self.connected = Some((device.to_owned(), port.to_owned()));
    }
}

struct KorgKaossDjLogInputEvent;
struct PioneerDdj400LogInputEvent;
struct DenonDjMc6000Mk2LogInputEvent;

type KorgKaossDjInputGateway = korg_kaoss_dj::InputGateway<KorgKaossDjLogInputEvent>;
type PioneerDdJ400InputGateway = pioneer_ddj_400::InputGateway<PioneerDdj400LogInputEvent>;
type DenonDjMc6000Mk2InputGateway = denon_dj_mc6000mk2::InputGateway<DenonDjMc6000Mk2LogInputEvent>;

impl EmitInputEvent<korg_kaoss_dj::Input> for KorgKaossDjLogInputEvent {
    fn emit_input_event(&mut self, event: korg_kaoss_dj::InputEvent) {
        println!("Received input {event:?}");
    }
}

impl EmitInputEvent<pioneer_ddj_400::Input> for PioneerDdj400LogInputEvent {
    fn emit_input_event(&mut self, event: pioneer_ddj_400::InputEvent) {
        println!("Received input {event:?}");
    }
}

impl EmitInputEvent<denon_dj_mc6000mk2::Input> for DenonDjMc6000Mk2LogInputEvent {
    fn emit_input_event(&mut self, event: denon_dj_mc6000mk2::InputEvent) {
        println!("Received input {event:?}");
    }
}

fn main() {
    pretty_env_logger::init();

    match run() {
        Ok(_) => (),
        Err(err) => log::error!("{err}"),
    }
}

struct NewMidiDevice;

impl djio::NewMidiDevice for NewMidiDevice {
    type MidiDevice = Box<dyn MidiDevice>;

    fn new_midi_device(
        &self,
        device: &MidiDeviceDescriptor,
        _input_port: &MidiPortDescriptor,
    ) -> Self::MidiDevice {
        if device == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            Box::new(KorgKaossDjInputGateway::attach(KorgKaossDjLogInputEvent))
        } else if device == pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR {
            Box::new(PioneerDdJ400InputGateway::attach(
                PioneerDdj400LogInputEvent,
            ))
        } else if device == denon_dj_mc6000mk2::MIDI_DEVICE_DESCRIPTOR {
            Box::new(DenonDjMc6000Mk2InputGateway::attach(
                DenonDjMc6000Mk2LogInputEvent,
            ))
        } else {
            Box::<LogMidiInput>::default()
        }
    }
}

enum OutputGateway {
    KorgKaossDj {
        gateway: korg_kaoss_dj::OutputGateway,
    },
    Generic {
        midi_output_connection: MidiOutputConnection,
    },
}

impl OutputGateway {
    #[must_use]
    fn attach<T>(midi_device: &MidirDevice<T>, midi_output_connection: MidiOutputConnection) -> Self
    where
        T: MidiDevice,
    {
        if midi_device.descriptor() == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            let mut gateway = korg_kaoss_dj::OutputGateway::attach(midi_output_connection);
            // Simple LED animation
            gateway
                .send_all_led_outputs(LedOutput::Off, Duration::ZERO)
                .unwrap();
            gateway
                .send_all_led_outputs(LedOutput::On, Duration::from_millis(50))
                .unwrap();
            // Reset all
            gateway.reset_all_leds().unwrap();
            Self::KorgKaossDj { gateway }
        } else {
            Self::Generic {
                midi_output_connection,
            }
        }
    }

    #[must_use]
    fn detach(self) -> MidiOutputConnection {
        match self {
            Self::KorgKaossDj { gateway } => gateway.detach(),
            Self::Generic {
                midi_output_connection,
            } => midi_output_connection,
        }
    }
}

#[derive(Debug, Clone)]
struct LoggingInputPortEventSink {
    pub port_index: PortIndex,
}

impl ControlInputEventSink for LoggingInputPortEventSink {
    fn sink_input_events(&mut self, events: &[djio::ControlInputEvent]) {
        log::info!(
            "Received {num_events} input event(s) from port {port_index}: {events:?}",
            num_events = events.len(),
            port_index = self.port_index,
        );
    }
}

fn run() -> anyhow::Result<()> {
    let port_index_generator = PortIndexGenerator::new();
    let device_manager = GenericMidirDeviceManager::new()?;
    let mut dj_controllers =
        device_manager.detect_dj_controllers(MIDI_DJ_CONTROLLER_DESCRIPTORS, &port_index_generator);
    let (_descriptor, mut midir_device) = match dj_controllers.len() {
        0 => anyhow::bail!("No supported DJ controllers found"),
        1 => {
            println!(
                "Choosing the only available DJ Controller: {device_name}",
                device_name = dj_controllers[0].1.descriptor().device.name(),
            );
            dj_controllers.remove(0)
        }
        _ => {
            println!("\nAvailable devices:");
            for (i, (_descriptor, device)) in dj_controllers.iter().enumerate() {
                println!(
                    "{device_number}: {device_name}",
                    device_number = i + 1,
                    device_name = device.descriptor().device.name()
                );
            }
            print!("Please select a device: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            let device_number = input.trim().parse::<usize>()?;
            if device_number < 1 || device_number > dj_controllers.len() {
                eprintln!("Unknown device number {device_number}");
                return Ok(());
            }
            dj_controllers.remove(device_number - 1)
        }
    };

    let device_name = midir_device.descriptor().device.name();
    println!("{device_name}: connecting");
    let midi_output_connection = midir_device
        .reconnect(Some(NewMidiDevice), None)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut output_gateway = Some(OutputGateway::attach(&midir_device, midi_output_connection));

    println!("Starting endless loop, press CTRL-C to exit...");
    loop {
        match (
            midir_device.is_available(&device_manager),
            midir_device.is_connected(),
        ) {
            (true, false) => {
                println!("{device_name}: Reconnecting");
                let midi_output_connection = output_gateway.take().map(OutputGateway::detach);
                let midi_output_connection = midir_device
                    .reconnect(Some(NewMidiDevice), midi_output_connection)
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
                output_gateway = Some(OutputGateway::attach(&midir_device, midi_output_connection));
            }
            (false, true) => {
                println!("{device_name}: Disconnecting");
                output_gateway
                    .take()
                    .map(OutputGateway::detach)
                    .map(MidiOutputConnection::close);
                midir_device.disconnect();
            }
            (false, false) => println!("{device_name}: Disconnected"),
            (true, true) => println!("{device_name}: Connected"),
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
