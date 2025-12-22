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
//! This service allows for implementing role-based access control mechanisms
//! for a hierarchy of roles.
//!
//! # Role-Based Access Control (RBAC) Hierarchy
//!
//! * **Super Admin (`DEFAULT_ADMIN_ROLE`)**:
//!     * Is the admin for itself.
//!     * Is the default admin for all new roles.
//!     * Can: Grant/revoke `DEFAULT_ADMIN_ROLE` (to/from others).
//!     * Can: Grant/revoke any role (e.g., `MINTER_ROLE`) if that role's admin is `DEFAULT_ADMIN_ROLE`.
//!     * Can: Change the admin of any role (`set_role_admin`).
//!
//! * **Sub-Admin (e.g., `MINTER_ADMIN_ROLE`)**:
//!     * If configured as the admin for `MINTER_ROLE`.
//!     * Can: Grant/revoke `MINTER_ROLE`.
//!     * Cannot: Change the admin of the role (only the role's admin admin can do this).
//!
//! ## FAQ: Can the Super Admin revoke roles from users managed by a Sub-Admin?
//! **Answer:** It depends on the configuration.
//! * If `MINTER_ROLE`'s admin is `DEFAULT_ADMIN_ROLE`: Yes, the Super Admin can do everything.
//! * If `MINTER_ROLE`'s admin was changed to `MINTER_ADMIN_ROLE`:
//!     1. The Super Admin **CANNOT** directly revoke `MINTER_ROLE` (as they lack `MINTER_ADMIN_ROLE`).
//!     2. BUT, the Super Admin **CAN** grant themselves `MINTER_ADMIN_ROLE` (since the admin of `MINTER_ADMIN` is `DEFAULT_ADMIN`).
//!     3. Once they have the sub-admin role, they can revoke the target role.
//!        Alternatively, they can simply change the role's admin back to themselves.

#![no_std]

use awesome_sails_utils::{
    ensure,
    error::{BadOrigin, EmitError, Error},
    storage::{StorageMut, StorageRefCell},
};
use core::marker::PhantomData;
use sails_rs::{collections::BTreeMap, prelude::*};

pub type RoleId = [u8; 32];

pub const DEFAULT_ADMIN_ROLE: RoleId = [0u8; 32];

#[derive(Default, Debug)]
pub struct RolesStorage {
    pub roles: BTreeMap<RoleId, RoleData>,
}

#[derive(Default, Debug)]
pub struct RoleData {
    pub members: BTreeMap<ActorId, bool>,
    pub admin_role_id: RoleId,
}

impl RolesStorage {
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.roles
            .get(&role_id)
            .and_then(|data| data.members.get(&account_id))
            .copied()
            .unwrap_or(false)
    }

    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.roles
            .get(&role_id)
            .map(|data| data.admin_role_id)
            .unwrap_or(DEFAULT_ADMIN_ROLE)
    }
}

pub struct Service<'a, S: StorageMut<Item = RolesStorage> = StorageRefCell<'a, RolesStorage>> {
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, S: StorageMut<Item = RolesStorage>> Service<'a, S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            _phantom: PhantomData,
        }
    }

    /// Grants `role_id` to `target_account`.
    ///
    /// Internal function without access restriction.
    pub fn grant_role_unchecked(
        &mut self,
        role_id: RoleId,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let mut storage = self.storage.get_mut()?;
        let role_data = storage.roles.entry(role_id).or_default();

        if !role_data
            .members
            .get(&target_account)
            .copied()
            .unwrap_or(false)
        {
            role_data.members.insert(target_account, true);
        }
        Ok(())
    }

    /// Revokes `role_id` from `target_account`.
    ///
    /// Internal function without access restriction.
    pub fn revoke_role_unchecked(
        &mut self,
        role_id: RoleId,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let mut storage = self.storage.get_mut()?;
        if let Some(role_data) = storage.roles.get_mut(&role_id)
            && role_data.members.remove(&target_account).is_some()
        {
            // Logic handled
        }
        Ok(())
    }

    /// Sets `admin_role_id` as the admin role for `role_id`.
    ///
    /// Internal function without access restriction.
    pub fn set_role_admin_unchecked(
        &mut self,
        role_id: RoleId,
        admin_role_id: RoleId,
    ) -> Result<(), Error> {
        let mut storage = self.storage.get_mut()?;
        let role_data = storage.roles.entry(role_id).or_default();
        role_data.admin_role_id = admin_role_id;
        Ok(())
    }
}

#[service(events = Event)]
impl<'a, S: StorageMut<Item = RolesStorage>> Service<'a, S> {
    /// Returns `true` if `account_id` has been granted `role_id`.
    #[export]
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.storage
            .get()
            .map(|s| s.has_role(role_id, account_id))
            .unwrap_or(false)
    }

    /// Returns the admin role ID that controls `role_id`.
    #[export]
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage
            .get()
            .map(|s| s.get_role_admin(role_id))
            .unwrap_or(DEFAULT_ADMIN_ROLE)
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
        let admin_role_id = self.get_role_admin(role_id);
        ensure!(
            self.has_role(admin_role_id, Syscall::message_source()),
            BadOrigin
        );

        if !self.has_role(role_id, target_account) {
            self.grant_role_unchecked(role_id, target_account)?;
            self.emit_event(Event::RoleGranted {
                role_id,
                target_account,
                sender: Syscall::message_source(),
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
        let admin_role_id = self.get_role_admin(role_id);
        ensure!(
            self.has_role(admin_role_id, Syscall::message_source()),
            BadOrigin
        );

        if self.has_role(role_id, target_account) {
            self.revoke_role_unchecked(role_id, target_account)?;
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account,
                sender: Syscall::message_source(),
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
        ensure!(account_id == Syscall::message_source(), BadOrigin);

        if self.has_role(role_id, account_id) {
            self.revoke_role_unchecked(role_id, account_id)?;
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account: account_id,
                sender: Syscall::message_source(),
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
        let current_admin_role_id = self.get_role_admin(role_id);
        ensure!(
            self.has_role(current_admin_role_id, Syscall::message_source()),
            BadOrigin
        );

        self.set_role_admin_unchecked(role_id, new_admin_role_id)?;

        self.emit_event(Event::RoleAdminChanged {
            role_id,
            previous_admin_role_id: current_admin_role_id,
            new_admin_role_id,
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
    RoleAdminChanged {
        role_id: RoleId,
        previous_admin_role_id: RoleId,
        new_admin_role_id: RoleId,
    },
}
