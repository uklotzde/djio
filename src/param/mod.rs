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

use std::borrow::{Borrow, Cow};
use std::{cmp::Ordering, ops::Bound};

use derive_more::{AsRef, Deref, Display, From, Into};
use enum_as_inner::EnumAsInner;
use smol_str::SmolStr;
use strum::EnumDiscriminants;

mod atomic;
pub use self::atomic::{AtomicValue, SharedAtomicValue, WeakAtomicValue};

mod ramping;
pub use self::ramping::{RampingF32, RampingMode, RampingProfile};

mod registry;
pub use self::registry::{
    RegisteredId, Registration, RegistrationError, RegistrationStatus, Registry, RegistryEntryRef,
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
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, EnumAsInner, EnumDiscriminants, From)]
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

/// Human-readable name.
///
/// Used as a label.
#[derive(
    Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd, Display, From, Into, AsRef, Deref,
)]
pub struct Name(SmolStr);

impl Name {
    #[must_use]
    pub const fn new(inner: SmolStr) -> Self {
        Self(inner)
    }
}

impl From<&str> for Name {
    fn from(from: &str) -> Self {
        Self(from.into())
    }
}

impl From<Cow<'_, str>> for Name {
    fn from(from: Cow<'_, str>) -> Self {
        Self(from.into())
    }
}

impl From<String> for Name {
    fn from(from: String) -> Self {
        Self(from.into())
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

/// Parameter unit.
///
/// A short code for display purposes, preferably according to ISO standards.
#[derive(
    Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd, Display, From, Into, AsRef, Deref,
)]
pub struct Unit(SmolStr);

impl Unit {
    #[must_use]
    pub const fn new(inner: SmolStr) -> Self {
        Self(inner)
    }
}
impl From<&str> for Unit {
    fn from(from: &str) -> Self {
        Self(from.into())
    }
}

impl From<Cow<'_, str>> for Unit {
    fn from(from: Cow<'_, str>) -> Self {
        Self(from.into())
    }
}

impl From<String> for Unit {
    fn from(from: String) -> Self {
        Self(from.into())
    }
}

impl AsRef<str> for Unit {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for Unit {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Descriptor {
    /// Display name.
    ///
    /// Mandatory, but could be left empty if no innate name is available.
    pub name: Name,

    /// Display unit.
    ///
    /// Unit of the value.
    pub unit: Option<Unit>,

    /// The direction.
    pub direction: Direction,

    /// Value metadata.
    pub value: ValueDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDescriptor {
    /// Default value for initialization and reset.
    ///
    /// The default value implicitly determines the value type.
    pub default: Value,

    /// Range restrictions
    ///
    /// The value type of the boundary values must match the value type of the default value.
    pub range: ValueRangeDescriptor,
}

impl ValueDescriptor {
    #[must_use]
    pub const fn default(default: Value) -> Self {
        Self {
            default,
            range: ValueRangeDescriptor::unbounded(),
        }
    }

    #[must_use]
    pub fn value_type(&self) -> ValueType {
        self.default.into()
    }
}

/// Value limits.
///
/// Both `min` and `max` value must be of the same type if bounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueRangeDescriptor {
    /// Minimum value.
    pub min: Bound<Value>,

    /// Maximum value.
    pub max: Bound<Value>,
}

impl ValueRangeDescriptor {
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            min: Bound::Unbounded,
            max: Bound::Unbounded,
        }
    }

    #[must_use]
    pub fn value_type(&self) -> Option<ValueType> {
        match self {
            Self {
                min: Bound::Unbounded,
                max: Bound::Unbounded,
            } => None,
            Self {
                min: Bound::Included(min) | Bound::Excluded(min),
                max: Bound::Unbounded,
            } => Some(min.into()),
            Self {
                min: Bound::Unbounded,
                max: Bound::Included(max) | Bound::Excluded(max),
            } => Some(max.into()),
            Self {
                min: Bound::Included(min) | Bound::Excluded(min),
                max: Bound::Included(max) | Bound::Excluded(max),
            } => {
                debug_assert_eq!(ValueType::from(min), ValueType::from(max));
                Some(min.into())
            }
        }
    }

    /// Check if a value is in range.
    ///
    /// Comparing values of different types is not allowed. The result
    /// is `None` in this case and a debug assertions is triggered.
    #[must_use]
    pub fn contains_value(&self, value: Value) -> Option<bool> {
        let Self { min, max } = self;
        match min {
            Bound::Unbounded => (),
            Bound::Included(min_inclusive) => match value.partial_cmp(min_inclusive)? {
                Ordering::Equal | Ordering::Greater => (),
                Ordering::Less => return Some(false),
            },
            Bound::Excluded(min_exclusive) => match value.partial_cmp(min_exclusive)? {
                Ordering::Greater => (),
                Ordering::Less | Ordering::Equal => return Some(false),
            },
        }
        match max {
            Bound::Unbounded => (),
            Bound::Included(max_inclusive) => match value.partial_cmp(max_inclusive)? {
                Ordering::Less | Ordering::Equal => (),
                Ordering::Greater => return Some(false),
            },
            Bound::Excluded(max_exclusive) => match value.partial_cmp(max_exclusive)? {
                Ordering::Less => (),
                Ordering::Equal | Ordering::Greater => return Some(false),
            },
        }
        Some(true)
    }
}

impl Default for ValueRangeDescriptor {
    fn default() -> Self {
        Self::unbounded()
    }
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Display, From, Into, AsRef, Deref,
)]
pub struct Address(SmolStr);

/// Stringified parameter address.
///
/// Addresses are supposed to be stable and permanent, i.e. they do not change between sessions.
impl Address {
    #[must_use]
    pub const fn new(inner: SmolStr) -> Self {
        Self(inner)
    }
}

impl From<&str> for Address {
    fn from(from: &str) -> Self {
        Self(from.into())
    }
}

impl From<Cow<'_, str>> for Address {
    fn from(from: Cow<'_, str>) -> Self {
        Self(from.into())
    }
}

impl From<String> for Address {
    fn from(from: String) -> Self {
        Self(from.into())
    }
}

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for Address {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_descriptor() {
        assert_eq!(
            ValueDescriptor::default(Value::Bool(true)),
            ValueDescriptor {
                default: Value::Bool(true),
                range: Default::default(),
            }
        );
    }

    #[test]
    fn default_value_range_is_unbounded() {
        assert_eq!(
            ValueRangeDescriptor::default(),
            ValueRangeDescriptor::unbounded()
        );
    }
}
