// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::io::{stdin, stdout, Write as _};

use midir::MidiInputPort;

use djio::midi::{MidiDeviceManager, MidiInputHandler};

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

fn run() -> anyhow::Result<()> {
    let manager = MidiDeviceManager::new()?;
    let mut dj_controllers = manager.dj_controllers().collect::<Vec<_>>();
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
            let mut devices = manager.devices().collect::<Vec<_>>();
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

    device
        .reconnect(Some(LogMidiInput::default))
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("Starting endless loop, press CTRL-C to exit...");
    let mut last_state = None;
    loop {
        let current_state = manager.is_connected(device.port_name());
        if last_state != Some(current_state) {
            if current_state {
                println!("{}: connected", device.port_name());
                device
                    .reconnect(Some(LogMidiInput::default))
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
            } else {
                println!("{}: disconnected", device.port_name());
                device.disconnect();
            }
            last_state = Some(current_state);
        }
        // Re-check connectivity periodically every second
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
