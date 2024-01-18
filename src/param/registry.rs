// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use atomic::AtomicValue;
use thiserror::Error;

use super::{atomic, Address, Descriptor, Direction, SharedAtomicValue};

const INITIAL_CAPACITY: usize = 1024;

/// Identifier of registered parameters
///
/// Opaque, 0-based, consecutive index that enumerates registered parameters.
///
/// The value is immutable after initial registration. The actual value may vary
/// depending on the order of registration or other circumstances and must neither
/// be hard-coded nor stored persistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
pub struct RegisteredId(usize);

/// Metadata of a registered parameter
#[derive(Debug, Clone)]
pub struct RegisteredParam<'a> {
    pub descriptor: Descriptor<'a>,
    pub address: Address<'a>,
    pub id: RegisteredId,
}

impl<'a> RegisteredParam<'a> {
    #[must_use]
    pub fn into_owned(self) -> RegisteredParam<'static> {
        let Self {
            descriptor,
            address,
            id,
        } = self;
        RegisteredParam {
            descriptor: descriptor.into_owned(),
            address: address.into_owned(),
            id,
        }
    }
}

/// Map parameter addresses to their registered identifiers.
#[allow(missing_debug_implementations)]
struct AddressToIdMap {
    inner: HashMap<Address<'static>, RegisteredId>,
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

    fn iter(&self) -> impl Iterator<Item = (&Address<'static>, RegisteredId)> {
        self.inner.iter().map(|(address, &id)| (address, id))
    }

    /// Obtain an id for an address.
    fn get_or_add(&mut self, address: Address<'static>) -> (Address<'static>, RegisteredId) {
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

    fn get(&self, address: &Address<'_>) -> Option<RegisteredId> {
        self.inner.get(address).map(ToOwned::to_owned)
    }
}

#[derive(Debug)]
struct RegistryEntry<'a> {
    address: Address<'a>,
    descriptor: Option<Descriptor<'a>>,
    output_value: Option<SharedAtomicValue>,
}

impl<'a> RegistryEntry<'a> {
    fn registration(&self, status: RegistrationStatus, id: RegisteredId) -> Registration<'_> {
        let Self {
            address,
            descriptor,
            output_value,
        } = self;
        debug_assert!(descriptor.is_some() || output_value.is_none());
        let descriptor = descriptor.as_ref().map(|descriptor| RegisteredDescriptor {
            descriptor,
            output_value: output_value.as_ref(),
        });
        Registration {
            header: RegistrationHeader {
                status,
                id,
                address,
            },
            descriptor,
        }
    }
}

#[derive(Debug, Error)]
pub enum RegisterError {
    /// The address is already in use and the descriptors differ.
    ///
    /// Could only occur when registering a provider.
    #[error("address occupied")]
    AddressOccupied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationStatus {
    NewlyRegistered,
    AlreadyRegistered,
}

// Intermediate, internal type with borrowed contents
#[derive(Debug)]
struct RegisteredEntry<'a> {
    status: RegistrationStatus,
    id: RegisteredId,
    entry: &'a mut RegistryEntry<'static>,
}

/// Common properties of registrations
///
/// Contents borrowed from [`Registry`] entries.
#[derive(Debug)]
pub struct RegistrationHeader<'a> {
    pub status: RegistrationStatus,
    pub id: RegisteredId,
    pub address: &'a Address<'a>,
}

/// Registration properties that require a descriptor
///
/// Contents borrowed from [`Registry`] entries.
#[derive(Debug)]
pub struct RegisteredDescriptor<'a> {
    pub descriptor: &'a Descriptor<'a>,

    /// Observable output value
    ///
    /// Should only be written by a single owner who once registered
    /// the corresponding descriptor with [`Registry::register_descriptor()`].
    ///
    /// Readers can observe the shared value by registering for the same
    /// address with [`Registry::register_address()`] after the descriptor
    /// has been registered.
    pub output_value: Option<&'a SharedAtomicValue>,
}

/// Registration with mandatory descriptor
#[derive(Debug)]
pub struct DescriptorRegistration<'a> {
    pub header: RegistrationHeader<'a>,
    pub descriptor: RegisteredDescriptor<'a>,
}

/// Registration with optional descriptor
#[derive(Debug)]
pub struct Registration<'a> {
    pub header: RegistrationHeader<'a>,
    pub descriptor: Option<RegisteredDescriptor<'a>>,
}

/// Parameter registry
///
/// Permanently maps addresses to ids and stores metadata
/// about the associated parameters.
#[allow(missing_debug_implementations)]
pub struct Registry {
    address_to_id: AddressToIdMap,
    entries: Vec<RegistryEntry<'static>>,
}

const fn registry_entry_id(param_id: RegisteredId) -> usize {
    let RegisteredId(entry_id) = param_id;
    entry_id
}

impl Registry {
    pub fn address_to_id_iter(&self) -> impl Iterator<Item = (&Address<'static>, RegisteredId)> {
        self.address_to_id.iter()
    }

    fn register(&mut self, address: Address<'static>) -> RegisteredEntry<'_> {
        debug_assert_eq!(self.address_to_id.len(), self.entries.len());
        let (address, id) = self.address_to_id.get_or_add(address);
        let entry_id = registry_entry_id(id);
        if entry_id < self.entries.len() {
            // Occupied
            debug_assert_eq!(self.address_to_id.len(), self.entries.len());
            #[allow(unsafe_code)]
            let entry = unsafe { self.entries.get_unchecked_mut(registry_entry_id(id)) };
            RegisteredEntry {
                status: RegistrationStatus::AlreadyRegistered,
                id,
                entry,
            }
        } else {
            // Vacant
            let new_entry = RegistryEntry {
                address,
                descriptor: None,
                output_value: None,
            };
            self.entries.push(new_entry);
            debug_assert_eq!(self.address_to_id.len(), self.entries.len());
            let entry = self
                .entries
                .last_mut()
                // Safe unwrap after push
                .expect("entry exists");
            RegisteredEntry {
                status: RegistrationStatus::NewlyRegistered,
                id,
                entry,
            }
        }
    }

    /// Register the parameter descriptor for an address.
    ///
    /// Re-registering the same parameter twice registers only a single parameter
    /// if the descriptors match. If the descriptors do not match, a [`RegisterError`]
    /// is returned.
    ///
    /// For output parameters registering a descriptor adds a shared, atomic
    /// value that is initialized with the default parameter value. The
    /// registry will keep a strong reference to this shared value and
    /// provide it together with the descriptor.
    ///
    /// Addresses strings will be used verbatim as the key.
    #[allow(clippy::missing_panics_doc)]
    pub fn register_descriptor(
        &mut self,
        address: Address<'static>,
        descriptor: Descriptor<'static>,
    ) -> Result<DescriptorRegistration<'_>, RegisterError> {
        let RegisteredEntry { status, id, entry } = self.register(address);
        let RegistryEntry {
            address,
            descriptor: registered_descriptor,
            output_value: registered_output_value,
        } = entry;
        let descriptor = if let Some(registered_descriptor) = registered_descriptor {
            if registered_descriptor != &descriptor {
                return Err(RegisterError::AddressOccupied);
            }
            log::debug!("Descriptor already registered @ {address}: {descriptor:?}");
            registered_descriptor
        } else {
            log::debug!("Registering descriptor @ {address}: {descriptor:?}");
            debug_assert!(registered_output_value.is_none());
            let output_value = match descriptor.direction {
                Direction::Input => None,
                Direction::Output => Some(Arc::new(AtomicValue::from(descriptor.value.default))),
            };
            *registered_descriptor = Some(descriptor);
            *registered_output_value = output_value;
            // Safe unwrap (see above)
            registered_descriptor.as_ref().unwrap()
        };
        Ok(DescriptorRegistration {
            header: RegistrationHeader {
                status,
                id,
                address,
            },
            descriptor: RegisteredDescriptor {
                descriptor,
                output_value: registered_output_value.as_ref(),
            },
        })
    }

    /// Register a parameter address.
    ///
    /// Addresses can be registered at any time, even before the corresponding descriptor
    /// is registered. The descriptor will not be available until it has been registered.
    pub fn register_address(&mut self, address: Address<'static>) -> Registration<'_> {
        let RegisteredEntry { status, id, entry } = self.register(address);
        entry.registration(status, id)
    }

    /// Get the metadata of a parameter by id.
    #[must_use]
    pub fn get_registered(&self, id: RegisteredId) -> Option<Registration<'_>> {
        self.entries
            .get(registry_entry_id(id))
            .map(|entry| entry.registration(RegistrationStatus::AlreadyRegistered, id))
    }

    /// Find the metadata of a parameter by address.
    #[must_use]
    pub fn find_registered(
        &self,
        address: &Address<'_>,
    ) -> Option<(
        RegisteredId,
        Option<&Descriptor<'_>>,
        Option<&SharedAtomicValue>,
    )> {
        self.address_to_id
            .get(address)
            .and_then(|id| {
                self.entries
                    .get(registry_entry_id(id))
                    .map(|entry| (id, entry))
            })
            .map(|(id, entry)| {
                let RegistryEntry {
                    address: entry_address,
                    descriptor,
                    output_value,
                } = entry;
                debug_assert_eq!(address, entry_address);
                (id, descriptor.as_ref(), output_value.as_ref())
            })
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
