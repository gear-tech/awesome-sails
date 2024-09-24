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

extern crate alloc;

use alloc::boxed::Box;

pub type BoxedStorage<T> = Box<dyn Storage<Item = T>>;

pub trait StorageAccessor<T> {
    fn get() -> impl Storage<Item = T> + 'static;

    fn boxed() -> BoxedStorage<T> {
        Box::new(Self::get())
    }
}

pub trait Storage {
    type Item;

    fn get(&self) -> &Self::Item;

    fn get_mut(&mut self) -> &mut Self::Item;

    fn take(&mut self) -> Self::Item;
}
