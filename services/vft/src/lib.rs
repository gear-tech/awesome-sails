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

use awesome_sails::{
    error::Error,
    event::Emitter,
    math::{Max, NonZero, Zero},
    ok_if,
    pause::Pausable,
    storage::Storage,
};
use awesome_sails_vft_service_utils::{Allowance, Allowances, Balance, Balances};
use core::cell::RefCell;
use sails_rs::{
    gstd::{exec, msg},
    prelude::*,
};

/// Re-exporting the utils module for easier access.
pub use awesome_sails_vft_service_utils as utils;

/// Awesome VFT service itself.
pub struct Service<'a, A = Pausable<RefCell<Allowances>>, B = Pausable<RefCell<Balances>>> {
    // Allowances storage.
    allowances: &'a A,
    // Balances storage.
    balances: &'a B,
}

impl<'a, A, B> Service<'a, A, B> {
    /// Constructor for [`Self`].
    pub fn new(allowances: &'a A, balances: &'a B) -> Self {
        Self {
            allowances,
            balances,
        }
    }
}

#[service(events = Event)]
impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Service<'_, A, B> {
    #[export(unwrap_result)]
    pub fn approve(&mut self, spender: ActorId, value: U256) -> Result<bool, Error> {
        let owner = msg::source();

        ok_if!(owner == spender, false);

        let approval = Allowance::try_from(value).unwrap_or(Allowance::MAX);
        let value = if approval.is_max() { U256::MAX } else { value };

        let previous = self.allowances.get_mut()?.set(
            owner.try_into()?,
            spender.try_into()?,
            approval,
            exec::block_height(),
        )?;

        let changed = previous.map(NonZero::cast).unwrap_or(U256::ZERO) != value;

        if changed {
            self.emit(Event::Approval {
                owner,
                spender,
                value,
            })?;
        }

        Ok(changed)
    }

    #[export(unwrap_result)]
    pub fn transfer(&mut self, to: ActorId, value: U256) -> Result<bool, Error> {
        let from = msg::source();

        ok_if!(from == to || value.is_zero(), false);

        self.balances.get_mut()?.transfer(
            from.try_into()?,
            to,
            Balance::try_from(value)?.try_into()?,
        )?;

        self.emit(Event::Transfer { from, to, value })?;

        Ok(true)
    }

    #[export(unwrap_result)]
    pub fn transfer_from(
        &mut self,
        from: ActorId,
        to: ActorId,
        value: U256,
    ) -> Result<bool, Error> {
        let spender = msg::source();

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
            exec::block_height(),
        )?;

        self.balances.get_mut()?.transfer(_from, to, _value)?;

        self.emit(Event::Transfer { from, to, value })?;

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

#[derive(Encode, TypeInfo)]
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

/* TODO: DELETE CODE BELOW ONCE APPROPRIATE SAILS CHANGES APPLIED */

impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Emitter for Service<'_, A, B> {
    type Event = Event;

    fn notify(&mut self, event: Self::Event) -> Result<(), sails_rs::errors::Error> {
        self.notify_on(event)
    }
}

impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Emitter
    for ServiceExposure<Service<'_, A, B>>
{
    type Event = Event;

    fn notify(&mut self, event: Self::Event) -> Result<(), sails_rs::errors::Error> {
        self.inner.notify_on(event)
    }
}
