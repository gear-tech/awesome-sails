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

//! Awesome Access Control service (Allocation-free Sorted Descriptor version).

#![no_std]

pub use awesome_sails_utils::ensure;

use crate::error::{AccessDenied, CapacityExceeded, EmitError, Error, NotAccountOwner};
use awesome_sails_utils::storage::{InfallibleStorageMut, StorageRefCell};
use core::marker::PhantomData;
use sails_rs::prelude::*;

/// Standard Role ID size.
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
    /// Sorted descriptors (small)
    pub descriptors: [Option<RoleDescriptor>; N],
    /// Fixed data slots (never moved)
    pub role_data: [RoleData<M>; N],
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct RoleDescriptor {
    pub role_id: RoleId,
    pub data_idx: u16,
}

#[derive(Clone, Copy, Debug, Decode, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct RoleData<const M: usize> {
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

impl<const M: usize> Default for RoleData<M> {
    fn default() -> Self {
        Self {
            admin_role_id: default_admin_role(),
            member_count: 0,
            members: [None; M],
        }
    }
}

impl<const N: usize, const M: usize> Default for AccessControlStorage<N, M> {
    fn default() -> Self {
        Self {
            role_count: 0,
            descriptors: [None; N],
            role_data: [const {
                RoleData {
                    admin_role_id: default_admin_role(),
                    member_count: 0,
                    members: [None; M],
                }
            }; N],
        }
    }
}

impl<const N: usize, const M: usize> AccessControlStorage<N, M> {
    fn find_descriptor_idx(&self, role_id: RoleId) -> Result<usize, usize> {
        self.descriptors[..self.role_count as usize].binary_search_by(|opt| {
            opt.as_ref()
                .map(|d| d.role_id.cmp(&role_id))
                .unwrap_or(core::cmp::Ordering::Greater)
        })
    }

    pub fn has_role(&self, role_id: RoleId, account_id: ActorId) -> bool {
        if let Ok(idx) = self.find_descriptor_idx(role_id) {
            let data_idx = self.descriptors[idx].as_ref().unwrap().data_idx as usize;
            return self.role_data[data_idx].has_member(account_id);
        }
        false
    }

    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        if let Ok(idx) = self.find_descriptor_idx(role_id) {
            let data_idx = self.descriptors[idx].as_ref().unwrap().data_idx as usize;
            return self.role_data[data_idx].admin_role_id;
        }
        default_admin_role()
    }

    pub fn get_roles(&self, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = Pagination::range(query);
        self.descriptors
            .iter()
            .flatten()
            .map(|d| d.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    pub fn get_role_member_count(&self, role_id: RoleId) -> u32 {
        if let Ok(idx) = self.find_descriptor_idx(role_id) {
            let data_idx = self.descriptors[idx].as_ref().unwrap().data_idx as usize;
            return self.role_data[data_idx].member_count;
        }
        0
    }

    pub fn get_role_members(&self, role_id: RoleId, query: Option<Pagination>) -> Vec<ActorId> {
        let (offset, limit) = Pagination::range(query);
        if let Ok(idx) = self.find_descriptor_idx(role_id) {
            let data_idx = self.descriptors[idx].as_ref().unwrap().data_idx as usize;
            return self.role_data[data_idx]
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
        self.descriptors
            .iter()
            .flatten()
            .filter(|d| self.role_data[d.data_idx as usize].has_member(member_id))
            .count() as u32
    }

    pub fn get_member_roles(&self, member_id: ActorId, query: Option<Pagination>) -> Vec<RoleId> {
        let (offset, limit) = Pagination::range(query);
        self.descriptors
            .iter()
            .flatten()
            .filter(|d| self.role_data[d.data_idx as usize].has_member(member_id))
            .map(|d| d.role_id)
            .skip(offset)
            .take(limit)
            .collect()
    }

    fn ensure_role_mut(&mut self, role_id: RoleId) -> Result<&mut RoleData<M>, Error> {
        match self.find_descriptor_idx(role_id) {
            Ok(idx) => {
                let data_idx = self.descriptors[idx].as_ref().unwrap().data_idx as usize;
                Ok(&mut self.role_data[data_idx])
            }
            Err(idx) => {
                let count = self.role_count as usize;
                if count >= N {
                    return Err(CapacityExceeded.into());
                }

                let data_idx = count as u16;
                unsafe {
                    let p = self.descriptors.as_mut_ptr().add(idx);
                    core::ptr::copy(p, p.add(1), count - idx);
                    *p = Some(RoleDescriptor { role_id, data_idx });

                    let data = &mut self.role_data[data_idx as usize];
                    data.admin_role_id = default_admin_role();
                    data.member_count = 0;
                }

                self.role_count += 1;
                Ok(&mut self.role_data[data_idx as usize])
            }
        }
    }

    pub fn grant_initial_admin(&mut self, deployer: ActorId) {
        if let Ok(role) = self.ensure_role_mut(default_admin_role()) {
            let _ = role.add_member(deployer);
        }
    }
}

impl<const M: usize> RoleData<M> {
    fn find_member_idx(&self, actor_id: ActorId) -> Result<usize, usize> {
        self.members[..self.member_count as usize].binary_search_by(|opt| {
            opt.as_ref()
                .map(|m| m.cmp(&actor_id))
                .unwrap_or(core::cmp::Ordering::Greater)
        })
    }

    pub fn has_member(&self, actor_id: ActorId) -> bool {
        self.find_member_idx(actor_id).is_ok()
    }

    fn add_member(&mut self, actor_id: ActorId) -> Result<bool, Error> {
        match self.find_member_idx(actor_id) {
            Ok(_) => Ok(false),
            Err(idx) => {
                let count = self.member_count as usize;
                if count >= M {
                    return Err(CapacityExceeded.into());
                }
                unsafe {
                    let p = self.members.as_mut_ptr().add(idx);
                    core::ptr::copy(p, p.add(1), count - idx);
                    *p = Some(actor_id);
                }
                self.member_count += 1;
                Ok(true)
            }
        }
    }

    fn remove_member(&mut self, actor_id: ActorId) -> bool {
        if let Ok(idx) = self.find_member_idx(actor_id) {
            let count = self.member_count as usize;
            unsafe {
                let p = self.members.as_mut_ptr().add(idx);
                core::ptr::copy(p.add(1), p, count - idx - 1);
                self.members[count - 1] = None;
            }
            self.member_count -= 1;
            return true;
        }
        false
    }
}

pub struct AccessControl<
    'a,
    const N: usize,
    const M: usize,
    S: InfallibleStorageMut<Item = AccessControlStorage<N, M>> = StorageRefCell<
        'a,
        AccessControlStorage<N, M>,
    >,
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

    fn grant_role_unchecked(
        &mut self,
        role_id: RoleId,
        target_account: ActorId,
    ) -> Result<bool, Error> {
        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .add_member(target_account)
    }

    fn revoke_role_unchecked(&mut self, role_id: RoleId, target_account: ActorId) -> bool {
        let mut storage = self.storage.get_mut();
        if let Ok(idx) = storage.find_descriptor_idx(role_id) {
            let data_idx = storage.descriptors[idx].as_ref().unwrap().data_idx;
            return storage.role_data[data_idx as usize].remove_member(target_account);
        }
        false
    }

    fn set_role_admin_unchecked(
        &mut self,
        role_id: RoleId,
        admin_role_id: RoleId,
    ) -> Result<(), Error> {
        self.storage
            .get_mut()
            .ensure_role_mut(role_id)?
            .admin_role_id = admin_role_id;
        Ok(())
    }

    pub fn require_role(&self, role_id: RoleId, account_id: ActorId) -> Result<(), Error> {
        let storage = self.storage.get();
        let admin = default_admin_role();

        if storage.role_count > 0
            && storage.descriptors[0].as_ref().is_some_and(|d| {
                d.role_id == admin && storage.role_data[d.data_idx as usize].has_member(account_id)
            })
        {
            return Ok(());
        }

        if role_id != admin && storage.has_role(role_id, account_id) {
            return Ok(());
        }

        Err(AccessDenied {
            account_id,
            role_id,
        }
        .into())
    }

    pub fn get_role_admin(&self, role_id: RoleId) -> RoleId {
        self.storage.get().get_role_admin(role_id)
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
        role_ids: Vec<RoleId>,
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
    pub fn revoke_role(&mut self, role_id: RoleId, target_account: ActorId) -> Result<(), Error> {
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
        role_ids: Vec<RoleId>,
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

    #[export(unwrap_result)]
    pub fn set_role_admin(
        &mut self,
        role_id: RoleId,
        new_admin_role_id: RoleId,
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

    fn perform_role_action<F, E>(
        &mut self,
        role_id: RoleId,
        target: ActorId,
        mut action_fn: F,
        event_fn: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Self, RoleId, ActorId) -> Result<bool, Error>,
        E: FnOnce(RoleId, ActorId, ActorId) -> Event,
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
        role_ids: Vec<RoleId>,
        target: ActorId,
        mut action: F,
        mut event_builder: E,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut Self, RoleId, ActorId) -> Result<bool, Error>,
        E: FnMut(RoleId, ActorId, ActorId) -> Event,
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
        scale_codec::{Decode, Encode},
        scale_info::TypeInfo,
    };

    use crate::RoleId;

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

    #[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[error("Capacity exceeded")]
    #[scale_info(crate = sails_rs::scale_info)]
    pub struct CapacityExceeded;
}
