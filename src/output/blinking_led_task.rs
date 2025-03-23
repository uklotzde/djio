// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{future::Future, time::Duration};

use discro::Publisher;

use crate::{BlinkingLedOutput, BlinkingLedTicker};

#[expect(clippy::manual_async_fn)] // Explicit return type to to enforce the trait bounds
pub fn blinking_led_task(
    period: Duration,
    publisher: Publisher<BlinkingLedOutput>,
) -> impl Future<Output = ()> + Send + 'static {
    async move {
        let mut ticker = BlinkingLedTicker::default();
        let mut interval = tokio::time::interval(period);
        // Unlikely that a tick is missed. If it happens, then simply delay the next tick.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            // The first tick arrives immediately
            interval.tick().await;
            let output = ticker.tick();
            publisher.write(output);
        }
    }
}

/// Spawn a task that periodically emits a blinking LED trigger.
///
/// Needed to synchronize the frequencies of all blinking LEDs.
///
/// Returns a subscriber for receiving update triggers. The initial
/// update is available immediately and emitted as a change notification.
#[cfg(feature = "blinking-led-task-tokio-rt")]
#[must_use]
pub fn spawn_blinking_led_task(period: Duration) -> discro::Subscriber<BlinkingLedOutput> {
    let publisher = Publisher::new(BlinkingLedOutput::ON);
    // Mark the subscriber as changed on subscription for emitting
    // an initial change notification immediately.
    let subscriber = publisher.subscribe_changed();
    let task = blinking_led_task(period, publisher);
    tokio::spawn(task);
    subscriber
}
