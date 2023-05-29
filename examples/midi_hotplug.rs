// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::io::{stdin, stdout, Write as _};

use djio::midi::{GenericMidiDeviceManager, MidiInputHandler};
use midir::MidiInputPort;

#[derive(Debug, Clone, Default)]
struct LogMidiInput {
    port_name: String,
}

impl MidiInputHandler for LogMidiInput {
    fn connect_midi_input_port(&mut self, port_name: &str, _port: &MidiInputPort) {
        if self.port_name.is_empty() {
            self.port_name = port_name.to_owned();
        } else {
            debug_assert_eq!(self.port_name, port_name);
        }
    }

    fn handle_midi_input(&mut self, stamp: u64, data: &[u8]) {
        println!(
            "{port_name}@{stamp}: {data:?} (len = {data_len})",
            port_name = self.port_name,
            data_len = data.len(),
        );
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
    let mut dj_controllers = device_manager.dj_controllers().collect::<Vec<_>>();
    let mut device = match dj_controllers.len() {
        0 => anyhow::bail!("no port found"),
        1 => {
            println!(
                "Choosing the only available DJ Controller: {port_name}",
                port_name = dj_controllers[0].port_name(),
            );
            dj_controllers.remove(0)
        }
        _ => {
            println!("\nAvailable devices:");
            let mut devices = device_manager.devices().collect::<Vec<_>>();
            for (i, device) in devices.iter().enumerate() {
                println!("{i}: {port_name}", port_name = device.port_name());
            }
            print!("Please select a device: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            devices.remove(input.trim().parse::<usize>()?)
        }
    };

    println!("{port_name}: connecting", port_name = device.port_name());
    device
        .reconnect(Some(new_input_handler))
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("Starting endless loop, press CTRL-C to exit...");
    loop {
        match (device.is_available(&device_manager), device.is_connected()) {
            (true, false) => {
                println!("{port_name}: reconnecting", port_name = device.port_name());
                device
                    .reconnect(Some(new_input_handler))
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
            }
            (false, true) => {
                println!("{port_name}: disconnecting", port_name = device.port_name());
                device.disconnect();
            }
            (false, false) => println!("{port_name}: disconnected", port_name = device.port_name()),
            (true, true) => println!("{port_name}: connected", port_name = device.port_name()),
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
