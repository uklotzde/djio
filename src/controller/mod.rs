// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;

use crate::{DeviceDescriptor, PortIndex};

#[cfg(feature = "midi")]
pub(super) mod midi;

#[cfg(feature = "controller-thread")]
pub(super) mod thread;

/// Asynchronous context listener task.
///
/// Listens for changes in [`ControllerTypes::Context`] and updates
/// the corresponding hardware state accordingly, e.g. LEDs, screens,
/// or motorized jog wheels and faders.
pub type BoxedControllerTask = Box<dyn Future<Output = ()> + Send + 'static>;

pub trait ControllerTypes {
    type Context;
    type InputEvent: std::fmt::Debug;
    type ControlAction;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDescriptor {
    /// Number of physical decks
    pub num_decks: u8,

    /// Number of virtual decks
    pub num_virtual_decks: u8,

    /// Number of mixer channels
    ///
    /// Usually equals the number of virtual decks.
    pub num_mixer_channels: u8,

    /// Number of pads per deck
    ///
    /// Pads could either be multi-purpose performance pads or dedicated hot cue pads.
    pub num_pads_per_deck: u8,

    /// Number of effect units
    pub num_effect_units: u8,
}

pub trait Controller {
    type Types: ControllerTypes;

    /// Device descriptor
    ///
    /// Each controller instance must always return the same descriptor
    /// during its lifetime!
    #[must_use]
    fn device_descriptor(&self) -> DeviceDescriptor;

    /// Controller descriptor
    ///
    /// Each controller instance must always return the same descriptor
    /// during its lifetime!
    #[must_use]
    fn controller_descriptor(&self) -> ControllerDescriptor;

    /// Attach a context listener task.
    ///
    /// Invoked once when the controller is connected. Only needs to return `Some`
    /// when invoked for the first time. For subsequent invocations, `None` should
    /// be returned. This allows to conveniently consume resources that are required
    /// for setting up the task by using [`Option::take()`].
    ///
    /// Stateless controllers may return `None`.
    #[must_use]
    fn attach_context_listener(
        &mut self,
        context: &<Self::Types as ControllerTypes>::Context,
    ) -> Option<BoxedControllerTask>;

    /// Input port index
    ///
    /// Only needs to be implemented for controllers that generate input events.
    /// The default implementation returns [`PortIndex::INVALID`].
    #[must_use]
    fn input_port_index(&self) -> PortIndex {
        PortIndex::INVALID
    }

    /// Map a generic input event into a control action.
    ///
    /// Invoked when an input event is received from the hardware sensors,
    /// e.g. a MIDI message.
    ///
    /// Each input event induces 0 or 1 control action(s). If the input event
    /// is unsupported and should be ignored or if it only affects the internal
    /// state then `None` could be returned.
    ///
    /// Only needs to be implemented for controllers that generate input events.
    /// The default implementation ignores the event and returns `None`.
    #[must_use]
    fn map_input_event(
        &mut self,
        event: <Self::Types as ControllerTypes>::InputEvent,
    ) -> Option<<Self::Types as ControllerTypes>::ControlAction> {
        log::debug!("Unmapped input event: {event:?}");
        None
    }
}
