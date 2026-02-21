// This file is part of Gear.

// Copyright (C) 2025 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Awesome Access Control service.
//!
//! This service implements a role-based access control (RBAC) mechanism with support for
//! role hierarchies, enumeration, and batch operations.
//!
//! # Role Hierarchy
//!
//! * **Super Admin (`DEFAULT_ADMIN_ROLE`)**:
//!     * Acts as a **Master Key**: an account with this role passes any `require_role` check,
//!       regardless of the specific role requested.
//!     * Is the default administrator for all new roles.
//!     * Can grant/revoke any role and change any role's administrator.
//!
//! * **Role Admin**:
//!     * Each role has an associated administrator role (by default, the Super Admin role).
//!     * Only accounts with the administrator role can grant or revoke the managed role.
//!     * Administrator roles can be changed via `set_role_admin` to create complex
//!       permission structures.
//!
//! The service uses deterministic storage (`BTreeMap`) and provides methods to enumerate
//! all roles and their members, as well as perform bulk updates via batch functions.

#![no_std]

pub use awesome_sails_utils::ensure;

use crate::error::{AccessDenied, EmitError, Error, NotAccountOwner};
use awesome_sails_utils::storage::{InfallibleStorageMut, StorageRefCell};
use core::marker::PhantomData;
use sails_rs::{
    collections::{BTreeMap, BTreeSet},
    prelude::*,
};

/// Type alias for role identifiers (32-byte array).
pub type RoleId = [u8; 32];

/// The identifier for the default super-admin role.
pub const DEFAULT_ADMIN_ROLE: RoleId = [0u8; 32];

/// Internal storage structure for managing roles and their members.
#[derive(Default, Debug)]
pub struct RolesStorage {
    roles: BTreeMap<RoleId, RoleData>,
}

/// Internal structure holding data for a specific role.
#[derive(Default, Debug)]
pub struct RoleData {
    members: BTreeSet<ActorId>,
    admin_role_id: RoleId,
}

/// Pagination parameters for listing roles or members.
#[derive(Encode, Decode, TypeInfo, Debug, Clone, Copy)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Pagination {
    /// The number of items to skip.
    pub offset: u32,
    /// The maximum number of items to return.
    pub limit: u32,
}

impl RolesStorage {
    /// Checks if an account possesses a specific role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role to check.
    /// * `account_id` - The identifier of the account to check.
    ///
    /// # Returns
    ///
    /// `true` if the account has the role, `false` otherwise.
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.roles
            .get(&role_id)
            .is_some_and(|data| data.members.contains(&account_id))
    }

    /// Retrieves the administrator role for a given role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role.
    ///
    /// # Returns
    ///
    /// The `RoleId` of the administrator role. Returns `DEFAULT_ADMIN_ROLE` if not explicitly set.
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.roles
            .get(&role_id)
            .map(|data| data.admin_role_id)
            .unwrap_or(DEFAULT_ADMIN_ROLE)
    }

    /// Returns the total number of roles defined in the storage.
    pub fn get_role_count(&self) -> u32 {
        self.roles.len() as u32
    }

    /// Retrieves a list of role identifiers, optionally paginated.
    ///
    /// # Arguments
    ///
    /// * `query` - Optional pagination parameters.
    ///
    /// # Returns
    ///
    /// A vector of `RoleId`s.
    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = query
            .map(|q| (q.offset as usize, q.limit as usize))
            .unwrap_or((0, usize::MAX));

        self.roles
            .keys()
            .skip(offset)
            .take(limit)
            .copied()
            .collect()
    }

    /// Returns the number of members assigned to a specific role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role.
    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.roles
            .get(&role_id)
            .map(|data| data.members.len() as u32)
            .unwrap_or_default()
    }

    /// Retrieves a list of members assigned to a specific role, optionally paginated.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role.
    /// * `query` - Optional pagination parameters.
    ///
    /// # Returns
    ///
    /// A vector of `ActorId`s.
    pub fn get_role_members(&self, role_id: RoleId, query: Option<Pagination>) -> Vec<ActorId> {
        let (offset, limit) = query
            .map(|q| (q.offset as usize, q.limit as usize))
            .unwrap_or((0, usize::MAX));

        self.roles
            .get(&role_id)
            .map(|data| {
                data.members
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the number of roles assigned to a specific account.
    ///
    /// # Arguments
    ///
    /// * `member_id` - The identifier of the account.
    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.roles
            .values()
            .filter(|data| data.members.contains(&member_id))
            .count() as u32
    }

    /// Retrieves a list of roles assigned to a specific account, optionally paginated.
    ///
    /// # Arguments
    ///
    /// * `member_id` - The identifier of the account.
    /// * `query` - Optional pagination parameters.
    ///
    /// # Returns
    ///
    /// A vector of `RoleId`s.
    pub fn get_member_roles(&self, member_id: ActorId, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = query
            .map(|q| (q.offset as usize, q.limit as usize))
            .unwrap_or((0, usize::MAX));

        self.roles
            .iter()
            .filter(|(_, data)| data.members.contains(&member_id))
            .map(|(&role_id, _)| role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Grants the `DEFAULT_ADMIN_ROLE` to the specified account.
    ///
    /// Typically used during initialization.
    ///
    /// # Arguments
    ///
    /// * `deployer` - The account to grant the super-admin role to.
    pub fn grant_initial_admin(&mut self, deployer: ActorId) {
        self.roles
            .entry(DEFAULT_ADMIN_ROLE)
            .or_default()
            .members
            .insert(deployer);
    }
}

/// The Access Control service struct.
///
/// Wraps storage and provides RBAC functionality.
pub struct AccessControl<
    'a,
    S: InfallibleStorageMut<Item = RolesStorage> = StorageRefCell<'a, RolesStorage>,
> {
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, S: InfallibleStorageMut<Item = RolesStorage>> AccessControl<'a, S> {
    /// Creates a new instance of the Access Control service.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend used to persist role data.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            _phantom: PhantomData,
        }
    }

    fn grant_role_unchecked(&mut self, role_id: RoleId, target_account: ActorId) -> bool {
        self.storage
            .get_mut()
            .roles
            .entry(role_id)
            .or_default()
            .members
            .insert(target_account)
    }

    fn revoke_role_unchecked(&mut self, role_id: RoleId, target_account: ActorId) -> bool {
        self.storage
            .get_mut()
            .roles
            .get_mut(&role_id)
            .is_some_and(|role_data| role_data.members.remove(&target_account))
    }

    fn set_role_admin_unchecked(&mut self, role_id: RoleId, admin_role_id: RoleId) {
        self.storage
            .get_mut()
            .roles
            .entry(role_id)
            .or_default()
            .admin_role_id = admin_role_id;
    }
}

#[service(events = Event)]
impl<'a, S: InfallibleStorageMut<Item = RolesStorage>> AccessControl<'a, S> {
    /// Checks if `account_id` has been granted `role_id`.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The role identifier.
    /// * `account_id` - The account identifier.
    ///
    /// # Returns
    ///
    /// `true` if the account possesses the role.
    #[export]
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.storage.get().has_role(role_id, account_id)
    }

    /// Returns the admin role ID that controls `role_id`.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The role identifier.
    ///
    /// # Returns
    ///
    /// The `RoleId` of the administrator.
    #[export]
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage.get().get_role_admin(role_id)
    }

    /// Returns the total number of roles in the system.
    #[export]
    pub fn get_role_count(&self) -> u32 {
        self.storage.get().get_role_count()
    }

    /// Returns a list of role IDs with pagination.
    ///
    /// # Arguments
    ///
    /// * `query` - Optional pagination configuration.
    #[export]
    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId> {
        self.storage.get().get_roles(query)
    }

    /// Returns the number of members in the specified role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The role identifier.
    #[export]
    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.storage.get().get_role_member_count(role_id)
    }

    /// Returns a list of members in the specified role with pagination.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The role identifier.
    /// * `query` - Optional pagination configuration.
    #[export]
    pub fn get_role_members(&self, role_id: RoleId, query: Option<Pagination>) -> Vec<ActorId> {
        self.storage.get().get_role_members(role_id, query)
    }

    /// Returns the number of roles assigned to the specified member.
    ///
    /// # Arguments
    ///
    /// * `member_id` - The account identifier.
    #[export]
    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.storage.get().get_member_role_count(member_id)
    }

    /// Returns a list of roles assigned to the specified member with pagination.
    ///
    /// # Arguments
    ///
    /// * `member_id` - The account identifier.
    /// * `query` - Optional pagination configuration.
    #[export]
    pub fn get_member_roles(&self, member_id: ActorId, query: Option<Pagination>) -> Vec<RoleId> {
        self.storage.get().get_member_roles(member_id, query)
    }

    /// Ensures that `account_id` has `role_id` or is a super admin.
    ///
    /// # Requirements
    ///
    /// * `account_id` must have `role_id` or `DEFAULT_ADMIN_ROLE`.
    ///
    /// # Returns
    ///
    /// `Ok(())` if access is granted, otherwise `Err(AccessDenied)`.
    pub fn require_role(&self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        if self.has_role(role_id, account_id) || self.has_role(DEFAULT_ADMIN_ROLE, account_id) {
            Ok(())
        } else {
            Err(AccessDenied {
                account_id,
                role_id,
            }
            .into())
        }
    }

    /// Grants `role_id` to `target_account`.
    ///
    /// If `target_account` had not been already granted `role_id`, emits a `RoleGranted`
    /// event.
    ///
    /// # Requirements
    ///
    /// * The caller must have `role_id`'s admin role.
    #[export(unwrap_result)]
    pub fn grant_role(&mut self, role_id: RoleId, target_account: ActorId) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        self.require_role(self.get_role_admin(role_id), message_source)?;

        if self.grant_role_unchecked(role_id, target_account) {
            self.emit_event(Event::RoleGranted {
                role_id,
                target_account,
                sender: message_source,
            })
            .map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Grants `role_ids` to `target_account`.
    ///
    /// If `target_account` had not been already granted any of the `role_ids`,
    /// emits a `RoleGranted` event for each newly granted role.
    ///
    /// # Requirements
    ///
    /// * The caller must have the admin role for all specified `role_ids`.
    #[export(unwrap_result)]
    pub fn grant_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), message_source)?;
        }

        for role_id in role_ids {
            if self.grant_role_unchecked(role_id, target_account) {
                self.emit_event(Event::RoleGranted {
                    role_id,
                    target_account,
                    sender: message_source,
                })
                .map_err(|_| EmitError)?;
            }
        }

        Ok(())
    }

    /// Revokes `role_id` from `target_account`.
    ///
    /// If `target_account` had been granted `role_id`, emits a `RoleRevoked` event.
    ///
    /// # Requirements
    ///
    /// * The caller must have `role_id`'s admin role.
    #[export(unwrap_result)]
    pub fn revoke_role(&mut self, role_id: RoleId, target_account: ActorId) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        self.require_role(self.get_role_admin(role_id), message_source)?;

        if self.revoke_role_unchecked(role_id, target_account) {
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account,
                sender: message_source,
            })
            .map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Revokes `role_ids` from `target_account`.
    ///
    /// If `target_account` had been granted any of the `role_ids`,
    /// emits a `RoleRevoked` event for each newly revoked role.
    ///
    /// # Requirements
    ///
    /// * The caller must have the admin role for all specified `role_ids`.
    #[export(unwrap_result)]
    pub fn revoke_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), message_source)?;
        }

        for role_id in role_ids {
            if self.revoke_role_unchecked(role_id, target_account) {
                self.emit_event(Event::RoleRevoked {
                    role_id,
                    target_account,
                    sender: message_source,
                })
                .map_err(|_| EmitError)?;
            }
        }

        Ok(())
    }

    /// Revokes `role_id` from the calling account.
    ///
    /// Roles are often managed via `grant_role` and `revoke_role`: this function's
    /// purpose is to provide a mechanism for accounts to lose their privileges
    /// if they are compromised (such as when a trusted device is misplaced).
    ///
    /// If the calling account had been granted `role_id`, emits a `RoleRevoked`
    /// event.
    ///
    /// # Requirements
    ///
    /// * The caller must be `account_id`.
    #[export(unwrap_result)]
    pub fn renounce_role(&mut self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        ensure!(
            account_id == message_source,
            NotAccountOwner {
                account_id,
                message_source,
            }
        );

        if self.revoke_role_unchecked(role_id, account_id) {
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account: account_id,
                sender: message_source,
            })
            .map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Sets `new_admin_role_id` as the admin role for `role_id`.
    ///
    /// Emits a `RoleAdminChanged` event.
    ///
    /// # Requirements
    ///
    /// * The caller must have `role_id`'s admin role.
    #[export(unwrap_result)]
    pub fn set_role_admin(
        &mut self,
        role_id: RoleId,
        new_admin_role_id: RoleId,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        let current_admin_role_id = self.get_role_admin(role_id);
        self.require_role(current_admin_role_id, message_source)?;

        self.set_role_admin_unchecked(role_id, new_admin_role_id);

        self.emit_event(Event::RoleAdminChanged {
            role_id,
            previous_admin_role_id: current_admin_role_id,
            new_admin_role_id,
            sender: message_source,
        })
        .map_err(|_| EmitError)?;

        Ok(())
    }
}

/// Events emitted by the Access Control service.
#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    /// Emitted when `target_account` is granted `role_id`.
    RoleGranted {
        role_id: RoleId,
        target_account: ActorId,
        sender: ActorId,
    },
    /// Emitted when `role_id` is revoked from `target_account`.
    RoleRevoked {
        role_id: RoleId,
        target_account: ActorId,
        sender: ActorId,
    },
    /// Emitted when `new_admin_role_id` is set as the admin role for `role_id`.
    RoleAdminChanged {
        role_id: RoleId,
        previous_admin_role_id: RoleId,
        new_admin_role_id: RoleId,
        sender: ActorId,
    },
}

/// Errors occurring within the Access Control service.
pub mod error {
    use crate::RoleId;
    pub use awesome_sails_utils::error::{BadOrigin, EmitError, Error};
    use sails_rs::{
        ActorId,
        scale_codec::{Decode, Encode},
        scale_info::TypeInfo,
    };

    /// Error indicating access was denied due to missing role permissions.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Access denied: account {account_id:?} does not have role {role_id:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct AccessDenied {
        pub account_id: ActorId,
        pub role_id: RoleId,
    }

    /// Error indicating that an operation required the caller to be the account owner, but they were not.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Not account owner: account {account_id:?}, message source {message_source:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct NotAccountOwner {
        pub account_id: ActorId,
        pub message_source: ActorId,
    }
}
