// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Receiving and processing sensor data from devices
//! .

use std::{
    borrow::Borrow,
    cmp::Ordering,
    ops::{Add, Mul, RangeInclusive, Sub},
};

use float_cmp::approx_eq;
use strum::FromRepr;

use crate::{Control, ControlValue, TimeStamp};

/// Time-stamped input event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent<T> {
    pub ts: TimeStamp,
    pub input: T,
}

pub fn input_events_ordered_chronologically<I, T>(events: I) -> bool
where
    I: IntoIterator,
    I::Item: Borrow<InputEvent<T>>,
{
    events.into_iter().is_sorted_by_key(|item| item.borrow().ts)
}

/// A simple two-state button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u8)]
pub enum ButtonInput {
    Released = 0,
    Pressed = 1,
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
    fn from(value: ButtonInput) -> Self {
        Self::from_bits(value as _)
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
    pub fn as_button(self) -> ButtonInput {
        debug_assert!(Self::PRESSURE_RANGE.contains(&self.pressure));
        if self.pressure > Self::MIN_PRESSURE {
            ButtonInput::Pressed
        } else {
            ButtonInput::Released
        }
    }

    #[must_use]
    pub fn from_u7(input: u8) -> Self {
        debug_assert!(input <= 127);
        let pressure = f32::from(input) / 127.0;
        debug_assert!(Self::PRESSURE_RANGE.contains(&pressure));
        Self { pressure }
    }

    #[must_use]
    pub fn from_u14(input: u16) -> Self {
        debug_assert!(input <= 16383);
        let pressure = f32::from(input) / 16383.0;
        debug_assert!(Self::PRESSURE_RANGE.contains(&pressure));
        Self { pressure }
    }
}

impl From<ControlValue> for PadButtonInput {
    fn from(from: ControlValue) -> Self {
        let pressure = f32::from_bits(from.to_bits());
        debug_assert!(Self::PRESSURE_RANGE.contains(&pressure));
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
    pub const fn clamp_position(position: f32) -> f32 {
        position.clamp(Self::MIN_POSITION, Self::MAX_POSITION)
    }

    #[must_use]
    pub fn from_u7(input: u8) -> Self {
        debug_assert!(input <= 127);
        let position = f32::from(input) / 127.0;
        debug_assert!(Self::POSITION_RANGE.contains(&position));
        Self { position }
    }

    #[must_use]
    pub fn from_u14(input: u16) -> Self {
        debug_assert!(input <= 16383);
        let position = f32::from(input) / 16383.0;
        debug_assert!(Self::POSITION_RANGE.contains(&position));
        Self { position }
    }

    #[must_use]
    pub fn inverse(self) -> Self {
        let Self { position } = self;
        Self {
            position: Self::MAX_POSITION - position,
        }
    }

    #[must_use]
    pub fn map_position_linear<T>(self, min_value: T, max_value: T) -> T
    where
        T: From<f32> + Sub<Output = T> + Mul<Output = T> + Add<Output = T> + Copy,
    {
        let Self { position } = self;
        min_value + T::from(position) * (max_value - min_value)
    }

    /// Interpret the position as a ratio for adjusting the volume of a signal.
    ///
    /// The position is interpreted as a volume level between the silence level
    /// (< 0 dB) and 0 dB.
    ///
    /// Multiply the signal with the returned value to adjust the volume.
    #[must_use]
    #[inline]
    pub fn map_position_to_gain_ratio(self, silence_db: f32) -> f32 {
        debug_assert!(silence_db < 0.0);
        let Self { position } = self;
        let gain_ratio = db_to_ratio((1.0 - position) * silence_db);
        // Still in range after transformation
        debug_assert!(Self::POSITION_RANGE.contains(&gain_ratio));
        gain_ratio
    }
}

impl From<ControlValue> for SliderInput {
    fn from(from: ControlValue) -> Self {
        let position = f32::from_bits(from.to_bits());
        debug_assert!(Self::POSITION_RANGE.contains(&position));
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
    pub const fn clamp_position(position: f32) -> f32 {
        position.clamp(Self::MIN_POSITION, Self::MAX_POSITION)
    }

    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn from_u7(input: u8) -> Self {
        debug_assert!(input < 128);
        let position = if input < 64 {
            f32::from(input as i8 - 64) / 64.0
        } else {
            f32::from(input - 64) / 63.0
        };
        debug_assert!(Self::POSITION_RANGE.contains(&position));
        Self { position }
    }

    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn from_u14(input: u16) -> Self {
        debug_assert!(input < 16384);
        let position = if input < 8192 {
            f32::from(input as i16 - 8192) / 8192.0
        } else {
            f32::from(input - 8192) / 8191.0
        };
        debug_assert!(Self::POSITION_RANGE.contains(&position));
        Self { position }
    }

    #[must_use]
    pub fn inverse(self) -> Self {
        let Self { position } = self;
        if self.position == Self::CENTER_POSITION {
            // Prevent the value -0.0
            Self { position }
        } else {
            Self {
                position: -position,
            }
        }
    }

    #[must_use]
    #[inline]
    pub fn map_position_linear<T>(self, min_value: T, center_value: T, max_value: T) -> T
    where
        T: From<f32> + Sub<Output = T> + Mul<Output = T> + Add<Output = T> + Copy + PartialOrd,
    {
        debug_assert!(
            (min_value <= center_value && center_value <= max_value)
                || (min_value >= center_value && center_value >= max_value)
        );
        let Self { position } = self;
        match position
            .partial_cmp(&Self::CENTER_POSITION)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => center_value,
            Ordering::Less => T::from(position) * (center_value - min_value) + center_value,
            Ordering::Greater => T::from(position) * (max_value - center_value) + center_value,
        }
    }

    /// Interpret the position as a ratio for tuning the volume of a signal.
    ///
    /// The position is interpreted as a volume level between the `min_db`
    /// (< 0 dB) and `max_db` (> 0 dB), e.g. -26 dB and +6 dB (Pioneer DJM).
    ///
    /// Multiply the signal with the returned value to tune the volume.
    #[must_use]
    #[inline]
    pub fn map_position_to_gain_ratio(self, min_db: f32, max_db: f32) -> f32 {
        debug_assert!(min_db < 0.0);
        debug_assert!(max_db > 0.0);
        debug_assert!(min_db < max_db);
        let Self { position } = self;
        match position
            .partial_cmp(&Self::CENTER_POSITION)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => 1.0,
            Ordering::Less => db_to_ratio(-position * min_db),
            Ordering::Greater => db_to_ratio(position * max_db),
        }
    }
}

impl From<ControlValue> for CenterSliderInput {
    fn from(from: ControlValue) -> Self {
        let position = f32::from_bits(from.to_bits());
        debug_assert!(Self::POSITION_RANGE.contains(&position));
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

impl StepEncoderInput {
    #[must_use]
    pub fn from_u7(input: u8) -> Self {
        debug_assert!(input < 0x80);
        let delta = if input < 0x40 {
            i32::from(input)
        } else {
            i32::from(input) - 0x80
        };
        Self { delta }
    }

    #[must_use]
    pub fn from_u14(input: u16) -> Self {
        debug_assert!(input < 0x4000);
        let delta = if input < 0x2000 {
            i32::from(input)
        } else {
            i32::from(input) - 0x4000
        };
        Self { delta }
    }
}

impl From<ControlValue> for StepEncoderInput {
    fn from(from: ControlValue) -> Self {
        #[expect(clippy::cast_possible_wrap)]
        let delta = from.to_bits() as i32;
        Self { delta }
    }
}

impl From<StepEncoderInput> for ControlValue {
    fn from(from: StepEncoderInput) -> Self {
        let StepEncoderInput { delta } = from;
        #[expect(clippy::cast_sign_loss)]
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
    #[must_use]
    pub fn inverse(self) -> Self {
        let Self { delta } = self;
        if self.delta == 0.0 {
            // Prevent the value -0.0
            Self { delta }
        } else {
            Self { delta: -delta }
        }
    }
}

impl SliderEncoderInput {
    pub const DELTA_PER_CW_REV: f32 = 1.0;
    pub const DELTA_PER_CCW_REV: f32 = -1.0;

    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn from_u7(input: u8) -> Self {
        debug_assert!(input < 128);
        let delta = if input < 64 {
            f32::from(input) / 63.0
        } else {
            f32::from(input.wrapping_sub(128) as i8) / 64.0
        };
        Self { delta }
    }

    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub fn from_u14(input: u16) -> Self {
        debug_assert!(input < 16384);
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

/// Choose one out of many, discrete possible choices
///
/// Useful for configuration settings, e.g. selecting a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorInput {
    pub choice: u32,
}

impl From<ControlValue> for SelectorInput {
    fn from(from: ControlValue) -> Self {
        let choice = from.to_bits();
        Self { choice }
    }
}

impl From<SelectorInput> for ControlValue {
    fn from(from: SelectorInput) -> Self {
        let SelectorInput { choice } = from;
        Self::from_bits(choice)
    }
}

pub type ControlInputEvent = InputEvent<Control>;

pub trait ControlInputEventSink {
    /// Callback for sinking control input events
    ///
    /// The caller will provide one or more events per invocation.
    /// Multiple events are ordered chronologically according to
    /// their time stamps.
    fn sink_control_input_events(&mut self, events: &[ControlInputEvent]);
}

#[must_use]
pub fn split_crossfader_input_linear(input: CenterSliderInput) -> (SliderInput, SliderInput) {
    const fn f_x(x: f32) -> f32 {
        x
    }
    let CenterSliderInput { position } = input;
    let x = position * 0.5 + 0.5; // [0, 1]
    let left_position = f_x(1.0 - x);
    let right_position = f_x(x);
    debug_assert!(SliderInput::POSITION_RANGE.contains(&left_position));
    debug_assert!(SliderInput::POSITION_RANGE.contains(&right_position));
    (
        SliderInput {
            position: left_position,
        },
        SliderInput {
            position: right_position,
        },
    )
}

#[must_use]
pub fn split_crossfader_input_amplitude_preserving_approx(
    input: CenterSliderInput,
) -> (SliderInput, SliderInput) {
    // <https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/>
    #[expect(clippy::cast_possible_truncation)]
    fn f_x(x: f64) -> f32 {
        (x.powi(2) * (3.0 - 2.0 * x)) as _
    }
    let CenterSliderInput { position } = input;
    let x: f64 = f64::from(position) * 0.5 + 0.5; // [0, 1]
    let left_position = f_x(1.0 - x);
    let right_position = f_x(x);
    (
        SliderInput {
            position: SliderInput::clamp_position(left_position),
        },
        SliderInput {
            position: SliderInput::clamp_position(right_position),
        },
    )
}

#[must_use]
pub fn split_crossfader_input_energy_preserving_approx(
    input: CenterSliderInput,
) -> (SliderInput, SliderInput) {
    // <https://signalsmith-audio.co.uk/writing/2021/cheap-energy-crossfade/>
    #[expect(clippy::cast_possible_truncation)]
    fn f_x(x: f64) -> f32 {
        let y = x * (1.0 - x);
        (y * (1.0 + 1.4186 * y) + x).powi(2) as _
    }
    let CenterSliderInput { position } = input;
    let x = f64::from(position) * 0.5 + 0.5; // [0, 1]
    let left_position = f_x(1.0 - x);
    let right_position = f_x(x);
    (
        SliderInput {
            position: SliderInput::clamp_position(left_position),
        },
        SliderInput {
            position: SliderInput::clamp_position(right_position),
        },
    )
}

#[must_use]
pub fn split_crossfader_input_square(input: CenterSliderInput) -> (SliderInput, SliderInput) {
    let CenterSliderInput { position } = input;
    let left_position = if approx_eq!(f32, position, CenterSliderInput::MAX_POSITION) {
        SliderInput::MIN_POSITION
    } else {
        SliderInput::MAX_POSITION
    };
    let right_position = if approx_eq!(f32, position, CenterSliderInput::MIN_POSITION) {
        SliderInput::MIN_POSITION
    } else {
        SliderInput::MAX_POSITION
    };
    (
        SliderInput {
            position: left_position,
        },
        SliderInput {
            position: right_position,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfaderCurve {
    Linear,
    AmplitudePreserving,
    EnergyPreserving,
    Square,
}

impl CrossfaderCurve {
    #[must_use]
    pub fn split_input(self, input: CenterSliderInput) -> (SliderInput, SliderInput) {
        match self {
            Self::Linear => split_crossfader_input_linear(input),
            Self::AmplitudePreserving => split_crossfader_input_amplitude_preserving_approx(input),
            Self::EnergyPreserving => split_crossfader_input_energy_preserving_approx(input),
            Self::Square => split_crossfader_input_square(input),
        }
    }
}

#[inline]
fn db_to_ratio(gain: f32) -> f32 {
    10.0f32.powf(gain / 20.0)
}

#[cfg(test)]
mod tests;
