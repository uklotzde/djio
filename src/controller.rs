// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;

/// Asynchronous context listener task.
///
/// Listens for changes in [`Controller::Context`] and updates the
/// corresponding hardware state accordingly, e.g. LEDs, screens,
/// or motorized jog wheels and faders.
pub type BoxedControllerTask = Box<dyn Future<Output = ()> + Send + 'static>;

pub trait Controller {
    type Context;
    type InputEvent;
    type ControlAction;

    /// Attach a context listener task.
    ///
    /// Invoked once when the controller is connected. Only needs to return `Some`
    /// when invoked for the first time. For subsequent invocations, `None` should
    /// be returned. This allows to conveniently consume resources that are required
    /// for setting up the task by using [`Option::take()`].
    ///
    /// Controllers that don't need a task may return `None`.
    #[must_use]
    fn attach_context_listener(&mut self, context: &Self::Context) -> Option<BoxedControllerTask>;

    /// Map a generic input event into a control action.
    ///
    /// Invoked when an input event is received from the hardware sensors,
    /// e.g. a MIDI message.
    ///
    /// Each input event induces 0 or 1 control action(s). If the input event
    /// is unsupported and should be ignored or if it only affects the internal
    /// state then `None` could be returned.
    #[must_use]
    fn map_input_event(&mut self, event: Self::InputEvent) -> Option<Self::ControlAction>;
}
