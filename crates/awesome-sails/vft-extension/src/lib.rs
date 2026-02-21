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

//! Awesome VFT-Extension service.
//!
//! This service extends the standard VFT functionality with additional features such as:
//! - Cleaning up expired allowances.
//! - Transferring the entire balance (`transfer_all`).
//! - Enumerating allowances and balances.
//! - Managing storage shards explicitly.

#![no_std]

use awesome_sails_utils::{
    ensure,
    error::{EmitError, Error},
    math::{Max, NonZero, Zero},
    ok_if,
    pause::PausableRef,
    storage::StorageMut,
};
use awesome_sails_vft::{
    self as vft,
    utils::{Allowances, Balance, Balances},
};
use sails_rs::prelude::*;

/// The VFT Extension service struct.
pub struct VftExtension<
    'a,
    A: StorageMut<Item = Allowances> = PausableRef<'a, Allowances>,
    B: StorageMut<Item = Balances> = PausableRef<'a, Balances>,
> {
    allowances: A,
    balances: B,
    vft: vft::VftExposure<vft::Vft<'a, A, B>>,
}

impl<'a, A: StorageMut<Item = Allowances>, B: StorageMut<Item = Balances>> VftExtension<'a, A, B> {
    /// Creates a new instance of the VFT Extension service.
    ///
    /// # Arguments
    ///
    /// * `allowances` - Storage backend for allowances.
    /// * `balances` - Storage backend for balances.
    /// * `vft` - Exposure of the base VFT service.
    pub fn new(allowances: A, balances: B, vft: vft::VftExposure<vft::Vft<'a, A, B>>) -> Self {
        Self {
            allowances,
            balances,
            vft,
        }
    }
}

#[service]
impl<A: StorageMut<Item = Allowances>, B: StorageMut<Item = Balances>> VftExtension<'_, A, B> {
    /// Allocates the next shard for allowances storage.
    ///
    /// Useful when the current shard is full.
    ///
    /// # Returns
    ///
    /// `true` if a new shard was allocated, `false` otherwise.
    #[export(unwrap_result)]
    pub fn allocate_next_allowances_shard(&mut self) -> Result<bool, Error> {
        Ok(self.allowances.get_mut()?.allocate_next_shard())
    }

    /// Allocates the next shard for balances storage.
    ///
    /// Useful when the current shard is full.
    ///
    /// # Returns
    ///
    /// `true` if a new shard was allocated, `false` otherwise.
    #[export(unwrap_result)]
    pub fn allocate_next_balances_shard(&mut self) -> Result<bool, Error> {
        Ok(self.balances.get_mut()?.allocate_next_shard())
    }

    /// Removes an expired allowance.
    ///
    /// If the allowance from `owner` to `spender` has expired, it is removed to free up storage.
    ///
    /// # Arguments
    ///
    /// * `owner` - The account that granted the allowance.
    /// * `spender` - The account that was granted the allowance.
    ///
    /// # Returns
    ///
    /// `true` if the allowance was removed, `false` otherwise (e.g., if it didn't exist).
    #[export(unwrap_result)]
    pub fn remove_expired_allowance(
        &mut self,
        owner: ActorId,
        spender: ActorId,
    ) -> Result<bool, Error> {
        ok_if!(owner == spender, false);

        let _owner = owner.try_into()?;
        let _spender = spender.try_into()?;

        {
            let mut allowances = self.allowances.get_mut()?;

            let Some((_, (_, expiry))) = (**allowances).get(&(_owner, _spender)) else {
                return Ok(false);
            };

            ensure!(*expiry < Syscall::block_height(), AllowanceNotExpiredError);

            allowances.remove(_owner, _spender);
        }

        // TODO: consider if we need to emit event here.
        self.vft
            .emit_event(vft::Event::Approval {
                owner,
                spender,
                value: U256::zero(),
            })
            .map_err(|_| EmitError)?;

        Ok(true)
    }

    /// Transfers the entire balance from the caller to `to`.
    ///
    /// # Arguments
    ///
    /// * `to` - The recipient of the tokens.
    ///
    /// # Returns
    ///
    /// `true` if any tokens were transferred.
    #[export(unwrap_result)]
    pub fn transfer_all(&mut self, to: ActorId) -> Result<bool, Error> {
        let from = Syscall::message_source();

        ok_if!(from == to, false);

        let value: U256 = self
            .balances
            .get_mut()?
            .transfer_all(from.try_into()?, to.try_into()?)?
            .into();

        ok_if!(value.is_zero(), false);

        self.vft
            .emit_event(vft::Event::Transfer { from, to, value })
            .map_err(|_| EmitError)?;

        Ok(true)
    }

    /// Transfers the entire balance from `from` to `to` using the allowance mechanism.
    ///
    /// The caller must have sufficient allowance.
    ///
    /// # Arguments
    ///
    /// * `from` - The account to transfer tokens from.
    /// * `to` - The recipient of the tokens.
    ///
    /// # Returns
    ///
    /// `true` if any tokens were transferred.
    #[export(unwrap_result)]
    pub fn transfer_all_from(&mut self, from: ActorId, to: ActorId) -> Result<bool, Error> {
        let spender = Syscall::message_source();

        if spender == from {
            return self.transfer_all(to);
        }

        ok_if!(from == to, false);

        let _spender = spender.try_into()?;
        let _from = from.try_into()?;
        let _to = to.try_into()?;

        let value = self.balances.get_mut()?.transfer_all(_from, _to)?;

        ok_if!(value.is_zero(), false);

        let _value = <NonZero<Balance>>::try_from(value)?;

        self.allowances.get_mut()?.decrease(
            _from,
            _spender,
            _value.non_zero_cast(),
            Syscall::block_height(),
        )?;

        self.vft
            .emit_event(vft::Event::Transfer {
                from,
                to,
                value: value.into(),
            })
            .map_err(|_| EmitError)?;

        Ok(true)
    }

    /// Returns the allowance detail (amount and expiration block) for a given owner and spender.
    ///
    /// # Arguments
    ///
    /// * `owner` - The account owning the tokens.
    /// * `spender` - The account allowed to spend the tokens.
    ///
    /// # Returns
    ///
    /// An `Option` containing a tuple `(U256, u32)` representing the amount and expiration block height.
    #[export(unwrap_result)]
    pub fn allowance_of(
        &self,
        owner: ActorId,
        spender: ActorId,
    ) -> Result<Option<(U256, u32)>, Error> {
        Ok((**self.allowances.get()?)
            .get(&(owner.try_into()?, spender.try_into()?))
            .map(|(_, &(v, b))| {
                let approval = if v.is_max() { U256::MAX } else { (*v).into() };

                (approval, b)
            }))
    }

    /// Returns a list of all allowances with pagination.
    ///
    /// # Arguments
    ///
    /// * `cursor` - The index to start from.
    /// * `len` - The number of items to return.
    ///
    /// # Returns
    ///
    /// A vector of allowance details.
    #[allow(clippy::type_complexity)]
    #[export(unwrap_result)]
    pub fn allowances(
        &self,
        cursor: u32,
        len: u32,
    ) -> Result<Vec<((ActorId, ActorId), (U256, u32))>, Error> {
        Ok(self
            .allowances
            .get()?
            .iter()
            .skip(cursor as usize)
            .take(len as usize)
            .map(|(&(owner, spender), &(allowance, b))| {
                ((owner.into(), spender.into()), ((*allowance).into(), b))
            })
            .collect())
    }

    /// Returns the balance of an account, if it exists in storage.
    ///
    /// Unlike `vft::balance_of` which returns 0 for non-existent accounts, this returns `None`.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to query.
    ///
    /// # Returns
    ///
    /// An `Option<U256>` containing the balance.
    #[export(unwrap_result)]
    pub fn balance_of(&self, account: ActorId) -> Result<Option<U256>, Error> {
        Ok((**self.balances.get()?)
            .get(&account.try_into()?)
            .map(|(_, &v)| (*v).into()))
    }

    /// Returns a list of all balances with pagination.
    ///
    /// # Arguments
    ///
    /// * `cursor` - The index to start from.
    /// * `len` - The number of items to return.
    ///
    /// # Returns
    ///
    /// A vector of `(ActorId, U256)` pairs.
    #[export(unwrap_result)]
    pub fn balances(&self, cursor: u32, len: u32) -> Result<Vec<(ActorId, U256)>, Error> {
        Ok(self
            .balances
            .get()?
            .iter()
            .skip(cursor as usize)
            .take(len as usize)
            .map(|(&account, &v)| (account.into(), (*v).into()))
            .collect())
    }

    /// Returns the configured allowance expiry period.
    #[export(unwrap_result)]
    pub fn expiry_period(&self) -> Result<u32, Error> {
        Ok(self.allowances.get()?.expiry_period())
    }

    /// Returns the amount of value (tokens) that are currently "unused" or reserved.
    #[export(unwrap_result)]
    pub fn unused_value(&self) -> Result<U256, Error> {
        Ok(self.balances.get()?.unused_value())
    }
}

/// Error indicating that an attempt was made to remove an allowance that has not yet expired.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("allowance is not expired")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct AllowanceNotExpiredError;
