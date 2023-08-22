// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    io::{stdin, stdout, Write as _},
    time::Duration,
};

use djio::{
    consume_midi_input_event,
    devices::{korg_kaoss_dj, pioneer_ddj_400, MIDI_DJ_CONTROLLER_DESCRIPTORS},
    BoxedMidiOutputConnection, ControlInputEventSink, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputEventDecoder, MidiInputGateway, MidiInputHandler, MidiOutputGateway,
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
struct MidiLogger {
    decoder: Option<Box<dyn MidiInputEventDecoder + Send>>,
    event_sink: LogMidiInputEventSink,
}

impl MidiInputConnector for MidiLogger {
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

impl MidiInputHandler for MidiLogger {
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
    type MidiInputGateway = MidiLogger;

    fn new_midi_input_gateway(
        &self,
        _device: &MidiDeviceDescriptor,
        _input_port: &MidiPortDescriptor,
    ) -> Self::MidiInputGateway {
        Default::default()
    }
}

trait Controller {
    // ...
}

trait MidiController: Controller + MidiOutputGateway<BoxedMidiOutputConnection> {}

impl<T> MidiController for T where T: Controller + MidiOutputGateway<BoxedMidiOutputConnection> {}

#[derive(Default)]
struct KorgKaossDj {
    output_gateway: Option<korg_kaoss_dj::OutputGateway<BoxedMidiOutputConnection>>,
}

impl Controller for KorgKaossDj {}

impl MidiOutputGateway<BoxedMidiOutputConnection> for KorgKaossDj {
    fn attach_midi_output_connection(
        &mut self,
        connection: &mut Option<BoxedMidiOutputConnection>,
    ) -> OutputResult<()> {
        debug_assert!(self.output_gateway.is_none());
        let mut output_gateway =
            korg_kaoss_dj::OutputGateway::<BoxedMidiOutputConnection>::default();
        output_gateway.attach_midi_output_connection(connection)?;
        self.output_gateway = Some(output_gateway);
        Ok(())
    }

    fn detach_midi_output_connection(&mut self) -> Option<BoxedMidiOutputConnection> {
        self.output_gateway
            .take()
            .and_then(|mut output_gateway| output_gateway.detach_midi_output_connection())
    }
}

fn new_midi_controller<I>(
    device: &MidirDevice<I>,
    output_connection: &mut Option<BoxedMidiOutputConnection>,
) -> OutputResult<Option<Box<dyn MidiController>>>
where
    I: MidiInputGateway + Send,
{
    let mut controller: Box<dyn MidiController> =
        if device.descriptor() == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            Box::<KorgKaossDj>::default() as _
        } else {
            return Ok(None);
        };
    controller.attach_midi_output_connection(output_connection)?;
    Ok(Some(controller))
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
    _detached_output_connection: Option<BoxedMidiOutputConnection>,
) -> anyhow::Result<Option<Box<dyn MidiController>>>
where
    I: djio::NewMidiInputGateway,
    I::MidiInputGateway: Send,
{
    // Unfortunately, we cannot recover and reuse the wrapped `midir::MidiOutputConnection`
    // from the detached output connection and instead must create a new one.
    let reusable_output_connection = None;
    let output_connection = device
        .reconnect(new_input_gateway, reusable_output_connection)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut output_connection = Some(Box::new(output_connection) as _);
    new_midi_controller(device, &mut output_connection).map_err(|err| anyhow::anyhow!("{err}"))
}

fn disconnect_midi_controller(
    device: &mut MidirDevice<MidiLogger>,
    mut controller: Option<Box<dyn MidiController>>,
) -> Option<BoxedMidiOutputConnection> {
    let detached_output_connection = controller
        .as_mut()
        .and_then(|boxed| boxed.as_mut().detach_midi_output_connection());
    device.disconnect();
    detached_output_connection
}

// Controls the frequency of the polling thread while the device is connected.
const CONNECTED_DEVICE_SLEEP_DURATION: Duration = Duration::from_millis(1000);

// Poll more frequently while the device is not connected to reconnect it
// promptly when it becomes available again.
const DISCONNECTED_DEVICE_SLEEP_DURATION: Duration = Duration::from_millis(250);

fn run() -> anyhow::Result<()> {
    let port_index_generator = PortIndexGenerator::new();
    let device_manager = MidirDeviceManager::<MidiLogger>::new()?;
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

    println!("Starting endless loop, press CTRL-C to exit...");
    let mut output_connection: Option<BoxedMidiOutputConnection> = None;
    let mut controller: Option<Box<dyn MidiController>> = None;
    loop {
        match (
            midir_device.is_available(&device_manager),
            midir_device.is_connected(),
        ) {
            (true, false) => {
                println!("{device_name}: Connecting");
                controller = reconnect_midi_controller(
                    &mut midir_device,
                    new_midi_input_gateway.as_ref(),
                    output_connection.take(),
                )?;
            }
            (false, true) => {
                println!("{device_name}: Disconnecting");
                disconnect_midi_controller(&mut midir_device, controller.take());
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
