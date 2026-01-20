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

pub type RoleId = [u8; 32];

pub const DEFAULT_ADMIN_ROLE: RoleId = [0u8; 32];

#[derive(Default, Debug)]
pub struct RolesStorage {
    roles: BTreeMap<RoleId, RoleData>,
}

#[derive(Default, Debug)]
pub struct RoleData {
    members: BTreeSet<ActorId>,
    admin_role_id: RoleId,
}

impl RolesStorage {
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.roles
            .get(&role_id)
            .is_some_and(|data| data.members.contains(&account_id))
    }

    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.roles
            .get(&role_id)
            .map(|data| data.admin_role_id)
            .unwrap_or(DEFAULT_ADMIN_ROLE)
    }

    pub fn get_role_count(&self) -> u32 {
        self.roles.len() as u32
    }

    pub fn get_role_id(&self, index: u32) -> Option<RoleId> {
        self.roles.keys().nth(index as usize).copied()
    }

    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.roles
            .get(&role_id)
            .map(|data| data.members.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_role_member(&self, role_id: RoleId, index: u32) -> Option<ActorId> {
        self.roles
            .get(&role_id)
            .and_then(|data| data.members.iter().nth(index as usize))
            .copied()
    }

    pub fn grant_initial_admin(&mut self, deployer: ActorId) {
        self.roles
            .entry(DEFAULT_ADMIN_ROLE)
            .or_default()
            .members
            .insert(deployer);
    }
}

pub struct Service<
    'a,
    S: InfallibleStorageMut<Item = RolesStorage> = StorageRefCell<'a, RolesStorage>,
> {
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, S: InfallibleStorageMut<Item = RolesStorage>> Service<'a, S> {
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
            .map_or(false, |role_data| role_data.members.remove(&target_account))
    }

    fn set_role_admin_unchecked(&mut self, role_id: RoleId, admin_role_id: RoleId) {
        self.storage
            .get_mut()
            .roles
            .entry(role_id)
            .or_default()
            .admin_role_id = admin_role_id;
    }

    fn grant_roles_batch_unchecked(
        &mut self,
        role_ids: &[RoleId],
        target_accounts: &[ActorId],
    ) -> bool {
        let mut storage = self.storage.get_mut();
        let mut changed = false;
        for &role_id in role_ids {
            let role_data = storage.roles.entry(role_id).or_default();
            for &target_account in target_accounts {
                changed |= role_data.members.insert(target_account);
            }
        }
        changed
    }

    fn revoke_roles_batch_unchecked(
        &mut self,
        role_ids: &[RoleId],
        target_accounts: &[ActorId],
    ) -> bool {
        let mut storage = self.storage.get_mut();
        let mut changed = false;
        for &role_id in role_ids {
            if let Some(role_data) = storage.roles.get_mut(&role_id) {
                for &target_account in target_accounts {
                    changed |= role_data.members.remove(&target_account);
                }
            }
        }
        changed
    }
}

#[service(events = Event)]
impl<'a, S: InfallibleStorageMut<Item = RolesStorage>> Service<'a, S> {
    /// Returns `true` if `account_id` has been granted `role_id`.
    #[export]
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.storage.get().has_role(role_id, account_id)
    }

    /// Returns the admin role ID that controls `role_id`.
    #[export]
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage.get().get_role_admin(role_id)
    }

    /// Returns the number of roles in the system.
    #[export]
    pub fn get_role_count(&self) -> u32 {
        self.storage.get().get_role_count()
    }

    /// Returns the role ID at the specified index.
    #[export]
    pub fn get_role_id(&self, index: u32) -> Option<RoleId> {
        self.storage.get().get_role_id(index)
    }

    /// Returns the number of members in the specified role.
    #[export]
    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.storage.get().get_role_member_count(role_id)
    }

    /// Returns the member at the specified index in the specified role.
    #[export]
    pub fn get_role_member(&self, role_id: RoleId, index: u32) -> Option<ActorId> {
        self.storage.get().get_role_member(role_id, index)
    }

    /// Ensures that `account_id` has `role_id` or is a super admin.
    ///
    /// Requirements:
    ///
    /// - `account_id` must have `role_id` or `DEFAULT_ADMIN_ROLE`.
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
    /// Requirements:
    ///
    /// - the caller must have `role_id`'s admin role.
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

    /// Grants `role_ids` to `target_accounts`.
    ///
    /// If any of the `target_accounts` had not been already granted any of the `role_ids`,
    /// emits a `RolesGrantedBatch` event.
    ///
    /// Requirements:
    ///
    /// - the caller must have the admin role for all specified `role_ids`.
    #[export(unwrap_result)]
    pub fn grant_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_accounts: Vec<ActorId>,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), message_source)?;
        }

        if self.grant_roles_batch_unchecked(&role_ids, &target_accounts) {
            self.emit_event(Event::RolesGrantedBatch {
                role_ids,
                target_accounts,
                sender: message_source,
            })
            .map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Revokes `role_id` from `target_account`.
    ///
    /// If `target_account` had been granted `role_id`, emits a `RoleRevoked` event.
    ///
    /// Requirements:
    ///
    /// - the caller must have `role_id`'s admin role.
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

    /// Revokes `role_ids` from `target_accounts`.
    ///
    /// If any of the `target_accounts` had been granted any of the `role_ids`,
    /// emits a `RolesRevokedBatch` event.
    ///
    /// Requirements:
    ///
    /// - the caller must have the admin role for all specified `role_ids`.
    #[export(unwrap_result)]
    pub fn revoke_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_accounts: Vec<ActorId>,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), message_source)?;
        }

        if self.revoke_roles_batch_unchecked(&role_ids, &target_accounts) {
            self.emit_event(Event::RolesRevokedBatch {
                role_ids,
                target_accounts,
                sender: message_source,
            })
            .map_err(|_| EmitError)?;
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
    /// Requirements:
    ///
    /// - the caller must be `account_id`.
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
    /// Requirements:
    ///
    /// - the caller must have `role_id`'s admin role.
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

#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    RoleGranted {
        role_id: RoleId,
        target_account: ActorId,
        sender: ActorId,
    },
    RoleRevoked {
        role_id: RoleId,
        target_account: ActorId,
        sender: ActorId,
    },
    RolesGrantedBatch {
        role_ids: Vec<RoleId>,
        target_accounts: Vec<ActorId>,
        sender: ActorId,
    },
    RolesRevokedBatch {
        role_ids: Vec<RoleId>,
        target_accounts: Vec<ActorId>,
        sender: ActorId,
    },
    RoleAdminChanged {
        role_id: RoleId,
        previous_admin_role_id: RoleId,
        new_admin_role_id: RoleId,
        sender: ActorId,
    },
}

pub mod error {
    use crate::RoleId;
    pub use awesome_sails_utils::error::{BadOrigin, EmitError, Error};
    use sails_rs::{
        ActorId,
        scale_codec::{Decode, Encode},
        scale_info::TypeInfo,
    };

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Access denied: account {account_id:?} does not have role {role_id:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct AccessDenied {
        pub account_id: ActorId,
        pub role_id: RoleId,
    }

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Not account owner: account {account_id:?}, message source {message_source:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct NotAccountOwner {
        pub account_id: ActorId,
        pub message_source: ActorId,
    }
}
