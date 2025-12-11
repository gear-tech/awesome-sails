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
    math::{CustomUint, Max},
};
use sails_rs::{Decode, Encode, TypeInfo};

mod allowances;
mod balances;

pub use allowances::{Allowances, AllowancesError, AllowancesKey, AllowancesValue};
pub use balances::{Balances, BalancesError};

// --- ALLOWANCE ---

#[derive(Clone, Copy, Debug, Default, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Allowance(CustomUint<72, 2>);

impl_math_wrapper!(Allowance, CustomUint<72, 2>);

impl From<Balance> for Allowance {
    fn from(value: Balance) -> Self {
        Self(value.0.try_resize().unwrap_or(CustomUint::<72, 2>::MAX))
    }
}

impl From<CustomUint<72, 2>> for Allowance {
    fn from(v: CustomUint<72, 2>) -> Self {
        Self(v)
    }
}
impl From<Allowance> for CustomUint<72, 2> {
    fn from(v: Allowance) -> Self {
        v.0
    }
}

// --- BALANCE ---

#[derive(Clone, Copy, Debug, Default, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Balance(CustomUint<80, 2>);

impl_math_wrapper!(Balance, CustomUint<80, 2>);

impl From<u64> for Balance {
    fn from(value: u64) -> Self {
        Self(CustomUint::<80, 2>::from(value))
    }
}

// NOTE: From<Balance> for u64 removed to avoid panic on overflow in tests. Use u128.

impl From<CustomUint<80, 2>> for Balance {
    fn from(v: CustomUint<80, 2>) -> Self {
        Self(v)
    }
}
impl From<Balance> for CustomUint<80, 2> {
    fn from(v: Balance) -> Self {
        v.0
    }
}
