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
use awesome_sails_storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

const ROLES_LIMIT: usize = 41;
const MEMBERS_LIMIT: usize = 256;

type RolesStorage = access_control::AccessControlStorage<ROLES_LIMIT, MEMBERS_LIMIT>;

pub struct Program {
    roles: RefCell<RolesStorage>,
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        let deployer = Syscall::message_source();
        let mut storage = RolesStorage::default();

        storage.grant_initial_admin(deployer);

        Self {
            roles: RefCell::new(storage),
        }
    }

    pub fn access_control(
        &self,
    ) -> access_control::AccessControl<
        '_,
        ROLES_LIMIT,
        MEMBERS_LIMIT,
        StorageRefCell<'_, RolesStorage>,
    > {
        access_control::AccessControl::new(StorageRefCell::new(&self.roles))
    }
}
