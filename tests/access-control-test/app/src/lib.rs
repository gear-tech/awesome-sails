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

use awesome_sails::access_control::{AccessControl, AccessControlStorage};
use awesome_sails_utils::storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

const R: usize = 32;
const N: usize = 10;
const M: usize = 10;

#[derive(Default)]
pub struct Program {
    roles: RefCell<AccessControlStorage<R, N, M>>,
}

#[program]
impl Program {
    pub fn new() -> Self {
        let mut storage = AccessControlStorage::<R, N, M>::default();
        let deployer = Syscall::message_source();

        storage.grant_initial_admin(deployer);

        Self {
            roles: RefCell::new(storage),
        }
    }

    pub fn access_control(
        &self,
    ) -> AccessControl<'_, R, N, M, StorageRefCell<'_, AccessControlStorage<R, N, M>>> {
        AccessControl::new(StorageRefCell::new(&self.roles))
    }
}
