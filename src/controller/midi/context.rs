// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use crate::BoxedMidiController;

struct AttachedMidiController<T> {
    controller: BoxedMidiController<T>,
    controller_thread: Option<crate::ControllerThread>,
}

#[expect(missing_debug_implementations)]
#[derive(Default)]
pub struct SingleMidiControllerContext<T> {
    attached: Option<AttachedMidiController<T>>,
}

impl<T: crate::ControllerTypes> SingleMidiControllerContext<T> {
    #[must_use]
    pub fn attached_controller(&self) -> Option<&BoxedMidiController<T>> {
        Some(&self.attached.as_ref()?.controller)
    }

    pub fn attach_controller(
        &mut self,
        controller: BoxedMidiController<T>,
        controller_task: Option<crate::BoxedControllerTask>,
    ) {
        if let Some(detached_controller) = self.detach_controller() {
            log::warn!(
                "Detached existing MIDI controller {descriptor:?}",
                descriptor = detached_controller.device_descriptor()
            );
        }
        log::info!(
            "Attaching MIDI controller {descriptor:?}",
            descriptor = controller.device_descriptor()
        );
        let controller_thread = controller_task.map(crate::ControllerThread::spawn);
        self.attached = Some(AttachedMidiController {
            controller,
            controller_thread,
        });
    }

    pub fn detach_controller(&mut self) -> Option<BoxedMidiController<T>> {
        let AttachedMidiController {
            controller_thread,
            controller,
        } = self.attached.take()?;
        log::info!(
            "Detaching MIDI controller {descriptor:?}",
            descriptor = controller.device_descriptor()
        );
        if let Some(controller_thread) = controller_thread {
            log::debug!(
                "Aborting MIDI controller thread for {descriptor:?}",
                descriptor = controller.device_descriptor()
            );
            if let Err(err) = controller_thread.abort_and_join() {
                log::warn!(
                    "Unexpected error while detaching MIDI controller {descriptor:?}: {err}",
                    descriptor = controller.device_descriptor()
                );
            }
        }
        Some(controller)
    }

    #[must_use]
    pub fn map_input_event(
        &mut self,
        event: <T as crate::ControllerTypes>::InputEvent,
    ) -> Option<<T as crate::ControllerTypes>::ControlAction> {
        let Some(attached) = &mut self.attached else {
            log::debug!("Ignoring input {event:?}: No MIDI controller attached");
            return None;
        };
        attached.controller.map_input_event(event)
    }
}
