// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    any::Any,
    mem::MaybeUninit,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use super::{HidDevice, HidDeviceError, HidError, HidResult};

#[derive(Debug, Clone, Copy)]
pub enum State {
    Starting,
    Running,
    Terminating,
}

/// Emitted event
///
/// Supposed to be consumed by a single receiver that could then
/// selectively dispatch them as needed to a broader audience.
#[expect(missing_debug_implementations)]
pub enum Event<'e> {
    StateChanged(State),
    ReportRead {
        data: &'e [u8],
    },
    ReportReadError(HidError),
    ReportWritten {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        buf_len: usize,
        bytes_written: usize,
    },
    ReportWriteError {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        buf_len: usize,
        err: HidError,
    },
    ReportWriteExpired {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        buf_len: usize,
        deadline: Instant,
    },
    FeatureReportRead {
        buf: Vec<u8>,
        buf_len: usize,
    },
    FeatureReportReadError {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        err: HidError,
    },
    FeatureReportWritten {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        buf_len: usize,
    },
    FeatureReportWriteError {
        /// Return buffer for recycling to minimize allocations
        buf: Vec<u8>,
        buf_len: usize,
        err: HidError,
    },
}

#[derive(Debug, Clone)]
pub enum Command {
    ReadFeatureReport {
        buf: Vec<u8>,
    },
    WriteFeatureReport {
        buf: Vec<u8>,
        buf_len: usize,
    },
    WriteReport {
        buf: Vec<u8>,
        buf_len: usize,
        deadline: Option<Instant>,
    },
    Terminate,
}

#[derive(Debug)]
pub struct CommandDisconnected;

pub type ReceiveCommandResult = std::result::Result<Option<Command>, CommandDisconnected>;

pub trait CommandReceiver {
    /// Receive command within the thread.
    ///
    /// Non-blocking receive (polling). The thread only blocks on hidapi
    /// read requests (intentionally) and during blocking hidapi write
    /// requests (unintentionally).
    fn try_recv_command(&mut self) -> ReceiveCommandResult;
}

pub trait EventHandler {
    /// Handle an event within the thread.
    ///
    /// This function is invoked in the thread context and should not block
    /// the worker thread for longer than needed!
    fn handle_event(&mut self, event: Event<'_>);
}

#[expect(missing_debug_implementations)]
pub struct HidThread<C: CommandReceiver + EventHandler> {
    join_handle: JoinHandle<Environment<C>>,
}

// 1 byte for the report identifier + a huge buffer size
// that is hopefully sufficient for all available devices.
//
// TODO: Allow to configure the buffer size at runtime?
const READ_BUFFER_SIZE: usize = 1 + 16384;

// hidapi only supports timeouts with millisecond precision.
const MIN_READ_TIMEOUT: Duration = Duration::from_millis(1); // 1 kHz

const FIRST_READ_TIMEOUT: Duration = MIN_READ_TIMEOUT;

// Prevent burning too much CPU if a device is not acting as expected.
// This is achieved by limiting the maximum polling frequency as defined
// by the corresponding minimum cycle time.
// Could be disabled by setting it to `Duration::ZERO`.
const MIN_CYCLE_TIME: Duration = Duration::from_micros(250); // 4 kHz

struct ReadSlot {
    buf: MaybeUninit<[u8; READ_BUFFER_SIZE]>,
    len: usize,
}

impl ReadSlot {
    const fn new() -> Self {
        Self {
            buf: MaybeUninit::uninit(),
            len: 0,
        }
    }
}

fn handle_command(device: &mut HidDevice, command: Command) -> Option<Event<'_>> {
    match command {
        Command::Terminate => None,
        Command::ReadFeatureReport { mut buf } => {
            debug_assert!(!buf.is_empty());
            match device.get_feature_report(&mut buf) {
                Ok(bytes_read) => Some(Event::FeatureReportRead {
                    buf,
                    buf_len: bytes_read,
                }),
                Err(err) => Some(Event::FeatureReportReadError { buf, err }),
            }
        }
        Command::WriteFeatureReport { buf, buf_len } => {
            debug_assert!(buf_len > 0);
            debug_assert!(buf_len <= buf.len());
            match device.send_feature_report(&buf[0..buf_len]) {
                Ok(()) => Some(Event::FeatureReportWritten { buf, buf_len }),
                Err(err) => Some(Event::FeatureReportWriteError { buf, buf_len, err }),
            }
        }
        Command::WriteReport {
            buf,
            buf_len,
            deadline,
        } => {
            debug_assert!(buf_len > 0);
            debug_assert!(buf_len <= buf.len());
            let expired = deadline.is_some_and(|deadline| deadline > Instant::now());
            if expired {
                debug_assert!(deadline.is_some());
                Some(Event::ReportWriteExpired {
                    buf,
                    buf_len,
                    deadline: deadline.unwrap(),
                })
            } else {
                match device.write(&buf[0..buf_len]) {
                    Ok(bytes_written) => Some(Event::ReportWritten {
                        buf,
                        buf_len,
                        bytes_written,
                    }),
                    Err(err) => Some(Event::ReportWriteError { buf, buf_len, err }),
                }
            }
        }
    }
}

#[expect(unsafe_code)]
fn thread_fn<C: CommandReceiver + EventHandler>(environment: &mut Environment<C>) {
    let Environment {
        connected_device: device,
        context,
    } = environment;
    // Double-buffering for deduplication of subsequent incoming reports
    let mut read_slots = vec![ReadSlot::new(), ReadSlot::new()];
    let mut last_read_slot_index = 0;
    let mut last_read_cycle_started = Instant::now();
    while let Ok(command) = context.try_recv_command() {
        // Handle a single command during each cycle.
        if let Some(command) = command {
            if let Some(event) = handle_command(device, command) {
                context.handle_event(event);
            } else {
                // Received a termination command
                break;
            }
        }
        // Each new cycle starts with a read request, even though command processing
        // is placed at the top of the loop body. This improves readability and only
        // affects the execution order of the initial cycle.
        let mut read_cycle_started = Instant::now();
        if !MIN_CYCLE_TIME.is_zero() {
            let earliest_next_read_cycle = last_read_cycle_started + MIN_CYCLE_TIME;
            while earliest_next_read_cycle > read_cycle_started {
                let sleep_duration = earliest_next_read_cycle.duration_since(read_cycle_started);
                log::trace!(
                    "Throttling: {millis:0.3} ms)",
                    millis = sleep_duration.as_secs_f64() * 1_000.0
                );
                std::thread::sleep(sleep_duration);
                // Update the time stamp after waking up
                read_cycle_started = Instant::now();
            }
        }
        // Consume all available reports.
        //
        // Only the first read request uses a timeout, all subsequent requests
        // will return immediately if no incoming reports are available and the
        // loop is exited.
        debug_assert!(read_cycle_started >= last_read_cycle_started);
        let elapsed_since_last_read_cycle =
            read_cycle_started.duration_since(last_read_cycle_started);
        let mut next_read_timeout = if FIRST_READ_TIMEOUT > elapsed_since_last_read_cycle {
            let next_read_timeout = FIRST_READ_TIMEOUT - elapsed_since_last_read_cycle;
            // Truncate to milliseconds as expected by hidapi
            #[expect(clippy::cast_possible_truncation)]
            if next_read_timeout < MIN_READ_TIMEOUT {
                // Ensure that the first timeout is not 0
                MIN_READ_TIMEOUT
            } else {
                Duration::from_millis(next_read_timeout.as_millis() as u64)
            }
        } else {
            Duration::ZERO
        };
        loop {
            let read_slot_index = (last_read_slot_index + 1) % read_slots.len();
            {
                let read_slot = unsafe { read_slots.get_unchecked_mut(read_slot_index) };
                let read_buf = unsafe { read_slot.buf.assume_init_mut() };
                let read_timeout = next_read_timeout;
                // Reset the timeout for all subsequent read requests.
                next_read_timeout = Duration::ZERO;
                let bytes_read = match device.read(read_buf, Some(read_timeout)) {
                    Ok(count) => count,
                    Err(err) => {
                        context.handle_event(Event::ReportReadError(err));
                        continue;
                    }
                };
                debug_assert!(bytes_read < READ_BUFFER_SIZE);
                if bytes_read > 0 {
                    read_slot.len = bytes_read;
                } else {
                    // No report received -> exit loop
                    break;
                }
            }
            let read_slot = unsafe { read_slots.get_unchecked(read_slot_index) };
            let read_buf = unsafe { read_slot.buf.assume_init() };
            debug_assert!(read_slot.len > 0);
            // Dedup subsequent reports with the same id and content,
            // i.e. consider them as idempotent.
            //
            // This simple dedup algorithm based on double-buffering is ineffective
            // for devices that send reports with alternating identifiers. However,
            // we are not aware of any HID devices  that send reports with alternating
            // identifiers at a high frequency.
            let last_read_slot = unsafe { read_slots.get_unchecked(last_read_slot_index) };
            if read_slot.len == last_read_slot.len {
                let last_read_buf = unsafe { last_read_slot.buf.assume_init() };
                if read_buf[..read_slot.len] == last_read_buf[..read_slot.len] {
                    log::trace!(
                        "Discarding duplicate report (id = {id}, len = {len})",
                        id = read_buf[0],
                        len = read_slot.len
                    );
                    continue;
                }
            }
            // Mark the read slot as occupied.
            last_read_slot_index = read_slot_index;
            last_read_cycle_started = read_cycle_started;
            // Consume the report.
            context.handle_event(Event::ReportRead {
                data: &read_buf[0..read_slot.len],
            });
        }
    }
    context.handle_event(Event::StateChanged(State::Terminating));
}

#[expect(missing_debug_implementations)]
pub struct Environment<C> {
    pub connected_device: HidDevice,

    pub context: C,
}

impl<C> HidThread<C>
where
    C: CommandReceiver + EventHandler + Send + 'static,
{
    pub fn spawn(environment: Environment<C>) -> HidResult<Self> {
        if !environment.connected_device.is_connected() {
            return Err(HidDeviceError::NotConnected.into());
        }
        let join_handle = std::thread::spawn(move || {
            let mut environment = environment;
            thread_fn(&mut environment);
            environment
        });
        log::debug!("Spawned thread: {join_handle:?}");
        Ok(Self { join_handle })
    }

    pub fn join(self) -> JoinedThread<C> {
        let Self { join_handle } = self;
        log::debug!("Joining thread: {join_handle:?}");
        join_handle
            .join()
            .map_or_else(JoinedThread::JoinError, |context| {
                JoinedThread::Terminated(TerminatedThread { context })
            })
    }
}

#[expect(missing_debug_implementations)]
pub struct TerminatedThread<C> {
    pub context: Environment<C>,
}

#[expect(missing_debug_implementations)]
pub enum JoinedThread<C> {
    Terminated(TerminatedThread<C>),
    JoinError(Box<dyn Any + Send + 'static>),
}
