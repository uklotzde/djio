// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    io::{stdin, stdout, Write as _},
    time::Duration,
};

use djio::{
    consume_midi_input_event,
    devices::{denon_dj_mc6000mk2, korg_kaoss_dj, pioneer_ddj_400, MIDI_DJ_CONTROLLER_DESCRIPTORS},
    ControlInputEventSink, MidiControlOutputGateway, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputEventDecoder, MidiInputGateway, MidiInputHandler, MidiOutputConnection,
    MidiPortDescriptor, MidirDevice, MidirDeviceManager, OutputResult, PortIndex,
    PortIndexGenerator, TimeStamp,
};

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

#[derive(Default)]
struct MidiController {
    decoder: Option<Box<dyn MidiInputEventDecoder + Send>>,
    event_sink: LogMidiInputEventSink,
}

impl MidiInputConnector for MidiController {
    fn connect_midi_input_port(
        &mut self,
        device: &MidiDeviceDescriptor,
        input_port: &MidiPortDescriptor,
    ) {
        self.decoder = if device == pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR {
            Some(Box::<pioneer_ddj_400::MidiInputEventDecoder>::default())
        } else if device == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            Some(Box::<korg_kaoss_dj::MidiInputEventDecoder>::default())
        } else {
            log::warn!("Unsupported device: {device:?}");
            None
        };
        self.event_sink.connect_midi_input_port(device, input_port);
    }
}

impl MidiInputHandler for MidiController {
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) -> bool {
        let Some(decoder) = &mut self.decoder else {
            return false;
        };
        consume_midi_input_event(ts, input, decoder.as_mut(), &mut self.event_sink)
    }
}

fn main() {
    pretty_env_logger::init();

    match run() {
        Ok(_) => (),
        Err(err) => log::error!("{err}"),
    }
}

struct NewMidiInputGateway;

impl djio::NewMidiInputGateway for NewMidiInputGateway {
    type MidiInputGateway = MidiController;

    fn new_midi_input_gateway(
        &self,
        _device: &MidiDeviceDescriptor,
        _input_port: &MidiPortDescriptor,
    ) -> Self::MidiInputGateway {
        Default::default()
    }
}

type DynMidiOutputConnection = dyn MidiOutputConnection;
type DynMidiControlOutputGateway = dyn MidiControlOutputGateway<Box<DynMidiOutputConnection>>;

fn new_midi_control_output_gateway<I>(
    midi_device: &MidirDevice<I>,
    midi_output_connection: &mut Option<Box<DynMidiOutputConnection>>,
) -> OutputResult<Option<Box<DynMidiControlOutputGateway>>>
where
    I: MidiInputGateway + Send,
{
    let mut output_gateway: Box<DynMidiControlOutputGateway> =
        if midi_device.descriptor() == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            Box::<korg_kaoss_dj::OutputGateway<Box<DynMidiOutputConnection>>>::default() as _
        } else if midi_device.descriptor() == denon_dj_mc6000mk2::MIDI_DEVICE_DESCRIPTOR {
            Box::<denon_dj_mc6000mk2::OutputGateway<Box<DynMidiOutputConnection>>>::default() as _
        } else {
            return Ok(None);
        };
    output_gateway.attach_midi_output_connection(midi_output_connection)?;
    Ok(Some(output_gateway))
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

fn reconnect_midi_controller<I>(
    device: &mut MidirDevice<I::MidiInputGateway>,
    new_input_gateway: Option<&I>,
    mut output_gateway: Option<Box<DynMidiControlOutputGateway>>,
) -> anyhow::Result<Option<Box<DynMidiControlOutputGateway>>>
where
    I: djio::NewMidiInputGateway,
    I::MidiInputGateway: Send,
{
    let _detached_output_connection = output_gateway
        .as_mut()
        .and_then(|boxed| boxed.as_mut().detach_midi_output_connection());
    let output_connection = device
        .reconnect(new_input_gateway, None)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut output_connection = Some(Box::new(output_connection) as _);
    let output_gateway = new_midi_control_output_gateway(device, &mut output_connection)?;
    Ok(output_gateway)
}

fn disconnect_midi_controller(
    device: &mut MidirDevice<MidiController>,
    mut output_gateway: Option<Box<DynMidiControlOutputGateway>>,
) {
    let _detached_output_connection = output_gateway
        .as_mut()
        .and_then(|boxed| boxed.as_mut().detach_midi_output_connection());
    device.disconnect();
}

// Controls the frequency of the polling thread while the device is connected.
const CONNECTED_DEVICE_SLEEP_DURATION: Duration = Duration::from_millis(1000);

// Poll more frequently while the device is not connected to reconnect it
// promptly when it becomes available again.
const DISCONNECTED_DEVICE_SLEEP_DURATION: Duration = Duration::from_millis(250);

fn run() -> anyhow::Result<()> {
    let port_index_generator = PortIndexGenerator::new();
    let device_manager = MidirDeviceManager::<MidiController>::new()?;
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

    let new_midi_input_gateway = Some(NewMidiInputGateway);

    let device_name = midir_device.descriptor().device.name();
    println!("{device_name}: Connecting");
    let mut output_gateway =
        reconnect_midi_controller(&mut midir_device, new_midi_input_gateway.as_ref(), None)?;

    println!("Starting endless loop, press CTRL-C to exit...");
    loop {
        match (
            midir_device.is_available(&device_manager),
            midir_device.is_connected(),
        ) {
            (true, false) => {
                println!("{device_name}: Reconnecting");
                output_gateway = reconnect_midi_controller(
                    &mut midir_device,
                    new_midi_input_gateway.as_ref(),
                    output_gateway.take(),
                )?;
            }
            (false, true) => {
                println!("{device_name}: Disconnecting");
                disconnect_midi_controller(&mut midir_device, output_gateway.take());
                midir_device.disconnect();
            }
            (false, false) => {
                println!("{device_name}: Disconnected");
                std::thread::sleep(DISCONNECTED_DEVICE_SLEEP_DURATION);
            }
            (true, true) => {
                println!("{device_name}: Connected");
                std::thread::sleep(CONNECTED_DEVICE_SLEEP_DURATION);
            }
        }
    }
}
