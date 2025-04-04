// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    hash::{BuildHasher as _, Hash},
    sync::Arc,
};

use atomic::AtomicValue;
use derive_more::{Display, Error};
use hashbrown::{DefaultHashBuilder, HashTable, hash_table::Entry};

use super::{Address, Descriptor, Value, atomic};

const INITIAL_CAPACITY: usize = 1024;

/// Identifier of registered parameters
///
/// Opaque, 0-based, consecutive index that enumerates registered parameters.
///
/// The value is immutable after initial registration. The actual value may vary
/// depending on the order of registration or other circumstances and must neither
/// be hard-coded nor stored persistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[repr(transparent)]
pub struct RegisteredId(usize);

/// Map parameter addresses to their registered identifiers.
#[derive(Debug, Default)]
struct AddressToIdMap {
    hash_table: HashTable<(Address, RegisteredId)>,
    hash_builder: DefaultHashBuilder,
}

impl AddressToIdMap {
    #[must_use]
    fn with_capacity(initial_capacity: usize) -> Self {
        Self {
            hash_table: HashTable::with_capacity(initial_capacity),
            hash_builder: Default::default(),
        }
    }

    fn len(&self) -> usize {
        self.hash_table.len()
    }

    fn iter(&self) -> impl Iterator<Item = (&Address, RegisteredId)> {
        self.hash_table.iter().map(|(address, id)| (address, *id))
    }

    fn find(&self, addressable: &impl AsRef<str>) -> Option<(&Address, RegisteredId)> {
        let Self {
            hash_builder,
            hash_table,
        } = self;
        let hash = hash_builder.hash_one(addressable.as_ref());
        hash_table
            .find(hash, |(address, _id)| {
                address.as_str().eq(addressable.as_ref())
            })
            .map(|(address, id)| (address, *id))
    }

    fn find_or_add(
        &mut self,
        addressable: impl AsRef<str> + Into<Address>,
    ) -> (&Address, RegisteredId) {
        let Self {
            hash_builder,
            hash_table,
        } = self;
        let next_id = RegisteredId(hash_table.len());
        let hash = hash_builder.hash_one(addressable.as_ref());
        match self.hash_table.entry(
            hash,
            |(address, _id)| address.as_str().eq(addressable.as_ref()),
            |(address, _)| hash_builder.hash_one(address),
        ) {
            Entry::Occupied(occupied) => {
                // Clone and reuse the address of the existing entry in O(1) since we
                // do not know what the implementation of Into<Address> actually does.
                let (address, id) = occupied.into_mut();
                debug_assert_eq!(*address, addressable.into());
                (address, *id)
            }
            Entry::Vacant(vacant) => {
                let occupied = vacant.insert((addressable.into(), next_id));
                let (address, id) = occupied.into_mut();
                (address, *id)
            }
        }
    }
}

#[derive(Debug)]
enum RegistryEntry {
    Pending {
        address: Address,
    },
    Ready {
        address: Address,
        descriptor: Descriptor,
        shared_value: Arc<AtomicValue>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct RegistryEntryRef<'a>(&'a RegistryEntry);

impl RegistryEntryRef<'_> {
    #[must_use]
    pub const fn address(&self) -> &'_ Address {
        let Self(entry) = self;
        match entry {
            RegistryEntry::Pending { address } | RegistryEntry::Ready { address, .. } => address,
        }
    }

    #[must_use]
    pub const fn descriptor(&self) -> Option<&'_ Descriptor> {
        let Self(entry) = self;
        match entry {
            RegistryEntry::Pending { address: _ } => None,
            RegistryEntry::Ready { descriptor, .. } => Some(descriptor),
        }
    }

    /// Reads the shared value.
    ///
    /// Uses a "consume" memory ordering, which is supposed to sufficient and might
    /// be faster that an "acquire" memory ordering.
    #[must_use]
    pub fn load_consume_shared_value(&self) -> Option<Value> {
        let Self(entry) = self;
        match entry {
            RegistryEntry::Pending { address: _ } => None,
            RegistryEntry::Ready { shared_value, .. } => Some(shared_value.load_consume()),
        }
    }

    /// Reads the shared value.
    ///
    /// Uses a "relaxed" memory ordering.
    #[must_use]
    pub fn load_relaxed_shared_value(&self) -> Option<Value> {
        let Self(entry) = self;
        match entry {
            RegistryEntry::Pending { address: _ } => None,
            RegistryEntry::Ready { shared_value, .. } => Some(shared_value.load_relaxed()),
        }
    }

    /// Writes the shared value.
    ///
    /// Returns `true` if the shared values exists and has been updated.
    /// Returns `false` if the new value has been discarded.
    ///
    /// # Panics
    ///
    /// Panics if the value type does not match.
    #[expect(clippy::must_use_candidate)]
    pub fn store_shared_value(&self, new_value: Value) -> bool {
        let Self(entry) = self;
        match entry {
            RegistryEntry::Pending { address: _ } => false,
            RegistryEntry::Ready { shared_value, .. } => {
                shared_value.store(new_value);
                true
            }
        }
    }
}

#[derive(Debug, Display, Error)]
pub enum RegistrationError {
    /// The address is already in use and the descriptors differ.
    ///
    /// Could only occur when registering a provider.
    ///
    /// The address and the conflicting descriptor are returned to the caller.
    #[display("address occupied")]
    AddressOccupied {
        address: Address,
        descriptor: Descriptor,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationStatus {
    NewlyRegistered,
    AlreadyRegistered,
}

/// Registration with optional descriptor
#[derive(Debug, Clone, Copy)]
pub struct Registration<'a> {
    status: RegistrationStatus,
    id: RegisteredId,
    entry: &'a RegistryEntry,
}

impl Registration<'_> {
    #[must_use]
    pub const fn status(&self) -> RegistrationStatus {
        self.status
    }

    #[must_use]
    pub const fn id(&self) -> RegisteredId {
        self.id
    }

    #[must_use]
    pub const fn entry(&self) -> RegistryEntryRef<'_> {
        RegistryEntryRef(self.entry)
    }
}

// Intermediate, internal type with borrowed contents
#[derive(Debug)]
struct RegistrationMut<'a> {
    status: RegistrationStatus,
    id: RegisteredId,
    entry: &'a mut RegistryEntry,
}

impl<'a> From<RegistrationMut<'a>> for Registration<'a> {
    fn from(from: RegistrationMut<'a>) -> Self {
        let RegistrationMut { status, id, entry } = from;
        Self { status, id, entry }
    }
}

/// Parameter registry
///
/// Permanently maps addresses to ids and stores metadata
/// about the associated parameters.
#[expect(missing_debug_implementations)]
pub struct Registry {
    address_to_id: AddressToIdMap,
    entries: Vec<RegistryEntry>,
}

const fn registry_entry_index(id: RegisteredId) -> usize {
    let RegisteredId(index) = id;
    index
}

impl Registry {
    pub fn address_to_id_iter(&self) -> impl Iterator<Item = (&Address, RegisteredId)> {
        self.address_to_id.iter()
    }

    fn register(&mut self, addressable: impl AsRef<str> + Into<Address>) -> RegistrationMut<'_> {
        debug_assert_eq!(self.address_to_id.len(), self.entries.len());
        let (address, id) = self.address_to_id.find_or_add(addressable);
        let entry_index = registry_entry_index(id);
        if entry_index < self.entries.len() {
            // Occupied
            debug_assert_eq!(self.address_to_id.len(), self.entries.len());
            #[expect(unsafe_code, reason = "index has just been checked")]
            let entry = unsafe { self.entries.get_unchecked_mut(entry_index) };
            RegistrationMut {
                status: RegistrationStatus::AlreadyRegistered,
                id,
                entry,
            }
        } else {
            // Vacant
            let new_entry = RegistryEntry::Pending {
                address: address.clone(),
            };
            self.entries.push(new_entry);
            debug_assert_eq!(self.address_to_id.len(), self.entries.len());
            let entry = self
                .entries
                .last_mut()
                // Safe unwrap after push
                .expect("entry exists");
            RegistrationMut {
                status: RegistrationStatus::NewlyRegistered,
                id,
                entry,
            }
        }
    }

    /// Register the parameter descriptor for an address.
    ///
    /// Re-registering the same parameter twice registers only a single parameter
    /// if the descriptors match. If the descriptors do not match, a [`RegistrationError`]
    /// is returned.
    ///
    /// For output parameters registering a descriptor adds a shared, atomic
    /// value that is initialized with the default parameter value. The
    /// registry will keep a strong reference to this shared value and
    /// provide it together with the descriptor.
    ///
    /// Addresses strings will be used verbatim as the key.
    pub fn register_descriptor(
        &mut self,
        addressable: impl AsRef<str> + Into<Address>,
        descriptor: Descriptor,
    ) -> Result<Registration<'_>, RegistrationError> {
        let RegistrationMut { status, id, entry } = self.register(addressable);
        match entry {
            RegistryEntry::Pending { address } => {
                log::debug!("Registering descriptor @ {address}: {descriptor:?}");
                let shared_value = Arc::new(AtomicValue::from(descriptor.value.default));
                let address = std::mem::take(address);
                *entry = RegistryEntry::Ready {
                    address,
                    descriptor,
                    shared_value,
                };
                Ok(Registration { status, id, entry })
            }
            RegistryEntry::Ready {
                address,
                descriptor: registered_descriptor,
                shared_value: _,
            } => {
                log::debug!("Descriptor already registered @ {address}: {registered_descriptor:?}");
                if descriptor != *registered_descriptor {
                    return Err(RegistrationError::AddressOccupied {
                        address: address.clone(),
                        descriptor,
                    });
                }
                Ok(Registration { status, id, entry })
            }
        }
    }

    /// Register a parameter address.
    ///
    /// Addresses can be registered at any time, even before the corresponding descriptor
    /// is registered. The descriptor will not be available until it has been registered.
    pub fn register_address(
        &mut self,
        addressable: impl AsRef<str> + Into<Address>,
    ) -> Registration<'_> {
        self.register(addressable).into()
    }

    /// Find the [`RegisteredId`] for the given address(able).
    ///
    /// Returns both a reference to the found [`Address`] (for efficient cloning)
    /// and the corresponding [`RegisteredId`].
    #[must_use]
    pub fn resolve_address(
        &self,
        addressable: &impl AsRef<str>,
    ) -> Option<(&Address, RegisteredId)> {
        self.address_to_id.find(addressable)
    }

    /// Get the entry of a parameter by id.
    #[must_use]
    pub fn get_entry(&self, id: RegisteredId) -> Option<RegistryEntryRef<'_>> {
        self.entries
            .get(registry_entry_index(id))
            .map(RegistryEntryRef)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            // Reserve some extra space in the underlying `HashMap` to reduce collisions
            address_to_id: AddressToIdMap::with_capacity(INITIAL_CAPACITY + INITIAL_CAPACITY / 2),
            entries: Vec::with_capacity(INITIAL_CAPACITY),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use smol_str::SmolStr;

    use crate::param::{Descriptor, Direction, RegistrationStatus, Value, ValueDescriptor};

    use super::{RegisteredId, Registry};

    #[test]
    fn registry() {
        let mut registry = Registry::default();

        let consumer1 = registry.register_address("addr1");
        assert_eq!(consumer1.id(), RegisteredId(0));
        assert_eq!(consumer1.status(), RegistrationStatus::NewlyRegistered);
        assert_eq!(consumer1.entry().address().as_str(), "addr1");
        assert!(consumer1.entry().descriptor().is_none());

        registry.register_address("addr2".to_owned());
        registry.register_address(Cow::Borrowed("addr3"));
        registry.register_address(SmolStr::new_static("addr4"));

        let desc1 = Descriptor {
            name: "name1".into(),
            unit: None,
            direction: Direction::Input,
            value: ValueDescriptor::default(Value::Bool(true)),
        };
        let provider1 = registry
            .register_descriptor("addr1", desc1.clone())
            .unwrap();
        assert_eq!(provider1.id(), RegisteredId(0));
        assert_eq!(provider1.status(), RegistrationStatus::AlreadyRegistered);
        assert_eq!(provider1.entry().address().as_str(), "addr1");
        assert_eq!(provider1.entry().descriptor(), Some(&desc1));
    }
}
