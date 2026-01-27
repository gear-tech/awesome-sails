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

//! Awesome VFT (Vara Fungible Token) service.
//!
//! This standard is direct analog of ERC20 standard.

#![no_std]

use awesome_sails_utils::{
    error::{EmitError, Error},
    math::{Max, NonZero, Zero},
    ok_if,
    pause::PausableRef,
    storage::StorageMut,
};
use awesome_sails_vft_utils::{Allowance, Allowances, Balance, Balances};
use sails_rs::prelude::*;

/// Re-exporting the utils module for easier access.
pub use awesome_sails_vft_utils as utils;

/// Awesome VFT service itself.
pub struct Vft<'a, A = PausableRef<'a, Allowances>, B = PausableRef<'a, Balances>> {
    // Allowances storage.
    allowances: A,
    // Balances storage.
    balances: B,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<A, B> Vft<'_, A, B> {
    /// Constructor for [`Self`].
    pub fn new(allowances: A, balances: B) -> Self {
        Self {
            allowances,
            balances,
            _marker: core::marker::PhantomData,
        }
    }
}

#[service(events = Event)]
impl<A: StorageMut<Item = Allowances>, B: StorageMut<Item = Balances>> Vft<'_, A, B> {
    #[export(unwrap_result)]
    pub fn approve(&mut self, spender: ActorId, value: U256) -> Result<bool, Error> {
        let owner = Syscall::message_source();

        ok_if!(owner == spender, false);

        let approval = Allowance::try_from(value).unwrap_or(Allowance::MAX);
        let value = if approval.is_max() { U256::MAX } else { value };

        let previous = self.allowances.get_mut()?.set(
            owner.try_into()?,
            spender.try_into()?,
            approval,
            Syscall::block_height(),
        )?;

        let changed = previous.map(NonZero::cast).unwrap_or(U256::ZERO) != value;

        if changed {
            self.emit_event(Event::Approval {
                owner,
                spender,
                value,
            })
            .map_err(|_| EmitError)?;
        }

        Ok(changed)
    }

    #[export(unwrap_result)]
    pub fn transfer(&mut self, to: ActorId, value: U256) -> Result<bool, Error> {
        let from = Syscall::message_source();

        ok_if!(from == to || value.is_zero(), false);

        self.balances.get_mut()?.transfer(
            from.try_into()?,
            to,
            Balance::try_from(value)?.try_into()?,
        )?;

        self.emit_event(Event::Transfer { from, to, value })
            .map_err(|_| EmitError)?;

        Ok(true)
    }

    #[export(unwrap_result)]
    pub fn transfer_from(
        &mut self,
        from: ActorId,
        to: ActorId,
        value: U256,
    ) -> Result<bool, Error> {
        let spender = Syscall::message_source();

        if spender == from {
            return self.transfer(to, value);
        }

        ok_if!(from == to || value.is_zero(), false);

        let _from = from.try_into()?;
        let _spender = spender.try_into()?;
        let _value: NonZero<_> = Balance::try_from(value)?.try_into()?;

        self.allowances.get_mut()?.decrease(
            _from,
            _spender,
            _value.non_zero_cast(),
            Syscall::block_height(),
        )?;

        self.balances.get_mut()?.transfer(_from, to, _value)?;

        self.emit_event(Event::Transfer { from, to, value })
            .map_err(|_| EmitError)?;

        Ok(true)
    }

    #[export(unwrap_result)]
    pub fn allowance(&self, owner: ActorId, spender: ActorId) -> Result<U256, Error> {
        let allowance = self
            .allowances
            .get()?
            .get(owner.try_into()?, spender.try_into()?);

        let allowance = if allowance.is_max() {
            U256::MAX
        } else {
            allowance.into()
        };

        Ok(allowance)
    }

    #[export(unwrap_result)]
    pub fn balance_of(&self, account: ActorId) -> Result<U256, Error> {
        Ok(self.balances.get()?.get(account.try_into()?).into())
    }

    #[export(unwrap_result)]
    pub fn total_supply(&self) -> Result<U256, Error> {
        Ok(self.balances.get()?.total_supply())
    }
}

#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    Approval {
        owner: ActorId,
        spender: ActorId,
        value: U256,
    },

    Transfer {
        from: ActorId,
        to: ActorId,
        value: U256,
    },
}
