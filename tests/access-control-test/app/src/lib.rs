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
use core::mem::MaybeUninit;
use sails_rs::prelude::*;

// Configuration for 10,000 members and 100 roles support
const ROLES_LIMIT: usize = 101;
const MEMBERS_LIMIT: usize = 10_001;

type RolesStorage = access_control::AccessControlStorage<ROLES_LIMIT, MEMBERS_LIMIT>;

// Static memory for huge state (BSS section) (The wasm-opt optimization failed)
static mut STORAGE: MaybeUninit<RolesStorage> = MaybeUninit::uninit();

pub struct Program;

#[program]
impl Program {
    pub fn new() -> Self {
        let deployer = Syscall::message_source();

        unsafe {
            let storage_ptr = core::ptr::addr_of_mut!(STORAGE) as *mut RolesStorage;

            let storage = &mut *storage_ptr;
            storage.role_count = 0;
            for role in storage.roles.iter_mut() {
                *role = None;
            }

            storage
                .grant_initial_admin(deployer)
                .expect("Failed to grant initial admin");
        }

        Self
    }

    pub fn access_control(
        &self,
    ) -> access_control::AccessControl<'_, ROLES_LIMIT, MEMBERS_LIMIT, &'_ mut RolesStorage> {
        unsafe {
            let storage_ptr = core::ptr::addr_of_mut!(STORAGE) as *mut RolesStorage;
            access_control::AccessControl::new(&mut *storage_ptr)
        }
    }
}
