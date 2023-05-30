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
pub struct Event<T> {
    pub ts: TimeStamp,
    pub input: T,
}

pub trait EmitEvent<T> {
    fn emit_event(&mut self, event: Event<T>);
}

/// A simple two-state button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Pressed,
    Released,
}

/// A pad button with pressure information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadButton {
    Pressed {
        /// Pressure in the interval [0, 1]
        pressure: f32,
    },
    Released,
}

/// A continuous fader or knob.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slider {
    /// Position in the interval [0, 1]
    pub position: f32,
}

/// A continuous fader or knob with a symmetric center position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterSlider {
    /// Position in the interval [-1, 1]
    pub position: f32,
}

/// An endless encoder that sends discrete delta values when rotated
/// in CW (positive) or CCW (negative) direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepEncoder {
    pub delta: i32,
}

/// An endless encoder that sends continuous delta values when rotated
/// in CW (positive) or CCW (negative) direction.
///
///  1.0: One full CW rotation (360 degrees)
/// -1.0: One full CCW rotation (360 degrees)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderEncoder {
    pub delta: f32,
}

#[must_use]
pub fn u7_be_to_u14(hi: u8, lo: u8) -> u16 {
    debug_assert_eq!(hi, hi & 0x7f);
    debug_assert_eq!(lo, lo & 0x7f);
    u16::from(hi) << 7 | u16::from(lo)
}

#[must_use]
pub fn u7_to_slider(input: u8) -> Slider {
    debug_assert_eq!(input, input & 0x7f);
    let position = f32::from(input) / 127.0;
    Slider { position }
}

#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn u7_to_slider_encoder(input: u8) -> SliderEncoder {
    debug_assert_eq!(input, input & 0x7f);
    let delta = if input < 64 {
        f32::from(input) / 63.0
    } else {
        f32::from(input.wrapping_sub(128) as i8) / 64.0
    };
    SliderEncoder { delta }
}

#[must_use]
pub fn u14_to_slider(input: u16) -> Slider {
    debug_assert_eq!(input, input & 0x3fff);
    let position = f32::from(input) / 16383.0;
    Slider { position }
}

#[must_use]
pub fn u7_to_center_slider(input: u8) -> CenterSlider {
    debug_assert_eq!(input, input & 0x7f);
    let position = f32::from(input) * 2.0 / 127.0 - 1.0;
    CenterSlider { position }
}

#[must_use]
pub fn u14_to_center_slider(input: u16) -> CenterSlider {
    debug_assert_eq!(input, input & 0x3fff);
    let position = f32::from(input) * 2.0 / 16383.0 - 1.0;
    CenterSlider { position }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn u7_to_slider_encoder_delta() {
        debug_assert_eq!(0.0, u7_to_slider_encoder(0).delta);
        debug_assert_eq!(1.0, u7_to_slider_encoder(63).delta);
        debug_assert_eq!(-1.0, u7_to_slider_encoder(64).delta);
    }
}
