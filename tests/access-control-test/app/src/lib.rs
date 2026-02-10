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

#![no_std]

use awesome_sails::access_control;
use awesome_sails_utils::storage::{InfallibleStorage, InfallibleStorageMut};
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use sails_rs::prelude::*;

const ROLES_LIMIT: usize = 101;
const MEMBERS_LIMIT: usize = 10_001;

type RolesStorage = access_control::AccessControlStorage<ROLES_LIMIT, MEMBERS_LIMIT>;

static mut STORAGE: MaybeUninit<RolesStorage> = MaybeUninit::uninit();

#[derive(Clone, Copy)]
pub struct RolesStoragePtr;

impl InfallibleStorage for RolesStoragePtr {
    type Item = RolesStorage;
    fn get(&self) -> impl Deref<Target = Self::Item> {
        unsafe { &*core::ptr::addr_of!(STORAGE).cast::<RolesStorage>() }
    }
}

impl InfallibleStorageMut for RolesStoragePtr {
    fn get_mut(&mut self) -> impl DerefMut<Target = Self::Item> {
        unsafe { &mut *core::ptr::addr_of_mut!(STORAGE).cast::<RolesStorage>() }
    }
    fn replace(&mut self, _item: Self::Item) -> Self::Item {
        unimplemented!("Replacing huge storage on stack is not allowed")
    }
}

#[derive(Default)]
pub struct Program;

#[program]
impl Program {
    pub fn new() -> Self {
        let deployer = Syscall::message_source();

        unsafe {
            let storage_ptr = core::ptr::addr_of_mut!(STORAGE).cast::<RolesStorage>();

            // Manual reset for gtest isolation
            let storage = &mut *storage_ptr;
            storage.role_count = 0;
            for role in storage.roles.iter_mut() {
                *role = None;
            }

            storage.grant_initial_admin(deployer);
        }

        Self
    }

    pub fn access_control(
        &self,
    ) -> access_control::AccessControl<'static, ROLES_LIMIT, MEMBERS_LIMIT, RolesStoragePtr> {
        access_control::AccessControl::new(RolesStoragePtr)
    }
}
