// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Application/plugin control parameters
//!
//! Live control of parameters for connecting hardware inputs (buttons, pots, faders)
//! and outputs (LEDs, visuals, haptic feedback) to the real-time kernel.
//!
//! The parameter descriptors and addresses are defined by the application or plugin
//! and are registered in a global, application-wide registry.
//!
//! Controller-specific adapters for connecting selected hardware inputs/outputs to
//! parameter values are written separately. Input parameters are supposed to
//! be sent to the real-time kernel when changed. Values of output parameters must
//! be polled periodically for updating the corresponding hardware outputs.

use std::{borrow::Cow, cmp::Ordering};

use enum_as_inner::EnumAsInner;
use strum::EnumDiscriminants;

mod atomic;
pub use self::atomic::{AtomicValue, SharedAtomicValue, WeakAtomicValue};

mod registry;
pub use self::registry::{
    DescriptorRegistration, RegisterError, RegisteredDescriptor, RegisteredId, RegisteredParam,
    Registration, RegistrationHeader, RegistrationStatus, Registry,
};

/// Direction
///
/// Defines the direction of communication of parameter values, i.e.
/// read/load or write/store. The variant names reflect the view of
/// the _provider_.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Input
    ///
    /// Read-only for the provider, who will read out the current
    /// value periodically or when triggered by an event. Providers
    /// should never write an input.
    ///
    /// Consumers should primarily write those values, although
    /// reading back out the current values is possible. Multiple
    /// consumers of an input need to coordinate themselves,
    /// otherwise values are overwritten in an uncontrolled manner.
    Input,

    /// Output
    ///
    /// Write-only for the provider, who is allowed to update and
    /// overwrite the current value at any time. Providers should
    /// never read an output.
    ///
    /// Consumers should only read output parameters. Writing them
    /// is pointless and might confuse other consumers.
    Output,
}

/// Parameter value
///
/// Restricted to 32-bit types for portability. All values
/// need to be stored atomically.
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, EnumAsInner, EnumDiscriminants, derive_more::From,
)]
#[strum_discriminants(name(ValueType))]
pub enum Value {
    /// Boolean value
    Bool(bool),
    /// 32-bit signed integer value
    I32(i32),
    /// 32-bit unsigned integer value
    U32(u32),
    /// 32-bit single-precision floating-point number value
    F32(f32),
}

/// Human-readable name
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    derive_more::From,
    derive_more::Display,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct Name<'a>(Cow<'a, str>);

impl<'a> Name<'a> {
    #[must_use]
    pub const fn new(inner: Cow<'a, str>) -> Self {
        Self(inner)
    }
}

impl<'a> From<Name<'a>> for Cow<'a, str> {
    fn from(from: Name<'a>) -> Self {
        let Name(inner) = from;
        inner
    }
}

/// Human-readable unit label
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    derive_more::From,
    derive_more::Display,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct Unit<'a>(Cow<'a, str>);

impl<'a> Unit<'a> {
    #[must_use]
    pub const fn new(inner: Cow<'a, str>) -> Self {
        Self(inner)
    }
}

impl<'a> From<Unit<'a>> for Cow<'a, str> {
    fn from(from: Unit<'a>) -> Self {
        let Unit(inner) = from;
        inner
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Descriptor<'a> {
    /// Display name.
    ///
    /// Mandatory, but could be left empty if no innate name is available.
    pub name: Name<'a>,

    /// Display unit.
    ///
    /// Unit of the value.
    pub unit: Option<Unit<'a>>,

    /// The direction.
    pub direction: Direction,

    /// Value metadata.
    pub value: ValueDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDescriptor {
    /// Range restrictions
    pub range: ValueRangeDescriptor,

    /// Default value for initialization and reset.
    ///
    /// The default value implicitly determines the value type.
    pub default: Value,
}

impl ValueDescriptor {
    #[must_use]
    pub fn value_type(&self) -> ValueType {
        self.default.into()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueRangeDescriptor {
    /// Minimum value (inclusive)
    pub min: Option<Value>,

    /// Maximum value (inclusive)
    pub max: Option<Value>,
}

impl ValueRangeDescriptor {
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    /// Check if a value is in range.
    ///
    /// Comparing values of different types is not allowed. The result
    /// is `false` in this case and a debug assertion is triggered.
    #[must_use]
    pub fn is_value_in_range(&self, value: Value) -> bool {
        let Self { min, max } = self;
        if let Some(min) = min {
            debug_assert_eq!(ValueType::from(min), ValueType::from(value));
            match value.partial_cmp(min) {
                Some(Ordering::Equal | Ordering::Greater) => (),
                Some(Ordering::Less) | None => return false,
            }
        }
        if let Some(max) = max {
            debug_assert_eq!(ValueType::from(max), ValueType::from(value));
            match value.partial_cmp(max) {
                Some(Ordering::Equal | Ordering::Less) => (),
                Some(Ordering::Greater) | None => return false,
            }
        }
        true
    }
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    derive_more::From,
    derive_more::Display,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct Address<'a>(Cow<'a, str>);

impl<'a> Address<'a> {
    #[must_use]
    pub const fn new(inner: Cow<'a, str>) -> Self {
        Self(inner)
    }
}

impl<'a> From<Address<'a>> for Cow<'a, str> {
    fn from(from: Address<'a>) -> Self {
        let Address(inner) = from;
        inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_range_is_unbounded() {
        assert_eq!(
            ValueRangeDescriptor::default(),
            ValueRangeDescriptor::unbounded()
        );
    }
}
