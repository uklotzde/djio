// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Receiving and processing sensor data from devices
//! .

use std::ops::RangeInclusive;

use crate::{ControlRegister, ControlValue, TimeStamp};

/// Time-stamped input event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent<T> {
    pub ts: TimeStamp,
    pub input: T,
}

/// Emit input events after parsing the corresponding hardware-inputs
pub trait EmitInputEvent<T> {
    /// Emit an input event
    fn emit_input_event(&mut self, event: InputEvent<T>);
}

/// A simple two-state button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonInput {
    Pressed,
    Released,
}

impl From<ControlValue> for ButtonInput {
    fn from(from: ControlValue) -> Self {
        match from.to_bits() {
            0 => Self::Released,
            _ => Self::Pressed,
        }
    }
}

impl From<ButtonInput> for ControlValue {
    fn from(from: ButtonInput) -> Self {
        match from {
            ButtonInput::Released => Self::from_bits(0),
            ButtonInput::Pressed => Self::from_bits(1),
        }
    }
}

/// A pad button with pressure information.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct PadButtonInput {
    /// Pressure in the interval [0, 1]
    pub pressure: f32,
}

impl PadButtonInput {
    pub const MIN_PRESSURE: f32 = 0.0;
    pub const MAX_PRESSURE: f32 = 1.0;
    pub const PRESSURE_RANGE: RangeInclusive<f32> = Self::MIN_PRESSURE..=Self::MAX_PRESSURE;

    #[must_use]
    pub fn is_pressed(self) -> bool {
        debug_assert!(self.pressure >= Self::MIN_PRESSURE);
        debug_assert!(self.pressure <= Self::MAX_PRESSURE);
        self.pressure != Self::MIN_PRESSURE
    }

    #[must_use]
    pub fn from_u7(input: u8) -> Self {
        debug_assert_eq!(input, input & 0x7f);
        let pressure = f32::from(input) / 127.0;
        Self { pressure }
    }

    #[must_use]
    pub fn from_u14(input: u16) -> Self {
        debug_assert_eq!(input, input & 0x3fff);
        let pressure = f32::from(input) / 16383.0;
        Self { pressure }
    }
}

impl From<ControlValue> for PadButtonInput {
    fn from(from: ControlValue) -> Self {
        let pressure = f32::from_bits(from.to_bits());
        Self { pressure }
    }
}

impl From<PadButtonInput> for ControlValue {
    fn from(from: PadButtonInput) -> Self {
        let PadButtonInput { pressure } = from;
        Self::from_bits(pressure.to_bits())
    }
}

/// A continuous fader or knob.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct SliderInput {
    /// Position in the interval [0, 1]
    pub position: f32,
}

impl SliderInput {
    pub const MIN_POSITION: f32 = 0.0;
    pub const MAX_POSITION: f32 = 1.0;
    pub const POSITION_RANGE: RangeInclusive<f32> = Self::MIN_POSITION..=Self::MAX_POSITION;

    #[must_use]
    pub fn from_u7(input: u8) -> Self {
        debug_assert_eq!(input, input & 0x7f);
        let position = f32::from(input) / 127.0;
        Self { position }
    }

    #[must_use]
    pub fn from_u14(input: u16) -> Self {
        debug_assert_eq!(input, input & 0x3fff);
        let position = f32::from(input) / 16383.0;
        Self { position }
    }
}

impl From<ControlValue> for SliderInput {
    fn from(from: ControlValue) -> Self {
        let position = f32::from_bits(from.to_bits());
        Self { position }
    }
}

impl From<SliderInput> for ControlValue {
    fn from(from: SliderInput) -> Self {
        let SliderInput { position } = from;
        Self::from_bits(position.to_bits())
    }
}

/// A continuous fader or knob with a symmetric center position.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct CenterSliderInput {
    /// Position in the interval [-1, 1]
    pub position: f32,
}

impl CenterSliderInput {
    pub const MIN_POSITION: f32 = -1.0;
    pub const MAX_POSITION: f32 = 1.0;
    pub const POSITION_RANGE: RangeInclusive<f32> = Self::MIN_POSITION..=Self::MAX_POSITION;
    pub const CENTER_POSITION: f32 = 0.0;

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn from_u7(input: u8) -> Self {
        debug_assert_eq!(input, input & 0x7f);
        let position = if input < 64 {
            f32::from(input as i8 - 64) / 64.0
        } else {
            f32::from(input - 64) / 63.0
        };
        Self { position }
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn from_u14(input: u16) -> Self {
        debug_assert_eq!(input, input & 0x3fff);
        let position = if input < 8192 {
            f32::from(input as i16 - 8192) / 8192.0
        } else {
            f32::from(input - 8192) / 8191.0
        };
        Self { position }
    }
}

impl From<ControlValue> for CenterSliderInput {
    fn from(from: ControlValue) -> Self {
        let position = f32::from_bits(from.to_bits());
        Self { position }
    }
}

impl From<CenterSliderInput> for ControlValue {
    fn from(from: CenterSliderInput) -> Self {
        let CenterSliderInput { position } = from;
        Self::from_bits(position.to_bits())
    }
}

/// An endless encoder that sends discrete delta values
///
/// Usually implemented by a hardware knob/pot that sends either
/// positive or negative delta values while rotated in clockwise (CW)
/// or counter-clockwise (CCS) direction respectively.
///
/// The number of ticks per revolution or twist is device-dependent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct StepEncoderInput {
    pub delta: i32,
}

impl From<ControlValue> for StepEncoderInput {
    fn from(from: ControlValue) -> Self {
        #[allow(clippy::cast_possible_wrap)]
        let delta = from.to_bits() as i32;
        Self { delta }
    }
}

impl From<StepEncoderInput> for ControlValue {
    fn from(from: StepEncoderInput) -> Self {
        let StepEncoderInput { delta } = from;
        #[allow(clippy::cast_sign_loss)]
        Self::from_bits(delta as u32)
    }
}

/// An endless encoder that sends continuous delta values
///
/// Usually implemented by a hardware knob/pot that sends either
/// positive or negative delta values while rotated in clockwise (CW)
/// or counter-clockwise (CCS) direction respectively.
///
/// The scaling is device-dependent, but the following values are
/// recommended both for reference and for maximum portability:
///
///  1.0: One full CW rotation (360 degrees)
/// -1.0: One full CCW rotation (360 degrees)
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct SliderEncoderInput {
    pub delta: f32,
}

impl SliderEncoderInput {
    pub const DELTA_PER_CW_REV: f32 = 1.0;
    pub const DELTA_PER_CCW_REV: f32 = -1.0;

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn from_u7(input: u8) -> Self {
        debug_assert_eq!(input, input & 0x7f);
        let delta = if input < 64 {
            f32::from(input) / 63.0
        } else {
            f32::from(input.wrapping_sub(128) as i8) / 64.0
        };
        Self { delta }
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn from_u14(input: u16) -> Self {
        debug_assert_eq!(input, input & 0x3fff);
        let delta = if input < 8192 {
            f32::from(input) / 8191.0
        } else {
            f32::from(input.wrapping_sub(16384) as i16) / 8192.0
        };
        Self { delta }
    }
}

impl From<ControlValue> for SliderEncoderInput {
    fn from(from: ControlValue) -> Self {
        let delta = f32::from_bits(from.to_bits());
        Self { delta }
    }
}

impl From<SliderEncoderInput> for ControlValue {
    fn from(from: SliderEncoderInput) -> Self {
        let SliderEncoderInput { delta } = from;
        Self::from_bits(delta.to_bits())
    }
}

pub type ControlInputEvent = InputEvent<ControlRegister>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn pad_button_from_u7() {
        debug_assert_eq!(
            PadButtonInput::MIN_PRESSURE,
            PadButtonInput::from_u7(0).pressure
        );
        debug_assert_eq!(
            PadButtonInput::MAX_PRESSURE,
            PadButtonInput::from_u7(127).pressure
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn pad_button_from_u14() {
        debug_assert_eq!(
            PadButtonInput::MIN_PRESSURE,
            PadButtonInput::from_u14(0).pressure
        );
        debug_assert_eq!(
            PadButtonInput::MAX_PRESSURE,
            PadButtonInput::from_u14(16383).pressure
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_from_u7() {
        debug_assert_eq!(SliderInput::MIN_POSITION, SliderInput::from_u7(0).position);
        debug_assert_eq!(
            SliderInput::MAX_POSITION,
            SliderInput::from_u7(127).position
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_from_u14() {
        debug_assert_eq!(SliderInput::MIN_POSITION, SliderInput::from_u14(0).position);
        debug_assert_eq!(
            SliderInput::MAX_POSITION,
            SliderInput::from_u14(16383).position
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn center_slider_from_u7() {
        debug_assert_eq!(
            CenterSliderInput::MIN_POSITION,
            CenterSliderInput::from_u7(0).position
        );
        debug_assert_eq!(
            CenterSliderInput::CENTER_POSITION,
            CenterSliderInput::from_u7(64).position
        );
        debug_assert_eq!(
            CenterSliderInput::MAX_POSITION,
            CenterSliderInput::from_u7(127).position
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn center_slider_from_u14() {
        debug_assert_eq!(
            CenterSliderInput::MIN_POSITION,
            CenterSliderInput::from_u14(0).position
        );
        debug_assert_eq!(
            CenterSliderInput::CENTER_POSITION,
            CenterSliderInput::from_u14(8192).position
        );
        debug_assert_eq!(
            CenterSliderInput::MAX_POSITION,
            CenterSliderInput::from_u14(16383).position
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_encoder_from_u7() {
        debug_assert_eq!(0.0, SliderEncoderInput::from_u7(0).delta);
        debug_assert_eq!(
            SliderEncoderInput::DELTA_PER_CW_REV,
            SliderEncoderInput::from_u7(63).delta
        );
        debug_assert_eq!(
            SliderEncoderInput::DELTA_PER_CCW_REV,
            SliderEncoderInput::from_u7(64).delta
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_encoder_from_u14() {
        debug_assert_eq!(0.0, SliderEncoderInput::from_u14(0).delta);
        debug_assert_eq!(
            SliderEncoderInput::DELTA_PER_CW_REV,
            SliderEncoderInput::from_u14(8191).delta
        );
        debug_assert_eq!(
            SliderEncoderInput::DELTA_PER_CCW_REV,
            SliderEncoderInput::from_u14(8192).delta
        );
    }
}
