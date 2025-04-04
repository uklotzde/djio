// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Interpolation of parameter values

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampingMode {
    /// Switch to the target value at the start of the transition interval.
    StepLeading,

    /// Switch to the target value at the end of the transition interval.
    StepTrailing,

    /// Approach the target value by linear interpolation over the transition interval.
    Linear,
}

/// Ramping profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampingProfile {
    pub mode: RampingMode,
    pub steps: usize,
}

impl RampingProfile {
    #[must_use]
    pub const fn immediate() -> Self {
        Self {
            mode: RampingMode::StepLeading,
            steps: 0,
        }
    }
}

/// Stepwise interpolation between an initial and a target value.
#[derive(Debug, Clone, Copy)]
pub struct RampingConfigF32 {
    pub profile: RampingProfile,
    pub initial_value: f32,
    pub target_value: f32,
}

impl RampingConfigF32 {
    #[must_use]
    fn step_delta(self) -> f32 {
        let Self {
            profile: RampingProfile { mode, steps },
            initial_value,
            target_value,
        } = self;
        if steps > 0 {
            match mode {
                RampingMode::StepLeading | RampingMode::StepTrailing => 0f32,
                RampingMode::Linear => (target_value - initial_value) / steps_as_f32(steps),
            }
        } else {
            // Never read
            0f32
        }
    }
}

/// Stepwise interpolation between an initial and a target value.
#[derive(Debug, Clone)]
pub struct RampingF32 {
    config: RampingConfigF32,
    step_delta: f32,
    current_step: usize,
}

// ~43.7 sec at 192 kHz
const MAX_LOSSLESS_STEPS_F32: usize = (1usize << 23) - 1; // f32 has a 23-bit mantissa

#[expect(clippy::cast_precision_loss)]
#[inline]
const fn steps_as_f32(steps: usize) -> f32 {
    // Comment out this debug assertion if precision loss might
    // occur and is considered acceptable.
    debug_assert!(steps <= MAX_LOSSLESS_STEPS_F32);
    steps as f32
}

impl RampingF32 {
    /// Create an immediate value without interpolation
    #[must_use]
    pub fn new(config: RampingConfigF32) -> Self {
        let step_delta = config.step_delta();
        Self {
            config,
            step_delta,
            current_step: 0,
        }
    }

    #[must_use]
    pub const fn config(&self) -> RampingConfigF32 {
        self.config
    }

    pub fn reset(&mut self, target_value: f32) {
        self.reset_profile(target_value, self.config.profile);
    }

    pub fn reset_profile(&mut self, target_value: f32, profile: RampingProfile) {
        self.config = RampingConfigF32 {
            profile,
            initial_value: self.current_value(),
            target_value,
        };
        self.step_delta = self.config.step_delta();
        self.current_step = 0;
    }

    #[must_use]
    #[cfg_attr(
        debug_assertions,
        expect(
            clippy::float_cmp,
            reason = "exact equality comparison in debug assertion"
        )
    )]
    pub fn current_value(&self) -> f32 {
        let Self {
            config,
            step_delta,
            current_step,
        } = self;
        debug_assert_eq!(*step_delta, config.step_delta());
        let RampingConfigF32 {
            profile: RampingProfile { mode, steps },
            initial_value,
            target_value,
        } = config;
        if current_step < steps {
            match mode {
                RampingMode::StepLeading => *target_value,
                RampingMode::StepTrailing => *initial_value,
                RampingMode::Linear => *initial_value + *step_delta * steps_as_f32(*current_step),
            }
        } else {
            *target_value
        }
    }

    #[must_use]
    pub fn remaining_steps(&self) -> usize {
        debug_assert!(self.current_step <= self.config.profile.steps);
        self.config.profile.steps - self.current_step
    }

    pub fn advance(&mut self, steps: usize) {
        if steps < self.remaining_steps() {
            self.current_step += steps;
        } else {
            self.current_step = self.config.profile.steps;
        }
    }
}

/// Iterate over the values generated by [`RampingF32`].
///
/// Iteration starts with the current value.
impl Iterator for RampingF32 {
    type Item = f32;

    /// Returns the current value and advances the iterator
    /// by a single step.
    fn next(&mut self) -> Option<Self::Item> {
        let current_value = self.current_value();
        self.advance(1);
        Some(current_value)
    }
}

#[cfg(test)]
mod tests {
    use super::{RampingConfigF32, RampingF32, RampingMode, RampingProfile};

    #[test]
    fn step_leading() {
        let profile = RampingProfile {
            mode: RampingMode::StepLeading,
            steps: 4,
        };
        let config = RampingConfigF32 {
            profile,
            initial_value: -1.0,
            target_value: 1.0,
        };
        let ramping = RampingF32::new(config);
        let values = ramping.take(profile.steps + 2).collect::<Vec<_>>();
        assert_eq!(values, [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn step_trailing() {
        let profile = RampingProfile {
            mode: RampingMode::StepTrailing,
            steps: 4,
        };
        let config = RampingConfigF32 {
            profile,
            initial_value: -1.0,
            target_value: 1.0,
        };
        let ramping = &mut RampingF32::new(config);
        let values = ramping.take(profile.steps + 2).collect::<Vec<_>>();
        assert_eq!(values, [-1.0, -1.0, -1.0, -1.0, 1.0, 1.0]);
    }

    #[test]
    fn linear() {
        let profile = RampingProfile {
            mode: RampingMode::Linear,
            steps: 4,
        };
        let config = RampingConfigF32 {
            profile,
            initial_value: -1.0,
            target_value: 1.0,
        };
        let ramping = RampingF32::new(config);
        let values = ramping.take(profile.steps + 2).collect::<Vec<_>>();
        assert_eq!(values, [-1.0, -0.5, 0.0, 0.5, 1.0, 1.0]);
    }
}
