// This file is part of Gear.

// Copyright (C) 2021-2025 Gear Technologies Inc.
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

use awesome_sails_utils::{
    impl_math_wrapper,
    math::{LeBytes, Max},
};
use sails_rs::{Decode, Encode, TypeInfo};

mod allowances;
mod balances;

pub use allowances::{Allowances, AllowancesError, AllowancesKey, AllowancesValue};
pub use balances::{Balances, BalancesError};

// --- ALLOWANCE ---

// 72 bits = 9 bytes
#[derive(Clone, Copy, Debug, Default, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Allowance(LeBytes<9>);

impl_math_wrapper!(Allowance, LeBytes<9>);

impl From<Balance> for Allowance {
    fn from(value: Balance) -> Self {
        // Try resizing from 10 bytes (Balance) to 9 bytes (Allowance)
        Self(value.0.try_resize().unwrap_or(LeBytes::<9>::MAX))
    }
}

// --- BALANCE ---

// 80 bits = 10 bytes
#[derive(Clone, Copy, Debug, Default, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Balance(LeBytes<10>);

impl_math_wrapper!(Balance, LeBytes<10>);

impl From<u64> for Balance {
    fn from(value: u64) -> Self {
        Self(LeBytes::<10>::from(value))
    }
}
