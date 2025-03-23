// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use hidapi::DeviceInfo;
use smol_str::SmolStr;

use crate::{
    AudioInterfaceDescriptor, ControllerDescriptor, DeviceDescriptor, HidDevice, HidDeviceError,
    HidResult, HidThread,
    hid::{
        report::BufferRecycler,
        thread::{
            Command, CommandDisconnected, CommandReceiver, Environment, Event, EventHandler,
            JoinedThread, ReceiveCommandResult,
        },
    },
};

pub const AUDIO_INTERFACE_DESCRIPTOR: AudioInterfaceDescriptor = AudioInterfaceDescriptor {
    num_input_channels: 0, // TODO
    num_output_channels: 4,
};

pub const DEVICE_DESCRIPTOR: &DeviceDescriptor = &DeviceDescriptor {
    vendor_name: SmolStr::new_static("Native Instruments"),
    product_name: SmolStr::new_static("TRAKTOR KONTROL S4MK3"),
    audio_interface: Some(AUDIO_INTERFACE_DESCRIPTOR),
};

pub const CONTROLLER_DESCRIPTOR: ControllerDescriptor = ControllerDescriptor {
    num_decks: 2,
    num_virtual_decks: 4,
    num_mixer_channels: 4,
    num_pads_per_deck: 8,
    num_effect_units: 2,
};

#[derive(Debug, Clone, Default)]
struct ReportStats {
    count: usize,
    last_instant: Option<Instant>,
    max_duration_since_last_instant: Option<Duration>,
}

impl ReportStats {
    #[must_use]
    fn update(&mut self, instant: Instant) -> (usize, Option<Duration>) {
        self.count = self.count.checked_add(1).unwrap();
        let duration_since_last_instant = self
            .last_instant
            .map(|last_instant| instant.duration_since(last_instant));
        self.last_instant = Some(instant);
        self.max_duration_since_last_instant =
            duration_since_last_instant.map(|duration_since_last_instant| {
                if let Some(max_duration_since_last_instant) = self.max_duration_since_last_instant
                {
                    max_duration_since_last_instant.max(duration_since_last_instant)
                } else {
                    duration_since_last_instant
                }
            });
        (self.count, duration_since_last_instant)
    }
}

struct ThreadContext {
    command_rx: mpsc::Receiver<Command>,
    recycle_report_buffer_tx: mpsc::Sender<Vec<u8>>,
    report_stats_by_id: Vec<ReportStats>,
}

impl ThreadContext {
    fn recycle_report_buffer(&self, buf: Vec<u8>) {
        if let Err(err) = self.recycle_report_buffer_tx.send(buf) {
            // Should never happen
            log::error!(
                "Failed to submit buffer for recycling: {buf:?}",
                buf = err.0
            );
        }
    }
}

impl CommandReceiver for ThreadContext {
    fn try_recv_command(&mut self) -> ReceiveCommandResult {
        match self.command_rx.try_recv() {
            Ok(command) => Ok(Some(command)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(CommandDisconnected),
        }
    }
}

impl EventHandler for ThreadContext {
    fn handle_event(&mut self, event: Event<'_>) {
        match event {
            Event::StateChanged(state) => {
                log::info!("Thread state changed: {state:?}");
            }
            Event::FeatureReportRead { buf, buf_len } => {
                log::info!(
                    "TODO: Handle feature report: {data:?}",
                    data = &buf[..buf_len]
                );
            }
            Event::FeatureReportReadError { buf: _, err } => {
                log::warn!("Failed to read feature report: {err}");
            }
            Event::ReportRead { data } => {
                let report_id = data[0];
                let report_stats = self
                    .report_stats_by_id
                    .get_mut(usize::from(report_id))
                    .unwrap();
                let (_count, duration_since_last_report) = report_stats.update(Instant::now());
                log::info!(
                    "TODO: Handle report{stats_suffix}: {data:?}",
                    stats_suffix = duration_since_last_report
                        .map(|duration| {
                            format!(
                                " (\u{0394} = {millis:0.3} ms)",
                                millis = duration.as_secs_f64() * 1_000.0
                            )
                        })
                        .unwrap_or_default()
                );
            }
            Event::ReportReadError(err) => {
                log::warn!("Failed to read report: {err}");
            }
            Event::ReportWritten {
                buf,
                buf_len: _,
                bytes_written: _,
            } => {
                self.recycle_report_buffer(buf);
            }
            Event::FeatureReportWritten { buf: _, buf_len: _ } => {
                // Buffers of feature reports are not recycled
            }
            Event::ReportWriteError {
                buf: _,
                buf_len: _,
                err,
            } => {
                log::error!("Failed to write report: {err}");
                // Buffers of feature reports are not recycled
            }
            Event::ReportWriteExpired {
                buf,
                buf_len: _,
                deadline: _,
            } => {
                log::warn!("Deadline for writing report expired");
                self.recycle_report_buffer(buf);
            }
            Event::FeatureReportWriteError {
                buf,
                buf_len: _,
                err,
            } => {
                log::error!("Failed to write feature report: {err}");
                self.recycle_report_buffer(buf);
            }
        }
    }
}

#[expect(missing_debug_implementations)]
pub struct DeviceContext {
    info: DeviceInfo,
    thread: HidThread<ThreadContext>,
    command_tx: mpsc::Sender<Command>,
    recycle_report_buffer_rx: mpsc::Receiver<Vec<u8>>,
    report_buffer_recycler: BufferRecycler,
}

impl DeviceContext {
    #[must_use]
    pub const fn vendor_id() -> u16 {
        0x17cc
    }

    #[must_use]
    pub const fn product_id() -> u16 {
        0x1720
    }

    #[must_use]
    pub fn is_supported(device_info: &DeviceInfo) -> bool {
        device_info.vendor_id() == Self::vendor_id()
            && device_info.product_id() == Self::product_id()
    }

    pub fn attach(connected_device: HidDevice) -> HidResult<DeviceContext> {
        if !Self::is_supported(connected_device.info()) {
            return Err(HidDeviceError::NotSupported.into());
        }
        if !connected_device.is_connected() {
            return Err(HidDeviceError::NotConnected.into());
        }
        let (command_tx, command_rx) = mpsc::channel::<Command>();
        let (recycle_report_buffer_tx, recycle_report_buffer_rx) = mpsc::channel::<Vec<u8>>();
        let thread_context = ThreadContext {
            command_rx,
            recycle_report_buffer_tx,
            // One slot per report id
            report_stats_by_id: std::iter::repeat(ReportStats::default())
                .take(usize::from(u8::MAX) + 1)
                .collect(),
        };
        let info = connected_device.info().clone();
        let environment = Environment {
            connected_device,
            context: thread_context,
        };
        log::info!("Spawning HID I/O thread");
        let thread = HidThread::spawn(environment)?;
        Ok(DeviceContext {
            info,
            thread,
            command_tx,
            recycle_report_buffer_rx,
            report_buffer_recycler: BufferRecycler::new(),
        })
    }

    #[expect(clippy::missing_panics_doc)] // Never panics
    pub fn detach(self) -> HidResult<HidDevice> {
        log::info!("Terminating I/O thread");
        self.command_tx
            .send(Command::Terminate)
            .expect("command channel to I/O thread closed unexpectedly");
        log::info!("Joining I/O thread");
        let joined_thread = self.thread.join();
        match joined_thread {
            JoinedThread::Terminated(terminated_thread) => {
                // The device is still connected after the thread terminated.
                let connected_device = terminated_thread.context.connected_device;
                debug_assert!(connected_device.is_connected());
                Ok(connected_device)
            }
            JoinedThread::JoinError(err) => {
                Err(anyhow::anyhow!("Joining the I/O thread failed: {err:?}").into())
            }
        }
    }

    #[must_use]
    pub const fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Initialization sequence
    ///
    /// Should be invoked once after attaching the device.
    ///
    /// Reverse-engineered from Traktor Pro.
    pub fn initialize(&mut self) {
        // Send the initializing reports for both wheels 0/1.
        // This increases the frequency of report 3 by decreasing the cycle time
        // from ~250 ms when inactive down to ~2 ms when active. It is also
        // required for the jog wheel LEDs to work.
        let mut data = [0; 27];
        data[0] = 48; // report id
        debug_assert_eq!(data[1], 0); // wheel 0
        data[2] = 1;
        data[3] = 3;
        self.write_report(&data);
        data[1] = 1; // wheel 1
        self.write_report(&data);
    }

    /// Finalization sequence
    ///
    /// Should be invoked once before detaching the device.
    ///
    /// Reverse-engineered from Traktor Pro.
    pub fn finalize(&mut self) {
        // Turn off button LEDs.
        let mut data = [0; 95];
        data[0] = 128; // report id
        self.write_report(&data);
        // Turn off meter LEDs.
        let mut data = [0; 79];
        data[0] = 129; // report id
        self.write_report(&data);
        // Send the finalizing reports for both wheels 0/1.
        let mut data = [0; 41];
        data[0] = 50; // report id
        debug_assert_eq!(data[1], 0); // wheel 0
        self.write_report(&data);
        data[1] = 1; // wheel 1
        self.write_report(&data);
    }

    /// Recycle queued buffers on demand.
    ///
    /// Could be invoked periodically during idle times before actually
    /// writing the next report. Avoids delaying the write request that
    /// would otherwise first recycle all queued buffers by invoking this
    /// function.
    pub fn recycle_queued_buffers(&mut self) {
        while let Some(buf) = match self.recycle_report_buffer_rx.try_recv() {
            Ok(buf) => Some(buf),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                // Should never happen during regular operation
                log::warn!("Failed to receive recycled buffer from i/o thread");
                None
            }
        } {
            self.report_buffer_recycler.recycle_buf(buf);
        }
    }

    pub fn write_report(&mut self, data: &[u8]) {
        self.recycle_queued_buffers();
        let buf = self.report_buffer_recycler.fill_buf(data);
        let buf_len = buf.len();
        let cmd = Command::WriteReport {
            buf,
            buf_len,
            deadline: None,
        };
        self.submit_command(cmd);
    }

    pub fn submit_command(&self, cmd: Command) {
        if let Err(err) = self.command_tx.send(cmd) {
            // Should never happen during regular operation
            log::warn!("Failed to submit command: {cmd:?}", cmd = err.0);
        }
    }
}
