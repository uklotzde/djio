// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::io::{stdin, stdout, Write as _};

use djio::{
    devices::{korg_kaoss_dj, pioneer_ddj_400},
    EmitInputEvent, GenericMidiDeviceManager, MidiDevice, MidiDeviceDescriptor, MidiInputConnector,
    MidiInputHandler, MidirDevice, TimeStamp,
};
use midir::{MidiInputPort, MidiOutputConnection};

#[derive(Debug, Clone, Default)]
struct LogMidiInput {
    device_descriptor: Option<MidiDeviceDescriptor>,
    client_name: String,
}

impl MidiInputHandler for LogMidiInput {
    fn handle_midi_input(&mut self, ts: TimeStamp, input: &[u8]) {
        let device_descriptor = self.device_descriptor.as_ref().unwrap();
        let client_name = &self.client_name;
        if device_descriptor == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            if let Some(input) = korg_kaoss_dj::Input::try_from_midi_message(input) {
                println!("{client_name} @ {ts}: {input:?})");
                return;
            }
        }
        if device_descriptor == pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR {
            if let Some(input) = pioneer_ddj_400::Input::try_from_midi_message(input) {
                println!("{client_name} @ {ts}: {input:?})");
                return;
            }
        }
        println!(
            "{client_name} @ {ts}: {input:x?} (len = {input_len})",
            input_len = input.len()
        );
    }
}

impl MidiInputConnector for LogMidiInput {
    fn connect_midi_input_port(
        &mut self,
        device_descriptor: &MidiDeviceDescriptor,
        client_name: &str,
        port_name: &str,
        _port: &MidiInputPort,
    ) {
        println!("{client_name}: Connecting input port \"{port_name}\"");
        self.device_descriptor = Some(device_descriptor.to_owned());
        self.client_name = client_name.to_owned();
    }
}

struct KorgKaossDjLogInputEvent;
struct PioneerDdj400LogInputEvent;

type KorgKaossDjInputGateway = korg_kaoss_dj::InputGateway<KorgKaossDjLogInputEvent>;
type PioneerDdJ400InputGateway = pioneer_ddj_400::InputGateway<PioneerDdj400LogInputEvent>;

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

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

#[must_use]
fn new_midi_input_handler(device_descriptor: &MidiDeviceDescriptor) -> Box<dyn MidiDevice> {
    if device_descriptor == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
        Box::new(KorgKaossDjInputGateway::attach(KorgKaossDjLogInputEvent))
    } else if device_descriptor == pioneer_ddj_400::MIDI_DEVICE_DESCRIPTOR {
        Box::new(PioneerDdJ400InputGateway::attach(
            PioneerDdj400LogInputEvent,
        ))
    } else {
        Box::<LogMidiInput>::default()
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
        T: MidiInputHandler + MidiInputConnector,
    {
        if midi_device.descriptor() == korg_kaoss_dj::MIDI_DEVICE_DESCRIPTOR {
            let mut gateway = korg_kaoss_dj::OutputGateway::attach(midi_output_connection);
            // FIXME: Remove this test code
            gateway
                .send_deck_led_output(
                    korg_kaoss_dj::Deck::A,
                    korg_kaoss_dj::DeckLed::PlayPauseButton,
                    djio::LedOutput::On,
                )
                .unwrap();
            gateway
                .send_deck_led_output(
                    korg_kaoss_dj::Deck::B,
                    korg_kaoss_dj::DeckLed::PlayPauseButton,
                    djio::LedOutput::Off,
                )
                .unwrap();
            gateway
                .send_deck_led_output(
                    korg_kaoss_dj::Deck::A,
                    korg_kaoss_dj::DeckLed::CueButton,
                    djio::LedOutput::Off,
                )
                .unwrap();
            gateway
                .send_deck_led_output(
                    korg_kaoss_dj::Deck::B,
                    korg_kaoss_dj::DeckLed::CueButton,
                    djio::LedOutput::On,
                )
                .unwrap();
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

fn run() -> anyhow::Result<()> {
    let device_manager = GenericMidiDeviceManager::new()?;
    let mut dj_controllers = device_manager.detect_dj_controllers();
    let (_descriptor, mut device) = match dj_controllers.len() {
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

    let device_descriptor = device.descriptor().to_owned();
    let device_name = device_descriptor.device.name();
    println!("{device_name}: connecting");
    let midi_output_connection = device
        .reconnect(Some(|| new_midi_input_handler(&device_descriptor)), None)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut output_gateway = Some(OutputGateway::attach(&device, midi_output_connection));

    println!("Starting endless loop, press CTRL-C to exit...");
    loop {
        match (device.is_available(&device_manager), device.is_connected()) {
            (true, false) => {
                println!("{device_name}: Reconnecting");
                let midi_output_connection = output_gateway.take().map(OutputGateway::detach);
                let midi_output_connection = device
                    .reconnect(
                        Some(|| new_midi_input_handler(&device_descriptor)),
                        midi_output_connection,
                    )
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
                output_gateway = Some(OutputGateway::attach(&device, midi_output_connection));
            }
            (false, true) => {
                println!("{device_name}: Disconnecting");
                output_gateway
                    .take()
                    .map(OutputGateway::detach)
                    .map(MidiOutputConnection::close);
                device.disconnect();
            }
            (false, false) => println!("{device_name}: Disconnected"),
            (true, true) => println!("{device_name}: Connected"),
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
