// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::fmt;

/// Time stamp with microsecond precision
///
/// The actual value has no meaning, i.e. the origin with value 0 is arbitrary.
/// Only the difference between two time stamps should be considered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeStamp(u64);

impl TimeStamp {
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    #[must_use]
    pub const fn to_micros(self) -> u64 {
        let Self(micros) = self;
        micros
    }
}

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{micros} \u{00B5}s",
            micros = self.to_micros()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent<T> {
    pub ts: TimeStamp,
    pub input: T,
}

pub trait EmitInputEvent<T> {
    fn emit_input_event(&mut self, event: InputEvent<T>);
}

/// A simple two-state button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonInput {
    Pressed,
    Released,
}

/// A pad button with pressure information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadButtonInput {
    /// Pressure in the interval [0, 1]
    pub pressure: f32,
}

impl PadButtonInput {
    #[must_use]
    pub fn is_pressed(self) -> bool {
        debug_assert!(self.pressure >= 0.0);
        self.pressure != 0.0
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

/// A continuous fader or knob.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderInput {
    /// Position in the interval [0, 1]
    pub position: f32,
}

impl SliderInput {
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

/// A continuous fader or knob with a symmetric center position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterSliderInput {
    /// Position in the interval [-1, 1]
    pub position: f32,
}

impl CenterSliderInput {
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

/// An endless encoder that sends discrete delta values when rotated
/// in CW (positive) or CCW (negative) direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepEncoderInput {
    pub delta: i32,
}

/// An endless encoder that sends continuous delta values when rotated
/// in CW (positive) or CCW (negative) direction.
///
///  1.0: One full CW rotation (360 degrees)
/// -1.0: One full CCW rotation (360 degrees)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderEncoderInput {
    pub delta: f32,
}

impl SliderEncoderInput {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Button(ButtonInput),
    PadButton(PadButtonInput),
    Slider(SliderInput),
    CenterSlider(CenterSliderInput),
    SliderEncoder(SliderEncoderInput),
}

#[must_use]
pub fn u7_be_to_u14(hi: u8, lo: u8) -> u16 {
    debug_assert_eq!(hi, hi & 0x7f);
    debug_assert_eq!(lo, lo & 0x7f);
    u16::from(hi) << 7 | u16::from(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_from_u7() {
        debug_assert_eq!(0.0, SliderInput::from_u7(0).position);
        debug_assert_eq!(1.0, SliderInput::from_u7(127).position);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_from_u14() {
        debug_assert_eq!(0.0, SliderInput::from_u14(0).position);
        debug_assert_eq!(1.0, SliderInput::from_u14(16383).position);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn pad_button_from_u7() {
        debug_assert_eq!(0.0, PadButtonInput::from_u7(0).pressure);
        debug_assert_eq!(1.0, PadButtonInput::from_u7(127).pressure);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn pad_button_from_u14() {
        debug_assert_eq!(0.0, PadButtonInput::from_u14(0).pressure);
        debug_assert_eq!(1.0, PadButtonInput::from_u14(16383).pressure);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn center_slider_from_u7() {
        debug_assert_eq!(-1.0, CenterSliderInput::from_u7(0).position);
        debug_assert_eq!(0.0, CenterSliderInput::from_u7(64).position);
        debug_assert_eq!(1.0, CenterSliderInput::from_u7(127).position);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn center_slider_from_u14() {
        debug_assert_eq!(-1.0, CenterSliderInput::from_u14(0).position);
        debug_assert_eq!(0.0, CenterSliderInput::from_u14(8192).position);
        debug_assert_eq!(1.0, CenterSliderInput::from_u14(16383).position);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_encoder_from_u7() {
        debug_assert_eq!(0.0, SliderEncoderInput::from_u7(0).delta);
        debug_assert_eq!(1.0, SliderEncoderInput::from_u7(63).delta);
        debug_assert_eq!(-1.0, SliderEncoderInput::from_u7(64).delta);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn slider_encoder_from_u14() {
        debug_assert_eq!(0.0, SliderEncoderInput::from_u14(0).delta);
        debug_assert_eq!(1.0, SliderEncoderInput::from_u14(8191).delta);
        debug_assert_eq!(-1.0, SliderEncoderInput::from_u14(8192).delta);
    }
}
