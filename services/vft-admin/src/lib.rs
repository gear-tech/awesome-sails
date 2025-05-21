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

//! Awesome VFT-Admin service.
//!
//! This service provides admin functionality to VFT.

#![no_std]

use awesome_sails::{
    ensure,
    error::{BadInput, BadOrigin, Error},
    event::Emitter,
    math::{Max, NonZero, Zero},
    ok_if,
    pause::{PausableError, Pause},
    storage::{InfallibleStorage, Storage},
};
use awesome_sails_vft_service::{
    self as vft,
    utils::{Allowance, Allowances, Balance, Balances},
};
use core::convert::Infallible;
use sails_rs::{
    ActorId, U256,
    gstd::{exec, msg},
    prelude::*,
};

/// Re-exporting utils for easier access.
pub mod utils {
    pub use awesome_sails::pause::{Pausable, Pause};
}

/// Awesome VFT-Admin service itself.
pub struct Service<
    'a,
    S: Storage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> {
    authorities: S,
    allowances: A,
    balances: B,
    pause: &'a Pause,
    vft: vft::ServiceExposure<vft::Service<A, B>>,
}

impl<'a, S: Storage<Item = Authorities>, A: Storage<Item = Allowances>, B: Storage<Item = Balances>>
    Service<'a, S, A, B>
{
    /// Constructor for [`Self`].
    pub fn new(
        authorities: S,
        allowances: A,
        balances: B,
        pause: &'a Pause,
        vft: vft::ServiceExposure<vft::Service<A, B>>,
    ) -> Self {
        Self {
            authorities,
            allowances,
            balances,
            pause,
            vft,
        }
    }
}

#[service(events = Event)]
impl<
    S: InfallibleStorage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> Service<'_, S, A, B>
{
    #[export(unwrap_result)]
    pub fn append_allowances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.allowances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn append_balances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.balances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn approve_from(
        &mut self,
        owner: ActorId,
        spender: ActorId,
        value: U256,
    ) -> Result<bool, Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

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
            self.vft.emit_event(vft::Event::Approval {
                owner,
                spender,
                value,
            });
        }

        Ok(changed)
    }

    #[export(unwrap_result)]
    pub fn burn(&mut self, from: ActorId, value: U256) -> Result<(), Error> {
        ensure!(msg::source() == self.burner(), BadOrigin);

        self.balances
            .get_mut()?
            .burn(from.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.emit(Event::BurnerTookPlace)?;

        self.vft.emit_event(vft::Event::Transfer {
            from,
            to: ActorId::zero(),
            value,
        });

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn exit(&mut self, inheritor: ActorId) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);
        // TODO: error
        ensure!(self.is_paused(), PausableError::<Infallible>::Paused);
        // TODO: check ensure
        ensure!(inheritor.is_zero(), BadInput);

        self.emit(Event::Exited(inheritor))?;

        exec::exit(inheritor)
    }

    #[export(unwrap_result)]
    pub fn mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        ensure!(msg::source() == self.minter(), BadOrigin);

        ok_if!(value.is_zero());

        self.balances
            .get_mut()?
            .mint(to.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.emit(Event::MinterTookPlace)?;

        self.vft.emit_event(vft::Event::Transfer {
            from: ActorId::zero(),
            to,
            value,
        });

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn pause(&mut self) -> Result<(), Error> {
        ensure!(msg::source() == self.pauser(), BadOrigin);

        if self.pause.pause() {
            self.emit(Event::Paused)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn resume(&mut self) -> Result<(), Error> {
        ensure!(msg::source() == self.pauser(), BadOrigin);

        if self.pause.resume() {
            self.emit(Event::Resumed)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_admin(&mut self, admin: ActorId) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.authorities.get_mut().admin = admin;

        self.emit(Event::AdminChanged(admin))?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_burner(&mut self, burner: ActorId) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.authorities.get_mut().burner = burner;

        self.emit(Event::BurnerChanged(burner))?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_expiry_period(&mut self, period: u32) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.allowances.get_mut()?.set_expiry_period(period);

        self.emit(Event::ExpiryPeriodChanged(period))?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_minimum_balance(&mut self, value: U256) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.balances
            .get_mut()?
            .set_minimum_balance(value.try_into()?);

        self.emit(Event::MinimumBalanceChanged(value))?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_minter(&mut self, minter: ActorId) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.authorities.get_mut().minter = minter;

        self.emit(Event::MinterChanged(minter))?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_pauser(&mut self, pauser: ActorId) -> Result<(), Error> {
        ensure!(msg::source() == self.admin(), BadOrigin);

        self.authorities.get_mut().pauser = pauser;

        self.emit(Event::PauserChanged(pauser))?;

        Ok(())
    }

    pub fn admin(&self) -> ActorId {
        self.authorities.get().admin()
    }

    pub fn burner(&self) -> ActorId {
        self.authorities.get().burner()
    }

    pub fn minter(&self) -> ActorId {
        self.authorities.get().minter()
    }

    pub fn pauser(&self) -> ActorId {
        self.authorities.get().pauser()
    }

    pub fn is_paused(&self) -> bool {
        self.pause.is_paused()
    }
}

#[derive(Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    AdminChanged(ActorId),
    BurnerChanged(ActorId),
    MinterChanged(ActorId),
    PauserChanged(ActorId),

    BurnerTookPlace,
    MinterTookPlace,

    ExpiryPeriodChanged(u32),
    MinimumBalanceChanged(U256),

    Exited(ActorId),

    Paused,
    Resumed,
}

/// Address book of the authorities.
#[derive(Clone, Debug, Default)]
pub struct Authorities {
    admin: ActorId,
    burner: ActorId,
    minter: ActorId,
    pauser: ActorId,
}

impl Authorities {
    /// Creates a new [`Self`] instance.
    pub fn new(admin: ActorId, burner: ActorId, minter: ActorId, pauser: ActorId) -> Self {
        Self {
            admin,
            burner,
            minter,
            pauser,
        }
    }

    /// Creates a new [`Self`] instance with all authorities set to the same address.
    pub fn from_one(admin: ActorId) -> Self {
        Self {
            admin,
            burner: admin,
            minter: admin,
            pauser: admin,
        }
    }

    /// Returns the address of the admin.
    ///
    /// This address is eligible to change the authorities and other parameters.
    pub fn admin(&self) -> ActorId {
        self.admin
    }

    /// Returns the address of the burner.
    ///
    /// This address is eligible to burn.
    pub fn burner(&self) -> ActorId {
        self.burner
    }

    /// Returns the address of the minter.
    ///
    /// This address is eligible to mint.
    pub fn minter(&self) -> ActorId {
        self.minter
    }

    /// Returns the address of the pauser.
    ///
    /// This address is eligible to pause and resume.
    pub fn pauser(&self) -> ActorId {
        self.pauser
    }
}

/* TODO: DELETE CODE BELOW ONCE APPROPRIATE SAILS CHANGES APPLIED */

impl<
    S: InfallibleStorage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> Emitter for Service<'_, S, A, B>
{
    type Event = Event;

    fn notify(&mut self, event: Self::Event) -> Result<(), sails_rs::errors::Error> {
        self.emit_event(event)
    }
}
