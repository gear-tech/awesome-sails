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

use awesome_sails_access_control_service::{RolesStorage, Service as AccessControlService};
use awesome_sails_utils::storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Default)]
pub struct Program {
    roles: RefCell<RolesStorage>,
}

#[program]
impl Program {
    pub fn new() -> Self {
        let mut storage = RolesStorage::default();
        let deployer = Syscall::message_source();

        storage.grant_initial_admin(deployer);

        Self {
            roles: RefCell::new(storage),
        }
    }

    pub fn access_control(&self) -> AccessControlService<'_, StorageRefCell<'_, RolesStorage>> {
        AccessControlService::new(StorageRefCell::new(&self.roles))
    }
}
