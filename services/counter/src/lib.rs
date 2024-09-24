// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
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

use sails_rs::prelude::*;
use utils::{BoxedStorage, Storage, StorageAccessor};

pub struct Service {
    storage: BoxedStorage<u128>,
}

impl Service {
    pub fn new(storage: impl Storage<Item = u128> + 'static) -> Self {
        Self {
            storage: Box::new(storage),
        }
    }

    pub fn from_accessor<T: StorageAccessor<u128>>() -> Self {
        Self {
            storage: T::boxed(),
        }
    }
}

#[service(events = Event)]
impl Service {
    pub fn bump(&mut self) {
        let state = self.storage.get_mut();

        *state = state.saturating_add(1);

        self.notify_on(Event::Bumped).expect("unable to emit event");
    }

    pub fn get(&self) -> u128 {
        *self.storage.get()
    }
}

#[derive(Clone, Debug, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
enum Event {
    Bumped,
}
