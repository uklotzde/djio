// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    io::{stdin, stdout, Write as _},
    time::Duration,
};

use djio::{
    devices::{korg_kaoss_dj, MIDI_DJ_CONTROLLER_DESCRIPTORS},
    ControlInputEventSink, LedOutput, MidiDevice, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputDecodeError, MidiInputEventSink, MidiPortDescriptor, MidirDevice, MidirDeviceManager,
    PortIndex, PortIndexGenerator, TimeStamp,
};
use midir::MidiOutputConnection;

#[derive(Debug, Clone, Default)]
struct LogMidiInputEventSink {
    input_port: Option<MidiPortDescriptor>,
}

impl MidiInputConnector for LogMidiInputEventSink {
    fn connect_midi_input_port(
        &mut self,
        _device: &MidiDeviceDescriptor,
        input_port: &MidiPortDescriptor,
    ) {
        self.input_port = Some(input_port.to_owned());
    }
}

impl ControlInputEventSink for LogMidiInputEventSink {
    fn sink_input_events(&mut self, events: &[djio::ControlInputEvent]) {
        match &self.input_port {
            Some(input_port) => {
                for event in events {
                    log::info!("Received {event:?} from {input_port:?}");
                }
            }
            None => {
                for event in events {
                    log::error!("Received {event:?} from unknown MIDI device/port");
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct MidiInputEventDecoder {
    device: Option<MidiDeviceDescriptor>,
}

impl MidiInputConnector for MidiInputEventDecoder {
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        _input_port: &MidiPortDescriptor,
    ) {
        self.device = Some(device.to_owned());
    }
}

impl djio::MidiInputEventDecoder for MidiInputEventDecoder {
    fn try_decode_midi_input_event(
        &mut self,
        ts: TimeStamp,
        input: &[u8],
    ) -> Result<Option<djio::ControlInputEvent>, MidiInputDecodeError> {
        let Some(device) = &self.device else {
            log::error!("Cannot decode MIDI input from unknown/disconnected device: {ts} {input:x?}");
            return Err(MidiInputDecodeError);
        };
        if device == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            return korg_kaoss_dj::try_decode_midi_input_event(ts, input);
        }
        log::error!("Cannot decode MIDI input from unsupported device {device:?}: {ts} {input:x?}");
        Err(MidiInputDecodeError)
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
        _device: &MidiDeviceDescriptor,
        _input_port: &MidiPortDescriptor,
    ) -> Self::MidiDevice {
        Box::<MidiInputEventSink<MidiInputEventDecoder, LogMidiInputEventSink>>::default()
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
    let device_manager = MidirDeviceManager::<Box<dyn MidiDevice>>::new()?;
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
