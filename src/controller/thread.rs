// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use futures_util::future::{AbortHandle, Abortable, Aborted};

use super::BoxedControllerTask;

/// Dedicated thread for each controller.
///
/// Each controller gets its own thread to avoid blocking other controllers.
#[derive(Debug)]
pub struct ControllerThread {
    abort_handle: AbortHandle,
    os_thread: std::thread::JoinHandle<()>,
}

impl ControllerThread {
    #[must_use]
    pub fn spawn(controller_task: BoxedControllerTask) -> Self {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let abortable_task = Abortable::new(Box::into_pin(controller_task), abort_registration);
        let os_thread = std::thread::spawn(move || {
            log::info!("Entering controller thread");
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    log::error!("Failed to create Tokio runtime: {err}");
                    return;
                }
            };
            runtime.block_on(async move {
                log::info!("Running controller task");
                match abortable_task.await {
                    Ok(()) => {
                        log::info!("Controller task terminated");
                    }
                    Err(Aborted) => {
                        log::info!("Controller task aborted");
                    }
                }
            });
            log::info!("Exiting context listener thread");
        });
        Self {
            abort_handle,
            os_thread,
        }
    }

    pub fn abort_and_join(self) -> anyhow::Result<()> {
        let Self {
            abort_handle,
            os_thread,
        } = self;
        abort_handle.abort();
        os_thread
            .join()
            .map_err(|err| anyhow::anyhow!("Context listener thread panicked: {err:?}"))
    }
}
