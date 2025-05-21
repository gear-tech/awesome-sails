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
//! This service extends default VFT functionality with additional methods.

#![no_std]

use awesome_sails::{
    ensure,
    error::Error,
    math::{Max, NonZero, Zero},
    ok_if,
    storage::Storage,
};
use awesome_sails_vft_service::{
    self as vft,
    utils::{Allowances, Balance, Balances},
};
use sails_rs::{
    ActorId, U256,
    gstd::{exec, msg},
    prelude::*,
};

/// Awesome VFT-Extension service itself.
pub struct Service<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> {
    allowances: A,
    balances: B,
    vft: vft::ServiceExposure<vft::Service<A, B>>,
}

impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Service<A, B> {
    /// Constructor for [`Self`].
    pub fn new(allowances: A, balances: B, vft: vft::ServiceExposure<vft::Service<A, B>>) -> Self {
        Self {
            allowances,
            balances,
            vft,
        }
    }
}

#[service]
impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Service<A, B> {
    #[export(unwrap_result)]
    pub fn allocate_next_allowances_shard(&mut self) -> Result<bool, Error> {
        Ok(self.allowances.get_mut()?.allocate_next_shard())
    }

    #[export(unwrap_result)]
    pub fn allocate_next_balances_shard(&mut self) -> Result<bool, Error> {
        Ok(self.balances.get_mut()?.allocate_next_shard())
    }

    #[export(unwrap_result)]
    pub fn remove_expired_allowance(
        &mut self,
        owner: ActorId,
        spender: ActorId,
    ) -> Result<bool, Error> {
        ok_if!(owner == spender, false);

        let _owner = owner.try_into()?;
        let _spender = spender.try_into()?;

        let mut allowances = self.allowances.get_mut()?;

        let Some((_, (_, expiry))) = (**allowances).get(&(_owner, _spender)) else {
            return Ok(false);
        };

        ensure!(*expiry < exec::block_height(), AllowanceNotExpiredError);

        allowances.remove(_owner, _spender);

        // TODO: consider if we need to emit event here.
        self.vft.emit_event(vft::Event::Approval {
            owner,
            spender,
            value: U256::zero(),
        });

        Ok(true)
    }

    #[export(unwrap_result)]
    pub fn transfer_all(&mut self, to: ActorId) -> Result<bool, Error> {
        let from = msg::source();

        ok_if!(from == to, false);

        let value: U256 = self
            .balances
            .get_mut()?
            .transfer_all(from.try_into()?, to.try_into()?)?
            .into();

        ok_if!(value.is_zero(), false);

        self.vft
            .emit_event(vft::Event::Transfer { from, to, value });

        Ok(true)
    }

    #[export(unwrap_result)]
    pub fn transfer_all_from(&mut self, from: ActorId, to: ActorId) -> Result<bool, Error> {
        let spender = msg::source();

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
            exec::block_height(),
        )?;

        self.vft.emit_event(vft::Event::Transfer {
            from,
            to,
            value: value.into(),
        });

        Ok(true)
    }

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

    #[export(unwrap_result)]
    pub fn balance_of(&self, account: ActorId) -> Result<Option<U256>, Error> {
        Ok((**self.balances.get()?)
            .get(&account.try_into()?)
            .map(|(_, &v)| (*v).into()))
    }

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

    #[export(unwrap_result)]
    pub fn expiry_period(&self) -> Result<u32, Error> {
        Ok(self.allowances.get()?.expiry_period())
    }

    #[export(unwrap_result)]
    pub fn minimum_balance(&self) -> Result<U256, Error> {
        Ok((*self.balances.get()?.minimum_balance()).into())
    }

    #[export(unwrap_result)]
    pub fn unused_value(&self) -> Result<U256, Error> {
        Ok(self.balances.get()?.unused_value())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("allowance is not expired")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct AllowanceNotExpiredError;
