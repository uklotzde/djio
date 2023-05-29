// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

pub mod mapping;

/// A simple two-state button.
#[derive(Debug, Clone, Copy)]
pub enum Button {
    Pressed,
    Released,
}

/// A pad button with pressure information.
#[derive(Debug, Clone, Copy)]
pub enum PadButton {
    Pressed {
        /// Pressure in the interval [0, 1]
        pressure: f32,
    },
    Released,
}

/// A continuous fader or knob.
#[derive(Debug, Clone, Copy)]
pub struct Slider {
    /// Position in the interval [0, 1]
    pub position: f32,
}

/// A continuous fader or knob with a symmetric center position.
#[derive(Debug, Clone, Copy)]
pub struct CenterSlider {
    /// Position in the interval [-1, 1]
    pub position: f32,
}

/// An endless encoder that sends discrete delta values when rotated
/// in CW (positive) or CCW (negative) direction.
#[derive(Debug, Clone, Copy)]
pub struct StepEncoder {
    pub delta: i32,
}

/// An endless encoder that sends continuous delta values when rotated
/// in CW (positive) or CCW (negative) direction.
///
///  1.0: One full CW rotation (360 degrees)
/// -1.0: One full CCW rotation (360 degrees)
#[derive(Debug, Clone, Copy)]
pub struct SliderEncoder {
    pub delta: f32,
}
