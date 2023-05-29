// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::io::{stdin, stdout, Write as _};

use djio::{
    input,
    midi::{GenericMidiDeviceManager, MidiInputHandler},
};
use midir::MidiInputPort;

#[derive(Debug, Clone, Default)]
struct LogMidiInput {
    device_name: String,
}

impl MidiInputHandler for LogMidiInput {
    fn connect_midi_input_port(
        &mut self,
        device_name: &str,
        port_name: &str,
        _port: &MidiInputPort,
    ) {
        println!("{device_name}: Connecting input port \"{port_name}\"");
        if self.device_name.is_empty() {
            self.device_name = device_name.to_owned();
        } else {
            debug_assert_eq!(self.device_name, device_name);
        }
    }

    fn handle_midi_input(&mut self, stamp: u64, data: &[u8]) {
        if let Some(event) = input::mapping::korg_kaoss_dj::InputEvent::try_from_midi_message(data)
        {
            println!(
                "{device_name}@{stamp}: {event:?})",
                device_name = self.device_name,
            );
        } else {
            println!(
                "{device_name}@{stamp}: {data:?} (len = {data_len})",
                device_name = self.device_name,
                data_len = data.len(),
            );
        }
    }
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

fn new_input_handler() -> Box<dyn MidiInputHandler> {
    Box::<LogMidiInput>::default()
}

fn run() -> anyhow::Result<()> {
    let device_manager = GenericMidiDeviceManager::new()?;
    let mut dj_controllers = device_manager.detect_dj_controllers();
    let (_descriptor, mut device) = match dj_controllers.len() {
        0 => anyhow::bail!("No supported DJ controllers found"),
        1 => {
            println!(
                "Choosing the only available DJ Controller: {device_name}",
                device_name = dj_controllers[0].1.name(),
            );
            dj_controllers.remove(0)
        }
        _ => {
            println!("\nAvailable devices:");
            for (i, (_descriptor, device)) in dj_controllers.iter().enumerate() {
                println!(
                    "{device_number}: {device_name}",
                    device_number = i + 1,
                    device_name = device.name()
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

    println!("{device_name}: connecting", device_name = device.name());
    device
        .reconnect(Some(new_input_handler))
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("Starting endless loop, press CTRL-C to exit...");
    loop {
        match (device.is_available(&device_manager), device.is_connected()) {
            (true, false) => {
                println!("{device_name}: Reconnecting", device_name = device.name());
                device
                    .reconnect(Some(new_input_handler))
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
            }
            (false, true) => {
                println!("{device_name}: Disconnecting", device_name = device.name());
                device.disconnect();
            }
            (false, false) => println!("{device_name}: Disconnected", device_name = device.name()),
            (true, true) => println!("{device_name}: Connected", device_name = device.name()),
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
