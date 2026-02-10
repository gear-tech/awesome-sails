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

//! Awesome Access Control service (Linear Iteration version).

#![no_std]

pub use awesome_sails_utils::ensure;

use crate::error::{AccessDenied, CapacityExceeded, EmitError, Error, NotAccountOwner};
use awesome_sails_utils::storage::InfallibleStorageMut;
use core::marker::PhantomData;
use sails_rs::prelude::*;

pub const ROLE_ID_SIZE: usize = 32;
pub type RoleId = [u8; ROLE_ID_SIZE];

pub const fn default_admin_role() -> RoleId {
    [0u8; ROLE_ID_SIZE]
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct AccessControlStorage<const N: usize, const M: usize> {
    pub role_count: u32,
    pub roles: [Option<RoleEntry<M>>; N],
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct RoleEntry<const M: usize> {
    pub role_id: RoleId,
    pub admin_role_id: RoleId,
    pub member_count: u32,
    pub members: [Option<ActorId>; M],
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Pagination {
    pub offset: u32,
    pub limit: u32,
}

impl Pagination {
    fn range(query: Option<Self>) -> (usize, usize) {
        query
            .map(|q| (q.offset as usize, q.limit as usize))
            .unwrap_or((0, usize::MAX))
    }
}

impl<const M: usize> RoleEntry<M> {
    fn find_member_idx(&self, actor_id: ActorId) -> Option<usize> {
        self.members.iter().flatten().position(|&m| m == actor_id)
    }

    fn find_empty_slot(&self) -> Option<usize> {
        self.members.iter().position(|opt| opt.is_none())
    }

    pub fn has_member(&self, actor_id: ActorId) -> bool {
        self.find_member_idx(actor_id).is_some()
    }

    fn add_member(&mut self, actor_id: ActorId) -> Result<bool, Error> {
        if self.has_member(actor_id) {
            return Ok(false);
        }
        let idx = self.find_empty_slot().ok_or(CapacityExceeded)?;
        self.members[idx] = Some(actor_id);
        self.member_count += 1;
        Ok(true)
    }

    fn remove_member(&mut self, actor_id: ActorId) -> bool {
        if let Some(idx) = self.find_member_idx(actor_id) {
            self.members[idx] = None;
            self.member_count -= 1;
            return true;
        }
        false
    }
}

impl<const N: usize, const M: usize> Default for AccessControlStorage<N, M> {
    fn default() -> Self {
        Self {
            role_count: 0,
            roles: [None; N],
        }
    }
}

impl<const N: usize, const M: usize> AccessControlStorage<N, M> {
    fn find_role_idx(&self, role_id: RoleId) -> Option<usize> {
        self.roles
            .iter()
            .position(|opt| opt.as_ref().is_some_and(|r| r.role_id == role_id))
    }

    fn find_empty_role_slot(&self) -> Option<usize> {
        self.roles.iter().position(|opt| opt.is_none())
    }

    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.find_role_idx(role_id)
            .and_then(|idx| self.roles[idx].as_ref())
            .is_some_and(|r| r.has_member(account_id))
    }

    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.find_role_idx(role_id)
            .and_then(|idx| self.roles[idx].as_ref())
            .map(|r| r.admin_role_id)
            .unwrap_or(default_admin_role())
    }

    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = Pagination::range(query);
        self.roles
            .iter()
            .flatten()
            .map(|e| e.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.find_role_idx(role_id)
            .and_then(|idx| self.roles[idx].as_ref())
            .map(|e| e.member_count)
            .unwrap_or(0)
    }

    pub fn get_role_members(&self, role_id: RoleId, query: Option<Pagination>) -> Vec<ActorId> {
        let (offset, limit) = Pagination::range(query);
        if let Some(role) = self
            .find_role_idx(role_id)
            .and_then(|idx| self.roles[idx].as_ref())
        {
            return role
                .members
                .iter()
                .flatten()
                .copied()
                .skip(offset)
                .take(limit)
                .collect();
        }
        Vec::new()
    }

    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.roles
            .iter()
            .flatten()
            .filter(|e| e.has_member(member_id))
            .count() as u32
    }

    pub fn get_member_roles(&self, member_id: ActorId, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = Pagination::range(query);
        self.roles
            .iter()
            .flatten()
            .filter(|e| e.has_member(member_id))
            .map(|e| e.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    fn ensure_role_mut(&mut self, role_id: RoleId) -> Result<&mut RoleEntry<M>, Error> {
        if let Some(idx) = self.find_role_idx(role_id) {
            return Ok(self.roles[idx].as_mut().expect("Slot is None"));
        }

        let empty_idx = self.find_empty_role_slot().ok_or(CapacityExceeded)?;
        self.roles[empty_idx] = Some(RoleEntry {
            role_id,
            admin_role_id: default_admin_role(),
            member_count: 0,
            members: [None; M],
        });
        self.role_count += 1;
        Ok(self.roles[empty_idx].as_mut().unwrap())
    }

    pub fn grant_initial_admin(&mut self, deployer: ActorId) -> Result<(), Error> {
        self.ensure_role_mut(default_admin_role())?
            .add_member(deployer)
            .map(|_| ())
    }
}

pub struct AccessControl<
    'a,
    const N: usize,
    const M: usize,
    S: InfallibleStorageMut<Item = AccessControlStorage<N, M>>,
> {
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, const N: usize, const M: usize, S: InfallibleStorageMut<Item = AccessControlStorage<N, M>>>
    AccessControl<'a, N, M, S>
{
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            _phantom: PhantomData,
        }
    }

    fn grant_role_unchecked(&mut self, role_id: RoleId, target: ActorId) -> Result<bool, Error> {
        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .add_member(target)
    }

    fn revoke_role_unchecked(&mut self, role_id: RoleId, target: ActorId) -> bool {
        let mut storage = self.storage.get_mut();
        if let Some(idx) = storage.find_role_idx(role_id) {
            return storage.roles[idx].as_mut().unwrap().remove_member(target);
        }
        false
    }

    pub fn require_role(&self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        let storage = self.storage.get();
        let admin = default_admin_role();

        if storage.has_role(admin, account_id)
            || (role_id != admin && storage.has_role(role_id, account_id))
        {
            return Ok(());
        }

        Err(AccessDenied {
            account_id,
            role_id: role_id.to_vec(),
        }
        .into())
    }
}

#[service(events = Event)]
impl<'a, const N: usize, const M: usize, S: InfallibleStorageMut<Item = AccessControlStorage<N, M>>>
    AccessControl<'a, N, M, S>
{
    #[export]
    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        self.storage.get().has_role(role_id, account_id)
    }

    #[export]
    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage.get().get_role_admin(role_id)
    }

    #[export]
    pub fn get_role_count(&self) -> u32 {
        self.storage.get().role_count
    }

    #[export]
    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId> {
        self.storage.get().get_roles(query)
    }

    #[export]
    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        self.storage.get().get_role_member_count(role_id)
    }

    #[export]
    pub fn get_role_members(&self, role_id: RoleId, query: Option<Pagination>) -> Vec<ActorId> {
        self.storage.get().get_role_members(role_id, query)
    }

    #[export]
    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.storage.get().get_member_role_count(member_id)
    }

    #[export]
    pub fn get_member_roles(&self, member_id: ActorId, query: Option<Pagination>) -> Vec<RoleId> {
        self.storage.get().get_member_roles(member_id, query)
    }

    #[export(unwrap_result)]
    pub fn grant_role(&mut self, role_id: RoleId, target_account: ActorId) -> Result<(), Error> {
        let sender = Syscall::message_source();
        self.require_role(self.get_role_admin(role_id), sender)?;

        if self.grant_role_unchecked(role_id, target_account)? {
            self.emit_event(Event::RoleGranted {
                role_id,
                target_account,
                sender,
            })
            .map_err(|_| EmitError)?;
        }
        Ok(())
    }

    #[export(unwrap_result)]
    pub fn grant_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let sender = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), sender)?;
        }
        for role_id in role_ids {
            if self.grant_role_unchecked(role_id, target_account)? {
                self.emit_event(Event::RoleGranted {
                    role_id,
                    target_account,
                    sender,
                })
                .map_err(|_| EmitError)?;
            }
        }
        Ok(())
    }

    #[export(unwrap_result)]
    pub fn revoke_role(&mut self, role_id: RoleId, target_account: ActorId) -> Result<(), Error> {
        let sender = Syscall::message_source();
        self.require_role(self.get_role_admin(role_id), sender)?;

        if self.revoke_role_unchecked(role_id, target_account) {
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account,
                sender,
            })
            .map_err(|_| EmitError)?;
        }
        Ok(())
    }

    #[export(unwrap_result)]
    pub fn revoke_roles_batch(
        &mut self,
        role_ids: Vec<RoleId>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        let sender = Syscall::message_source();
        for &role_id in &role_ids {
            self.require_role(self.get_role_admin(role_id), sender)?;
        }
        for role_id in role_ids {
            if self.revoke_role_unchecked(role_id, target_account) {
                self.emit_event(Event::RoleRevoked {
                    role_id,
                    target_account,
                    sender,
                })
                .map_err(|_| EmitError)?;
            }
        }
        Ok(())
    }

    #[export(unwrap_result)]
    pub fn renounce_role(&mut self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        let sender = Syscall::message_source();
        ensure!(
            account_id == sender,
            NotAccountOwner {
                account_id,
                message_source: sender
            }
        );

        if self.revoke_role_unchecked(role_id, account_id) {
            self.emit_event(Event::RoleRevoked {
                role_id,
                target_account: account_id,
                sender,
            })
            .map_err(|_| EmitError)?;
        }
        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_role_admin(
        &mut self,
        role_id: RoleId,
        new_admin_role_id: RoleId,
    ) -> Result<(), Error> {
        let sender = Syscall::message_source();
        let current_admin = self.get_role_admin(role_id);
        self.require_role(current_admin, sender)?;

        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .admin_role_id = new_admin_role_id;

        self.emit_event(Event::RoleAdminChanged {
            role_id,
            previous_admin_role_id: current_admin,
            new_admin_role_id,
            sender,
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
        sender: ActorId,
    },
}

pub mod error {
    pub use awesome_sails_utils::error::{BadOrigin, EmitError, Error};
    use sails_rs::{
        ActorId,
        prelude::Vec,
        scale_codec::{Decode, Encode},
        scale_info::TypeInfo,
    };

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Access denied: account {account_id:?} does not have role {role_id:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct AccessDenied {
        pub account_id: ActorId,
        pub role_id: Vec<u8>,
    }

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Not account owner: account {account_id:?}, message source {message_source:?}")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct NotAccountOwner {
        pub account_id: ActorId,
        pub message_source: ActorId,
    }

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Capacity exceeded")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct CapacityExceeded;
}
