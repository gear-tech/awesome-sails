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

use awesome_sails_utils::{
    ensure,
    error::{BadOrigin, EmitError, Error},
    math::{Max, NonZero, Zero},
    ok_if,
    pause::{PausableRef, Pause, UnpausedError},
    storage::{InfallibleStorageMut, StorageMut, StorageRefCell},
};
use awesome_sails_vft_service::{
    self as vft,
    utils::{Allowance, Allowances, Balance, Balances},
};
use sails_rs::prelude::*;

/// Awesome VFT-Admin service itself.
pub struct Service<
    'a,
    S: InfallibleStorageMut<Item = Authorities> = StorageRefCell<'a, Authorities>,
    A: StorageMut<Item = Allowances> = PausableRef<'a, Allowances>,
    B: StorageMut<Item = Balances> = PausableRef<'a, Balances>,
> {
    authorities: S,
    allowances: A,
    balances: B,
    pause: &'a Pause,
    vft: vft::ServiceExposure<vft::Service<'a, A, B>>,
}

impl<
    'a,
    S: InfallibleStorageMut<Item = Authorities>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> Service<'a, S, A, B>
{
    /// Constructor for [`Self`].
    pub fn new(
        authorities: S,
        allowances: A,
        balances: B,
        pause: &'a Pause,
        vft: vft::ServiceExposure<vft::Service<'a, A, B>>,
    ) -> Self {
        Self {
            authorities,
            allowances,
            balances,
            pause,
            vft,
        }
    }

    /// Mints VFTs to the specified address.
    ///
    /// # Safety
    /// Make sure that you call mint in eligible places (called by minter, etc).
    unsafe fn do_mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        ok_if!(value.is_zero());

        self.balances
            .get_mut()?
            .mint(to.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.vft
            .emit_event(vft::Event::Transfer {
                from: ActorId::zero(),
                to,
                value,
            })
            .map_err(|_| EmitError)?;

        Ok(())
    }
}

#[service(events = Event)]
impl<
    'a,
    S: InfallibleStorageMut<Item = Authorities>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> Service<'a, S, A, B>
{
    /// Mints VFTs to the specified address.
    ///
    /// # Safety
    /// Make sure that you call mint in eligible places (called by minter, etc).
    pub unsafe fn do_mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        unsafe { self.inner.do_mint(to, value) }
    }

    #[export(unwrap_result)]
    pub fn append_allowances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.allowances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn append_balances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

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
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

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
            self.vft
                .emit_event(vft::Event::Approval {
                    owner,
                    spender,
                    value,
                })
                .map_err(|_| EmitError)?;
        }

        Ok(changed)
    }

    #[export(unwrap_result)]
    pub fn burn(&mut self, from: ActorId, value: U256) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.burner(), BadOrigin);

        self.balances
            .get_mut()?
            .burn(from.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.emit_event(Event::BurnerTookPlace)
            .map_err(|_| EmitError)?;

        self.vft
            .emit_event(vft::Event::Transfer {
                from,
                to: ActorId::zero(),
                value,
            })
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn exit(&mut self, inheritor: ActorId) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);
        ensure!(self.is_paused(), UnpausedError);

        self.emit_event(Event::Exited(inheritor))
            .map_err(|_| EmitError)?;

        Syscall::exit(inheritor)
    }

    #[export(unwrap_result)]
    pub fn mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.minter(), BadOrigin);

        unsafe {
            self.do_mint(to, value)?;
        }

        self.emit_event(Event::MinterTookPlace)
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn pause(&mut self) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.pauser(), BadOrigin);

        if self.pause.pause() {
            self.emit_event(Event::Paused).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn resume(&mut self) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.pauser(), BadOrigin);

        if self.pause.resume() {
            self.emit_event(Event::Resumed).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_admin(&mut self, admin: ActorId) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.authorities.get_mut().admin = admin;

        self.emit_event(Event::AdminChanged(admin))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_burner(&mut self, burner: ActorId) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.authorities.get_mut().burner = burner;

        self.emit_event(Event::BurnerChanged(burner))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_expiry_period(&mut self, period: u32) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.allowances.get_mut()?.set_expiry_period(period);

        self.emit_event(Event::ExpiryPeriodChanged(period))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_minimum_balance(&mut self, value: U256) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.balances
            .get_mut()?
            .set_minimum_balance(value.try_into()?);

        self.emit_event(Event::MinimumBalanceChanged(value))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_minter(&mut self, minter: ActorId) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.authorities.get_mut().minter = minter;

        self.emit_event(Event::MinterChanged(minter))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_pauser(&mut self, pauser: ActorId) -> Result<(), Error> {
        ensure!(Syscall::message_source() == self.admin(), BadOrigin);

        self.authorities.get_mut().pauser = pauser;

        self.emit_event(Event::PauserChanged(pauser))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export]
    pub fn admin(&self) -> ActorId {
        self.authorities.get().admin()
    }

    #[export]
    pub fn burner(&self) -> ActorId {
        self.authorities.get().burner()
    }

    #[export]
    pub fn minter(&self) -> ActorId {
        self.authorities.get().minter()
    }

    #[export]
    pub fn pauser(&self) -> ActorId {
        self.authorities.get().pauser()
    }

    #[export]
    pub fn is_paused(&self) -> bool {
        self.pause.is_paused()
    }
}

#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
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
