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
//! * **Super Admin (`default_admin_role()`)**:
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
//! The service uses stack-optimized hybrid storage (SmallVec) to provide deterministic
//! performance while minimizing heap allocations. It provides methods to enumerate
//! all roles and their members, as well as perform bulk updates via batch functions.

#![no_std]

pub use awesome_sails_utils::ensure;

use crate::error::{
    AccessDenied, CapacityExceeded, EmitError, Error, LastDefaultAdminRoleMember, NotAccountOwner,
};
use core::{cell::RefCell, marker::PhantomData};
use sails_rs::prelude::*;
use smallvec::{Array, SmallVec};

/// Type alias for role identifiers (32-byte array).
pub const ROLE_ID_SIZE: usize = 32;
pub type RoleId = [u8; ROLE_ID_SIZE];

/// The identifier for the default super-admin role.
pub const fn default_admin_role() -> RoleId {
    [0u8; ROLE_ID_SIZE]
}

/// SmallVec inline capacity for roles storage.
///
/// The `+ 1` accounts for the initial admin role.
pub const DEFAULT_ROLES_STACK: usize = 5;
/// SmallVec inline capacity for members storage.
///
/// The `+ 1` accounts for the initial deployer/admin member.
pub const DEFAULT_MEMBERS_STACK: usize = 17;

/// Internal storage structure for managing roles and their members.
#[derive(Clone, Debug, Default)]
pub struct AccessControlState<
    // Maximum global number of roles (total capacity).
    const N: usize,
    // Maximum global number of members per role (total capacity).
    const M: usize,
    // Reserved stack slots for roles. No heap allocation occurs while role count <= RS.
    const RS: usize = DEFAULT_ROLES_STACK,
    // Reserved stack slots for members. No heap allocation occurs while member count <= MS.
    const MS: usize = DEFAULT_MEMBERS_STACK,
> where
    [RoleDescriptor; RS]: Array<Item = RoleDescriptor>,
    [RoleData<M, MS>; RS]: Array<Item = RoleData<M, MS>>,
    [ActorId; MS]: Array<Item = ActorId>,
{
    /// Sorted descriptors for efficient role lookup.
    descriptors: SmallVec<[RoleDescriptor; RS]>,
    /// Indexed data slots for roles (appended on creation).
    /// Uses RS inline slots (no heap allocation if role count <= RS).
    role_data: SmallVec<[RoleData<M, MS>; RS]>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RoleDescriptor {
    role_id: RoleId,
    data_idx: u16,
}

/// Internal structure holding data for a specific role.
#[derive(Clone, Debug, Default)]
pub struct RoleData<const M: usize, const MS: usize>
where
    [ActorId; MS]: Array<Item = ActorId>,
{
    admin_role_id: RoleId,
    members: SmallVec<[ActorId; MS]>,
}

/// Pagination parameters for listing roles or members.
#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo, ReflectHash)]
#[codec(crate = sails_rs::scale_codec)]
#[reflect_hash(crate = sails_rs)]
pub struct Pagination {
    /// The number of items to skip.
    pub offset: u32,
    /// The maximum number of items to return.
    pub limit: u32,
}

impl Pagination {
    fn range(query: Option<Self>) -> (usize, usize) {
        query
            .map(|q| (q.offset as usize, q.limit as usize))
            .unwrap_or((0, usize::MAX))
    }
}

impl<const N: usize, const M: usize, const RS: usize, const MS: usize>
    AccessControlState<N, M, RS, MS>
where
    [RoleDescriptor; RS]: Array<Item = RoleDescriptor>,
    [RoleData<M, MS>; RS]: Array<Item = RoleData<M, MS>>,
    [ActorId; MS]: Array<Item = ActorId>,
{
    fn find_descriptor_idx(&self, role_id: &RoleId) -> Result<usize, usize> {
        if !self.descriptors.is_empty() && &self.descriptors[0].role_id == role_id {
            return Ok(0);
        }
        self.descriptors
            .binary_search_by(|d| d.role_id.cmp(role_id))
    }

    fn get_role_data(&self, role_id: &RoleId) -> Option<&RoleData<M, MS>> {
        self.find_descriptor_idx(role_id)
            .ok()
            .map(|idx| &self.role_data[usize::from(self.descriptors[idx].data_idx)])
    }

    fn get_role_data_mut(&mut self, role_id: &RoleId) -> Option<&mut RoleData<M, MS>> {
        self.find_descriptor_idx(role_id).ok().map(|idx| {
            let data_idx = usize::from(self.descriptors[idx].data_idx);
            &mut self.role_data[data_idx]
        })
    }

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
        self.get_role_data(&role_id)
            .is_some_and(|data| data.has_member(account_id))
    }

    /// Retrieves the administrator role for a given role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role.
    ///
    /// # Returns
    ///
    /// The `RoleId` of the administrator role. Returns `default_admin_role()` if not explicitly set.
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.get_role_data(&role_id)
            .map_or(default_admin_role(), |data| data.admin_role_id)
    }

    /// Returns the total number of roles defined in the storage.
    pub fn get_role_count(&self) -> u32 {
        self.descriptors.len() as u32
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
        let (offset, limit) = Pagination::range(query);
        self.descriptors
            .iter()
            .map(|d| d.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Returns the number of members assigned to a specific role.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The identifier of the role.
    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.get_role_data(&role_id)
            .map_or(0, |data| data.members.len() as u32)
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
        let (offset, limit) = Pagination::range(query);
        self.get_role_data(&role_id).map_or_else(Vec::new, |data| {
            data.members
                .iter()
                .copied()
                .skip(offset)
                .take(limit)
                .collect()
        })
    }

    /// Returns the number of roles assigned to a specific account.
    ///
    /// # Arguments
    ///
    /// * `member_id` - The identifier of the account.
    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.descriptors
            .iter()
            .filter(|d| self.role_data[usize::from(d.data_idx)].has_member(member_id))
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
        let (offset, limit) = Pagination::range(query);
        self.descriptors
            .iter()
            .filter(|d| self.role_data[usize::from(d.data_idx)].has_member(member_id))
            .map(|d| d.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Grants the `default_admin_role()` to the specified account.
    ///
    /// Typically used during initialization.
    ///
    /// # Arguments
    ///
    /// * `deployer` - The account to grant the super-admin role to.
    pub fn grant_initial_admin(&mut self, deployer: ActorId) {
        let role = self
            .ensure_role_mut(default_admin_role())
            .expect("grant_initial_admin: N must be >= 1 to create default admin role");
        let _ = role.add_member(deployer);
    }

    /// Reserves a role descriptor without assigning members.
    ///
    /// Returns `true` if the role was created and `false` if it already existed.
    pub fn reserve_role(&mut self, role_id: RoleId) -> Result<bool, Error> {
        if self.find_descriptor_idx(&role_id).is_ok() {
            return Ok(false);
        }

        self.ensure_role_mut(role_id)?;
        Ok(true)
    }

    fn ensure_role_mut(&mut self, role_id: RoleId) -> Result<&mut RoleData<M, MS>, Error> {
        match self.find_descriptor_idx(&role_id) {
            Ok(idx) => {
                let data_idx = usize::from(self.descriptors[idx].data_idx);
                Ok(&mut self.role_data[data_idx])
            }
            Err(idx) => {
                if self.descriptors.len() >= N {
                    return Err(CapacityExceeded.into());
                }

                let data_idx = u16::try_from(self.role_data.len()).map_err(|_| CapacityExceeded)?;
                self.descriptors
                    .insert(idx, RoleDescriptor { role_id, data_idx });
                self.role_data.push(RoleData {
                    admin_role_id: default_admin_role(),
                    members: SmallVec::new(),
                });

                Ok(&mut self.role_data[usize::from(data_idx)])
            }
        }
    }

    /// Grants `role_id` to `target_account` without an authorization check.
    ///
    /// Returns `true` if the membership changed.
    fn grant_role_inner(
        &mut self,
        role_id: RoleId,
        target_account: ActorId,
    ) -> Result<bool, Error> {
        self.ensure_role_mut(role_id)?.add_member(target_account)
    }

    /// Revokes `role_id` from `target_account` without an authorization check.
    ///
    /// Rejects removing the last member of `default_admin_role()` so the
    /// contract can never be left without a super admin.
    ///
    /// Returns `true` if the membership changed.
    fn revoke_role_inner(
        &mut self,
        role_id: RoleId,
        target_account: ActorId,
    ) -> Result<bool, Error> {
        let Some(data) = self.get_role_data_mut(&role_id) else {
            return Ok(false);
        };

        if role_id == default_admin_role()
            && data.has_member(target_account)
            && data.members.len() == 1
        {
            return Err(LastDefaultAdminRoleMember.into());
        }

        Ok(data.remove_member(target_account))
    }

    /// Sets `admin_role_id` as the admin role for `role_id` without an authorization check.
    fn set_role_admin_inner(
        &mut self,
        role_id: RoleId,
        admin_role_id: RoleId,
    ) -> Result<(), Error> {
        self.ensure_role_mut(role_id)?.admin_role_id = admin_role_id;
        Ok(())
    }
}

impl<const M: usize, const MS: usize> RoleData<M, MS>
where
    [ActorId; MS]: Array<Item = ActorId>,
{
    fn find_member_idx(&self, actor_id: &ActorId) -> Result<usize, usize> {
        self.members.binary_search(actor_id)
    }

    pub fn has_member(&self, actor_id: ActorId) -> bool {
        self.find_member_idx(&actor_id).is_ok()
    }

    fn add_member(&mut self, actor_id: ActorId) -> Result<bool, Error> {
        match self.find_member_idx(&actor_id) {
            Ok(_) => Ok(false),
            Err(idx) => {
                if self.members.len() >= M {
                    return Err(CapacityExceeded.into());
                }
                self.members.insert(idx, actor_id);
                Ok(true)
            }
        }
    }

    fn remove_member(&mut self, actor_id: ActorId) -> bool {
        if let Ok(idx) = self.find_member_idx(&actor_id) {
            self.members.remove(idx);
            return true;
        }
        false
    }
}

/// The Access Control service struct.
///
/// Wraps storage and provides RBAC functionality.
pub struct AccessControl<
    'a,
    const N: usize,
    const M: usize,
    const RS: usize = DEFAULT_ROLES_STACK,
    const MS: usize = DEFAULT_MEMBERS_STACK,
    S: StateMut<Item = AccessControlState<N, M, RS, MS>, Error = Infallible> = &'a RefCell<
        AccessControlState<N, M, RS, MS>,
    >,
> where
    [RoleDescriptor; RS]: Array<Item = RoleDescriptor>,
    [RoleData<M, MS>; RS]: Array<Item = RoleData<M, MS>>,
    [ActorId; MS]: Array<Item = ActorId>,
{
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<
    'a,
    const N: usize,
    const M: usize,
    const RS: usize,
    const MS: usize,
    S: StateMut<Item = AccessControlState<N, M, RS, MS>, Error = Infallible>,
> AccessControl<'a, N, M, RS, MS, S>
where
    [RoleDescriptor; RS]: Array<Item = RoleDescriptor>,
    [RoleData<M, MS>; RS]: Array<Item = RoleData<M, MS>>,
    [ActorId; MS]: Array<Item = ActorId>,
{
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

    /// Ensures that `account_id` has `role_id` or is a super admin.
    ///
    /// # Requirements
    ///
    /// * `account_id` must have `role_id` or `default_admin_role()`.
    ///
    /// # Returns
    ///
    /// `Ok(())` if access is granted, otherwise `Err(AccessDenied)`.
    pub fn require_role(&self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        let storage = self.storage.get();
        let admin = default_admin_role();

        if storage.has_role(admin, account_id) || storage.has_role(role_id, account_id) {
            return Ok(());
        }

        Err(AccessDenied {
            account_id,
            role_id,
        }
        .into())
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
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage.get().get_role_admin(role_id)
    }
}

#[service(events = Event)]
impl<
    'a,
    const N: usize,
    const M: usize,
    const RS: usize,
    const MS: usize,
    S: StateMut<Item = AccessControlState<N, M, RS, MS>, Error = Infallible>,
> AccessControl<'a, N, M, RS, MS, S>
where
    [RoleDescriptor; RS]: Array<Item = RoleDescriptor>,
    [RoleData<M, MS>; RS]: Array<Item = RoleData<M, MS>>,
    [ActorId; MS]: Array<Item = ActorId>,
{
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
        self.perform_role_action(
            role_id,
            target_account,
            |state, r, t| state.grant_role_inner(r, t),
            |r, t, s| Event::RoleGranted {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
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
        self.process_batch(
            role_ids,
            target_account,
            |state, r, t| state.grant_role_inner(r, t),
            |r, t, s| Event::RoleGranted {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
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
        self.perform_role_action(
            role_id,
            target_account,
            |state, r, t| state.revoke_role_inner(r, t),
            |r, t, s| Event::RoleRevoked {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
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
        self.process_batch(
            role_ids,
            target_account,
            |state, r, t| state.revoke_role_inner(r, t),
            |r, t, s| Event::RoleRevoked {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
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

        if self.storage.get_mut().revoke_role_inner(role_id, account_id)? {
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
    /// **Side-effect:** if `role_id` does not exist, it is created with
    /// an empty members list and `default_admin_role()` as initial admin.
    /// This consumes one role slot from the global capacity `N`.
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

        self.storage
            .get_mut()
            .set_role_admin_inner(role_id, new_admin_role_id)?;

        self.emit_event(Event::RoleAdminChanged {
            role_id,
            previous_admin_role_id: current_admin_role_id,
            new_admin_role_id,
            sender: message_source,
        })
        .map_err(|_| EmitError)?;

        Ok(())
    }

    fn perform_role_action<F, E>(
        &mut self,
        role_id: RoleId,
        target: ActorId,
        mut action_fn: F,
        event_fn: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut AccessControlState<N, M, RS, MS>, RoleId, ActorId) -> Result<bool, Error>,
        E: FnOnce(RoleId, ActorId, ActorId) -> Event,
    {
        let message_source = Syscall::message_source();
        let admin_role = self.get_role_admin(role_id);
        self.require_role(admin_role, message_source)?;

        if action_fn(&mut self.storage.get_mut(), role_id, target)? {
            let event = event_fn(role_id, target, message_source);
            self.emit_event(event).map_err(|_| EmitError)?;
        }
        Ok(())
    }

    fn process_batch<F, E>(
        &mut self,
        role_ids: Vec<RoleId>,
        target: ActorId,
        mut action: F,
        mut event_builder: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut AccessControlState<N, M, RS, MS>, RoleId, ActorId) -> Result<bool, Error>,
        E: FnMut(RoleId, ActorId, ActorId) -> Event,
    {
        let message_source = Syscall::message_source();

        for &role_id in &role_ids {
            let admin_role = self.get_role_admin(role_id);
            self.require_role(admin_role, message_source)?;
        }

        // A failure partway through the batch panics out of the `unwrap_result`
        // export; Gear then reverts all storage changes and discards the events
        // emitted for this message, so the batch is atomic without extra work here.
        for role_id in role_ids {
            if action(&mut self.storage.get_mut(), role_id, target)? {
                let event = event_builder(role_id, target, message_source);
                self.emit_event(event).map_err(|_| EmitError)?;
            }
        }

        Ok(())
    }
}

/// Events emitted by the Access Control service.
#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo, ReflectHash)]
#[codec(crate = sails_rs::scale_codec)]
#[reflect_hash(crate = sails_rs)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn role_id(id: usize) -> RoleId {
        let mut bytes = [0; ROLE_ID_SIZE];
        bytes[..core::mem::size_of::<usize>()].copy_from_slice(&id.to_le_bytes());
        bytes
    }

    #[test]
    fn role_data_index_overflow_returns_capacity_exceeded() {
        type State = AccessControlState<{ u16::MAX as usize + 2 }, 1, 1, 1>;

        let mut state = State::default();

        for id in 0..=u16::MAX as usize {
            state.ensure_role_mut(role_id(id)).unwrap();
        }

        let err = match state.ensure_role_mut(role_id(u16::MAX as usize + 1)) {
            Ok(_) => panic!("role index overflow must fail"),
            Err(err) => err,
        };

        assert_eq!(err.to_string(), CapacityExceeded.to_string());
        assert!(
            state
                .get_role_data(&role_id(u16::MAX as usize + 1))
                .is_none()
        );
    }
}

/// Errors occurring within the Access Control service.
pub mod error {
    use crate::RoleId;
    pub use awesome_sails_utils::error::{BadOrigin, EmitError, Error};
    use sails_rs::{
        ActorId, ReflectHash, TypeInfo,
        scale_codec::{Decode, Encode},
    };

    /// Error indicating access was denied due to missing role permissions.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, ReflectHash, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Access denied: account {account_id:?} does not have role {role_id:?}")]
    #[reflect_hash(crate = sails_rs)]
    pub struct AccessDenied {
        pub account_id: ActorId,
        pub role_id: RoleId,
    }

    /// Error indicating that an operation required the caller to be the account owner, but they were not.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, ReflectHash, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Not account owner: account {account_id:?}, message source {message_source:?}")]
    #[reflect_hash(crate = sails_rs)]
    pub struct NotAccountOwner {
        pub account_id: ActorId,
        pub message_source: ActorId,
    }

    /// Error indicating that the storage capacity has been exceeded.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, ReflectHash, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Capacity exceeded")]
    #[reflect_hash(crate = sails_rs)]
    pub struct CapacityExceeded;

    /// Error indicating that the last member of the default admin role cannot be removed.
    #[derive(Clone, Debug, Decode, Encode, TypeInfo, ReflectHash, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Cannot remove last default admin role member")]
    #[reflect_hash(crate = sails_rs)]
    pub struct LastDefaultAdminRoleMember;
}
