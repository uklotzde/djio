// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use atomic::AtomicValue;
use derive_more::{Display, Error};

use super::{Address, Descriptor, Direction, SharedAtomicValue, atomic};

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
#[derive(Debug)]
struct AddressToIdMap {
    inner: HashMap<Address, RegisteredId>,
}

impl AddressToIdMap {
    #[must_use]
    fn with_capacity(initial_capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(initial_capacity),
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn iter(&self) -> impl Iterator<Item = (&Address, RegisteredId)> {
        self.inner.iter().map(|(address, &id)| (address, id))
    }

    /// Obtain an id for an address.
    fn get_or_add(&mut self, address: Address) -> (Address, RegisteredId) {
        // The current length must be obtained before the mutable borrow,
        // even if it remains unused.
        let next_id = self.len();
        match self.inner.entry(address) {
            Entry::Occupied(entry) => {
                let id = *entry.get();
                // TODO: Replace needless clone() with entry.replace_key()
                // after #![feature(map_entry_replace)] has been stabilized.
                //let address = entry.replace_key();
                let address = entry.key().clone();
                (address, id)
            }
            Entry::Vacant(entry) => {
                let id = RegisteredId(next_id);
                let address = entry.key().clone();
                entry.insert(id);
                (address, id)
            }
        }
    }

    fn get(&self, address: &Address) -> Option<RegisteredId> {
        self.inner.get(address).map(ToOwned::to_owned)
    }
}

#[derive(Debug)]
pub enum RegistryEntry {
    Pending {
        address: Address,
    },
    Ready {
        address: Address,
        descriptor: Descriptor,
        output_value: Option<SharedAtomicValue>,
    },
}

impl RegistryEntry {
    #[must_use]
    pub const fn address(&self) -> &Address {
        match self {
            Self::Pending { address } | Self::Ready { address, .. } => address,
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
#[derive(Debug)]
pub struct Registration<'a> {
    pub status: RegistrationStatus,
    pub id: RegisteredId,
    pub entry: &'a RegistryEntry,
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

const fn registry_entry_id(param_id: RegisteredId) -> usize {
    let RegisteredId(entry_id) = param_id;
    entry_id
}

impl Registry {
    pub fn address_to_id_iter(&self) -> impl Iterator<Item = (&Address, RegisteredId)> {
        self.address_to_id.iter()
    }

    fn register(&mut self, address: Address) -> RegistrationMut<'_> {
        debug_assert_eq!(self.address_to_id.len(), self.entries.len());
        let (address, id) = self.address_to_id.get_or_add(address);
        let entry_id = registry_entry_id(id);
        if entry_id < self.entries.len() {
            // Occupied
            debug_assert_eq!(self.address_to_id.len(), self.entries.len());
            #[expect(unsafe_code)]
            let entry = unsafe { self.entries.get_unchecked_mut(registry_entry_id(id)) };
            RegistrationMut {
                status: RegistrationStatus::AlreadyRegistered,
                id,
                entry,
            }
        } else {
            // Vacant
            let new_entry = RegistryEntry::Pending { address };
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
        address: Address,
        descriptor: Descriptor,
    ) -> Result<Registration<'_>, RegistrationError> {
        let RegistrationMut { status, id, entry } = self.register(address);
        match entry {
            RegistryEntry::Pending { address } => {
                log::debug!("Registering descriptor @ {address}: {descriptor:?}");
                let output_value = match descriptor.direction {
                    Direction::Input => None,
                    Direction::Output => {
                        Some(Arc::new(AtomicValue::from(descriptor.value.default)))
                    }
                };
                let address = std::mem::take(address);
                *entry = RegistryEntry::Ready {
                    address,
                    descriptor,
                    output_value,
                };
                Ok(Registration { status, id, entry })
            }
            RegistryEntry::Ready {
                address,
                descriptor: registered_descriptor,
                output_value: _,
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
    pub fn register_address(&mut self, address: Address) -> Registration<'_> {
        self.register(address).into()
    }

    /// Find the [`RegisteredId`] for the given address.
    #[must_use]
    pub fn resolve_address(&self, address: &Address) -> Option<RegisteredId> {
        self.address_to_id.get(address)
    }

    /// Get the entry of a parameter by id.
    #[must_use]
    pub fn get_entry(&self, id: RegisteredId) -> Option<&RegistryEntry> {
        self.entries.get(registry_entry_id(id))
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
