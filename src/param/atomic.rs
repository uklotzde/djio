// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Atomic parameter values
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
    Arc, Weak,
};

use crossbeam_utils::atomic::AtomicConsume;
use enum_as_inner::EnumAsInner;

use super::{Value, ValueType};

/// Atomic f32 value with limited functionality.
#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicF32 {
    bits: AtomicU32,
}

impl AtomicF32 {
    #[must_use]
    pub fn new(value: f32) -> Self {
        let bits = value.to_bits();
        Self {
            bits: AtomicU32::new(bits),
        }
    }

    #[must_use]
    pub fn load(&self, ordering: Ordering) -> f32 {
        let bits = self.bits.load(ordering);
        f32::from_bits(bits)
    }

    #[must_use]
    fn load_consume(&self) -> f32 {
        let bits = self.bits.load_consume();
        f32::from_bits(bits)
    }

    pub fn store(&self, value: f32, ordering: Ordering) {
        let bits = value.to_bits();
        self.bits.store(bits, ordering);
    }

    pub fn swap(&self, value: f32, ordering: Ordering) -> f32 {
        let bits = value.to_bits();
        f32::from_bits(self.bits.swap(bits, ordering))
    }
}

impl AtomicConsume for AtomicF32 {
    type Val = f32;

    #[must_use]
    fn load_consume(&self) -> Self::Val {
        Self::load_consume(self)
    }
}

/// Fixed store ordering for all atomic store operations.
///
/// Loading uses `Relaxed` as the default ordering to avoid memory
/// fences that may affect the real-time thread. On demand a stronger
/// load ordering with consume semantics is available. This maps to
/// either `Consume` (only available on ARM/AArch64, no memory fence)
/// or `Acquire` (other architectures, with memory fence) ordering.
///
/// TODO: Writing a shared parameter value should not depend on strong
/// memory ordering guarantees. Therefore the ordering for both load
/// and store operations might be reduced to `Relaxed` if feasible to
/// improve performance.
const ATOMIC_STORE_ORDERING: Ordering = Ordering::Release;

/// Atomic values.
#[derive(Debug, EnumAsInner, derive_more::From)]
pub enum AtomicValue {
    /// [`crate::param::Value::Bool`]
    Bool(AtomicBool),
    /// [`crate::param::Value::I32`]
    I32(AtomicI32),
    /// [`crate::param::Value::U32`]
    U32(AtomicU32),
    /// [`crate::param::Value::F32`]
    F32(AtomicF32),
}

impl AtomicValue {
    pub fn load_bool(&self) -> Option<bool> {
        self.as_bool().map(|atomic| atomic.load(Ordering::Relaxed))
    }

    pub fn load_i32(&self) -> Option<i32> {
        self.as_i32().map(|atomic| atomic.load(Ordering::Relaxed))
    }

    pub fn load_u32(&self) -> Option<u32> {
        self.as_u32().map(|atomic| atomic.load(Ordering::Relaxed))
    }

    pub fn load_f32(&self) -> Option<f32> {
        self.as_f32().map(|atomic| atomic.load(Ordering::Relaxed))
    }

    pub fn load(&self) -> Value {
        match self {
            Self::Bool(atomic) => Value::Bool(atomic.load(Ordering::Relaxed)),
            Self::I32(atomic) => Value::I32(atomic.load(Ordering::Relaxed)),
            Self::U32(atomic) => Value::U32(atomic.load(Ordering::Relaxed)),
            Self::F32(atomic) => Value::F32(atomic.load(Ordering::Relaxed)),
        }
    }

    #[must_use]
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::I32(_) => ValueType::I32,
            Self::U32(_) => ValueType::U32,
            Self::F32(_) => ValueType::F32,
        }
    }

    pub fn load_consume_bool(&self) -> Option<bool> {
        self.as_bool().map(AtomicConsume::load_consume)
    }

    pub fn load_consume_i32(&self) -> Option<i32> {
        self.as_i32().map(AtomicConsume::load_consume)
    }

    pub fn load_consume_u32(&self) -> Option<u32> {
        self.as_u32().map(AtomicConsume::load_consume)
    }

    pub fn load_consume_f32(&self) -> Option<f32> {
        self.as_f32().map(AtomicConsume::load_consume)
    }

    pub fn load_consume(&self) -> Value {
        match self {
            Self::Bool(atomic) => Value::Bool(atomic.load_consume()),
            Self::I32(atomic) => Value::I32(atomic.load_consume()),
            Self::U32(atomic) => Value::U32(atomic.load_consume()),
            Self::F32(atomic) => Value::F32(atomic.load_consume()),
        }
    }

    pub fn store_bool(&self, value: bool) {
        debug_assert_eq!(self.value_type(), ValueType::Bool);
        self.as_bool()
            .expect("bool")
            .store(value, ATOMIC_STORE_ORDERING);
    }

    pub fn store_i32(&self, value: i32) {
        debug_assert_eq!(self.value_type(), ValueType::I32);
        self.as_i32()
            .expect("i32")
            .store(value, ATOMIC_STORE_ORDERING);
    }

    pub fn store_u32(&self, value: u32) {
        debug_assert_eq!(self.value_type(), ValueType::U32);
        self.as_u32()
            .expect("u32")
            .store(value, ATOMIC_STORE_ORDERING);
    }

    pub fn store_f32(&self, value: f32) {
        debug_assert_eq!(self.value_type(), ValueType::F32);
        self.as_f32()
            .expect("f32")
            .store(value, ATOMIC_STORE_ORDERING);
    }

    pub fn swap_bool(&self, value: bool) -> bool {
        debug_assert_eq!(self.value_type(), ValueType::Bool);
        self.as_bool()
            .expect("bool")
            .swap(value, ATOMIC_STORE_ORDERING)
    }

    pub fn swap_i32(&self, value: i32) -> i32 {
        debug_assert_eq!(self.value_type(), ValueType::I32);
        self.as_i32()
            .expect("i32")
            .swap(value, ATOMIC_STORE_ORDERING)
    }

    pub fn swap_u32(&self, value: u32) -> u32 {
        debug_assert_eq!(self.value_type(), ValueType::U32);
        self.as_u32()
            .expect("u32")
            .swap(value, ATOMIC_STORE_ORDERING)
    }

    pub fn swap_f32(&self, value: f32) -> f32 {
        debug_assert_eq!(self.value_type(), ValueType::F32);
        self.as_f32()
            .expect("f32")
            .swap(value, ATOMIC_STORE_ORDERING)
    }

    pub fn store(&self, value: Value) {
        debug_assert_eq!(self.value_type(), value.into());
        match value {
            Value::Bool(value) => self.store_bool(value),
            Value::I32(value) => self.store_i32(value),
            Value::U32(value) => self.store_u32(value),
            Value::F32(value) => self.store_f32(value),
        }
    }

    pub fn swap(&self, value: Value) -> Value {
        debug_assert_eq!(self.value_type(), value.into());
        match value {
            Value::Bool(value) => self.swap_bool(value).into(),
            Value::I32(value) => self.swap_i32(value).into(),
            Value::U32(value) => self.swap_u32(value).into(),
            Value::F32(value) => self.swap_f32(value).into(),
        }
    }
}

impl AtomicConsume for AtomicValue {
    type Val = Value;

    fn load_consume(&self) -> Self::Val {
        Self::load_consume(self)
    }
}

impl From<Value> for AtomicValue {
    fn from(from: Value) -> Self {
        match from {
            Value::Bool(val) => Self::Bool(AtomicBool::new(val)),
            Value::I32(val) => Self::I32(AtomicI32::new(val)),
            Value::U32(val) => Self::U32(AtomicU32::new(val)),
            Value::F32(val) => Self::F32(AtomicF32::new(val)),
        }
    }
}

pub type SharedAtomicValue = Arc<AtomicValue>;
pub type WeakAtomicValue = Weak<AtomicValue>;
