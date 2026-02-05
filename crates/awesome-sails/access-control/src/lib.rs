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

//! Awesome Access Control service (Allocation-free version).

#![no_std]

pub use awesome_sails_utils::ensure;

use crate::error::{AccessDenied, CapacityExceeded, EmitError, Error, NotAccountOwner};
use awesome_sails_utils::storage::{InfallibleStorageMut, StorageRefCell};
use core::marker::PhantomData;
use sails_rs::prelude::*;

/// Standard Role ID size used for the public API.
pub const ROLE_ID_32: usize = 32;

pub type RoleId<const R: usize> = [u8; R];

pub const fn default_admin_role<const R: usize>() -> RoleId<R> {
    [0u8; R]
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct AccessControlStorage<const R: usize, const N: usize, const M: usize> {
    pub role_count: u32,
    pub roles: [Option<RoleEntry<R, M>>; N],
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct RoleEntry<const R: usize, const M: usize> {
    pub role_id: RoleId<R>,
    pub admin_role_id: RoleId<R>,
    pub member_count: u32,
    pub members: [Option<ActorId>; M],
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, Copy)]
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

impl<const R: usize, const M: usize> RoleEntry<R, M> {
    fn has_member(&self, actor_id: ActorId) -> bool {
        self.members.iter().flatten().any(|&m| m == actor_id)
    }

    fn add_member(&mut self, actor_id: ActorId) -> Result<bool, Error> {
        if self.has_member(actor_id) {
            return Ok(false);
        }

        let slot = self
            .members
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(CapacityExceeded)?;

        *slot = Some(actor_id);
        self.member_count += 1;
        Ok(true)
    }

    fn remove_member(&mut self, actor_id: ActorId) -> bool {
        if let Some(slot) = self
            .members
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|&m| m == actor_id))
        {
            *slot = None;
            self.member_count -= 1;
            return true;
        }
        false
    }
}

impl<const R: usize, const N: usize, const M: usize> Default for AccessControlStorage<R, N, M> {
    fn default() -> Self {
        Self {
            role_count: 0,
            roles: [None; N],
        }
    }
}

impl<const R: usize, const N: usize, const M: usize> AccessControlStorage<R, N, M> {
    pub fn has_role(&self, role_id: RoleId<R>, account_id: ActorId) -> bool {
        self.roles
            .iter()
            .flatten()
            .any(|e| e.role_id == role_id && e.has_member(account_id))
    }

    pub fn get_role_admin(&self, role_id: RoleId<R>) -> RoleId<R> {
        self.roles
            .iter()
            .flatten()
            .find(|e| e.role_id == role_id)
            .map(|e| e.admin_role_id)
            .unwrap_or(default_admin_role::<R>())
    }

    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId<R>> {
        let (offset, limit) = Pagination::range(query);
        self.roles
            .iter()
            .flatten()
            .map(|e| e.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    pub fn get_role_member_count(&self, role_id: RoleId<R>) -> u32 {
        self.roles
            .iter()
            .flatten()
            .find(|e| e.role_id == role_id)
            .map(|e| e.member_count)
            .unwrap_or(0)
    }

    pub fn get_role_members(&self, role_id: RoleId<R>, query: Option<Pagination>) -> Vec<ActorId> {
        let (offset, limit) = Pagination::range(query);
        self.roles
            .iter()
            .flatten()
            .find(|e| e.role_id == role_id)
            .into_iter()
            .flat_map(|e| e.members.iter().flatten().copied())
            .skip(offset)
            .take(limit)
            .collect()
    }

    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.roles
            .iter()
            .flatten()
            .filter(|e| e.has_member(member_id))
            .count() as u32
    }

    pub fn get_member_roles(
        &self,
        member_id: ActorId,
        query: Option<Pagination>,
    ) -> Vec<RoleId<R>> {
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

    fn find_role_mut(&mut self, role_id: RoleId<R>) -> Option<&mut RoleEntry<R, M>> {
        self.roles
            .iter_mut()
            .flatten()
            .find(|entry| entry.role_id == role_id)
    }

    fn ensure_role_mut(&mut self, role_id: RoleId<R>) -> Result<&mut RoleEntry<R, M>, Error> {
        let existing_idx = self
            .roles
            .iter()
            .position(|s| s.as_ref().is_some_and(|e| e.role_id == role_id));

        if let Some(idx) = existing_idx {
            return Ok(self.roles[idx].as_mut().ok_or(CapacityExceeded)?);
        }

        let empty_idx = self
            .roles
            .iter()
            .position(|slot| slot.is_none())
            .ok_or(CapacityExceeded)?;

        self.roles[empty_idx] = Some(RoleEntry {
            role_id,
            admin_role_id: default_admin_role::<R>(),
            member_count: 0,
            members: [None; M],
        });
        self.role_count += 1;

        self.roles[empty_idx]
            .as_mut()
            .ok_or(CapacityExceeded.into())
    }

    pub fn grant_initial_admin(&mut self, deployer: ActorId) {
        if let Ok(entry) = self.ensure_role_mut(default_admin_role::<R>()) {
            let _ = entry.add_member(deployer);
        }
    }
}

pub struct AccessControl<
    'a,
    const R: usize,
    const N: usize,
    const M: usize,
    S: InfallibleStorageMut<Item = AccessControlStorage<R, N, M>> = StorageRefCell<
        'a,
        AccessControlStorage<R, N, M>,
    >,
> {
    storage: S,
    _phantom: PhantomData<&'a ()>,
}

impl<
    'a,
    const R: usize,
    const N: usize,
    const M: usize,
    S: InfallibleStorageMut<Item = AccessControlStorage<R, N, M>>,
> AccessControl<'a, R, N, M, S>
{
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            _phantom: PhantomData,
        }
    }

    // --- Base Logic (Internal & Shared) ---

    fn grant_role_unchecked(
        &mut self,
        role_id: RoleId<R>,
        target_account: ActorId,
    ) -> Result<bool, Error> {
        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .add_member(target_account)
    }

    fn revoke_role_unchecked(&mut self, role_id: RoleId<R>, target_account: ActorId) -> bool {
        self.storage
            .get_mut()
            .find_role_mut(role_id)
            .is_some_and(|e| e.remove_member(target_account))
    }

    fn set_role_admin_unchecked(
        &mut self,
        role_id: RoleId<R>,
        admin_role_id: RoleId<R>,
    ) -> Result<(), Error> {
        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .admin_role_id = admin_role_id;
        Ok(())
    }

    pub fn require_role(&self, role_id: RoleId<R>, account_id: ActorId) -> Result<(), Error> {
        let storage = self.storage.get();
        if storage.has_role(role_id, account_id)
            || storage.has_role(default_admin_role::<R>(), account_id)
        {
            Ok(())
        } else {
            Err(AccessDenied {
                account_id,
                role_id: role_id.as_ref().to_vec(),
            }
            .into())
        }
    }

    pub fn get_role_admin(&self, role_id: RoleId<R>) -> RoleId<R> {
        self.storage.get().get_role_admin(role_id)
    }
}

#[service(events = Event)]
impl<
    'a,
    const N: usize,
    const M: usize,
    S: InfallibleStorageMut<Item = AccessControlStorage<ROLE_ID_32, N, M>>,
> AccessControl<'a, ROLE_ID_32, N, M, S>
{
    // --- Public API ---

    #[export]
    pub fn has_role(&self, role_id: RoleId<ROLE_ID_32>, account_id: ActorId) -> bool {
        self.require_role(role_id, account_id).is_ok()
    }

    #[export]
    pub fn get_role_admin(&self, role_id: RoleId<ROLE_ID_32>) -> RoleId<ROLE_ID_32> {
        self.storage.get().get_role_admin(role_id)
    }

    #[export]
    pub fn get_role_count(&self) -> u32 {
        self.storage.get().role_count
    }

    #[export]
    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId<ROLE_ID_32>> {
        self.storage.get().get_roles(query)
    }

    #[export]
    pub fn get_role_member_count(&self, role_id: RoleId<ROLE_ID_32>) -> u32 {
        self.storage.get().get_role_member_count(role_id)
    }

    #[export]
    pub fn get_role_members(
        &self,
        role_id: RoleId<ROLE_ID_32>,
        query: Option<Pagination>,
    ) -> Vec<ActorId> {
        self.storage.get().get_role_members(role_id, query)
    }

    #[export]
    pub fn get_member_role_count(&self, member_id: ActorId) -> u32 {
        self.storage.get().get_member_role_count(member_id)
    }

    #[export]
    pub fn get_member_roles(
        &self,
        member_id: ActorId,
        query: Option<Pagination>,
    ) -> Vec<RoleId<ROLE_ID_32>> {
        self.storage.get().get_member_roles(member_id, query)
    }

    #[export(unwrap_result)]
    pub fn grant_role(
        &mut self,
        role_id: RoleId<ROLE_ID_32>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        self.perform_role_action(
            role_id,
            target_account,
            |svc, r, t| svc.grant_role_unchecked(r, t),
            |r, t, s| Event::RoleGranted {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
    }

    #[export(unwrap_result)]
    pub fn grant_roles_batch(
        &mut self,
        role_ids: Vec<RoleId<ROLE_ID_32>>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        self.process_batch(
            role_ids,
            target_account,
            |svc, r, t| svc.grant_role_unchecked(r, t),
            |r, t, s| Event::RoleGranted {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
    }

    #[export(unwrap_result)]
    pub fn revoke_role(
        &mut self,
        role_id: RoleId<ROLE_ID_32>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        self.perform_role_action(
            role_id,
            target_account,
            |svc, r, t| Ok(svc.revoke_role_unchecked(r, t)),
            |r, t, s| Event::RoleRevoked {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
    }

    #[export(unwrap_result)]
    pub fn revoke_roles_batch(
        &mut self,
        role_ids: Vec<RoleId<ROLE_ID_32>>,
        target_account: ActorId,
    ) -> Result<(), Error> {
        self.process_batch(
            role_ids,
            target_account,
            |svc, r, t| Ok(svc.revoke_role_unchecked(r, t)),
            |r, t, s| Event::RoleRevoked {
                role_id: r,
                target_account: t,
                sender: s,
            },
        )
    }

    #[export(unwrap_result)]
    pub fn renounce_role(
        &mut self,
        role_id: RoleId<ROLE_ID_32>,
        account_id: ActorId,
    ) -> Result<(), Error> {
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

    #[export(unwrap_result)]
    pub fn set_role_admin(
        &mut self,
        role_id: RoleId<ROLE_ID_32>,
        new_admin_role_id: RoleId<ROLE_ID_32>,
    ) -> Result<(), Error> {
        let message_source = Syscall::message_source();
        let current_admin_role_id = self.get_role_admin(role_id);
        self.require_role(current_admin_role_id, message_source)?;

        self.set_role_admin_unchecked(role_id, new_admin_role_id)?;

        self.emit_event(Event::RoleAdminChanged {
            role_id,
            previous_admin_role_id: current_admin_role_id,
            new_admin_role_id,
            sender: message_source,
        })
        .map_err(|_| EmitError)?;

        Ok(())
    }

    // --- Private Emitters ---

    fn perform_role_action<F, E>(
        &mut self,
        role_id: RoleId<ROLE_ID_32>,
        target: ActorId,
        mut action_fn: F,
        event_fn: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Self, RoleId<ROLE_ID_32>, ActorId) -> Result<bool, Error>,
        E: FnOnce(RoleId<ROLE_ID_32>, ActorId, ActorId) -> Event,
    {
        let message_source = Syscall::message_source();
        let admin_role = self.get_role_admin(role_id);
        self.require_role(admin_role, message_source)?;

        if action_fn(self, role_id, target)? {
            let event = event_fn(role_id, target, message_source);
            self.emit_event(event).map_err(|_| EmitError)?;
        }
        Ok(())
    }

    fn process_batch<F, E>(
        &mut self,
        role_ids: Vec<RoleId<ROLE_ID_32>>,
        target: ActorId,
        mut action: F,
        mut event_builder: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Self, RoleId<ROLE_ID_32>, ActorId) -> Result<bool, Error>,
        E: FnMut(RoleId<ROLE_ID_32>, ActorId, ActorId) -> Event,
    {
        let message_source = Syscall::message_source();

        for &role_id in &role_ids {
            let admin_role = self.get_role_admin(role_id);
            self.require_role(admin_role, message_source)?;
        }

        for role_id in role_ids {
            if action(self, role_id, target)? {
                let event = event_builder(role_id, target, message_source);
                self.emit_event(event).map_err(|_| EmitError)?;
            }
        }
        Ok(())
    }
}

#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    RoleGranted {
        role_id: RoleId<ROLE_ID_32>,
        target_account: ActorId,
        sender: ActorId,
    },
    RoleRevoked {
        role_id: RoleId<ROLE_ID_32>,
        target_account: ActorId,
        sender: ActorId,
    },
    RoleAdminChanged {
        role_id: RoleId<ROLE_ID_32>,
        previous_admin_role_id: RoleId<ROLE_ID_32>,
        new_admin_role_id: RoleId<ROLE_ID_32>,
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
    #[error("Access denied: account {account_id:?} does not have role")]
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
