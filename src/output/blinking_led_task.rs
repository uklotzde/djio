// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{future::Future, time::Duration};

use discro::Publisher;

use crate::{BlinkingLedOutput, BlinkingLedTicker};

#[allow(clippy::manual_async_fn)] // Explicit return type to to enforce the trait bounds
pub fn blinking_led_task(
    period: Duration,
    publisher: Publisher<BlinkingLedOutput>,
) -> impl Future<Output = ()> + Send + 'static {
    async move {
        let mut ticker = BlinkingLedTicker::default();
        let mut interval = tokio::time::interval(period);
        loop {
            // The first tick arrives immediately
            interval.tick().await;
            let output = ticker.tick();
            publisher.write(output);
        }
    }
}

#[cfg(feature = "blinking-led-task-tokio-rt")]
#[must_use]
pub fn spawn_blinking_led_task(period: Duration) -> discro::ReadOnlyPublisher<BlinkingLedOutput> {
    let publisher = Publisher::new(BlinkingLedOutput::ON);
    let ro_publisher = publisher.clone_read_only();
    let task = blinking_led_task(period, publisher);
    tokio::spawn(task);
    ro_publisher
}
