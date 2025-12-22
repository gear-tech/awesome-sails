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
//! This service provides admin functionality to VFT using Role-Based Access Control.

#![no_std]

use awesome_sails_access_control_service::{
    self as access_control, DEFAULT_ADMIN_ROLE, RoleId, RolesStorage,
};
use awesome_sails_utils::{
    ensure,
    error::{BadOrigin, EmitError, Error},
    math::{Max, NonZero, Zero},
    ok_if,
    pause::{PausableRef, Pause, UnpausedError},
    storage::{StorageMut, StorageRefCell},
};
use awesome_sails_vft_service::{
    self as vft,
    utils::{Allowance, Allowances, Balance, Balances},
};
use sails_rs::prelude::*;

pub const MINTER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"MINTER_ROLE")
    .finalize();
pub const BURNER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"BURNER_ROLE")
    .finalize();
pub const PAUSER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"PAUSER_ROLE")
    .finalize();

/// Awesome VFT-Admin service itself.
pub struct Service<
    'a,
    ACS: StorageMut<Item = RolesStorage> = StorageRefCell<'a, RolesStorage>,
    A: StorageMut<Item = Allowances> = PausableRef<'a, Allowances>,
    B: StorageMut<Item = Balances> = PausableRef<'a, Balances>,
> {
    access_control: access_control::ServiceExposure<access_control::Service<'a, ACS>>,
    allowances: A,
    balances: B,
    pause: &'a Pause,
    vft: vft::ServiceExposure<vft::Service<'a, A, B>>,
}

impl<
    'a,
    ACS: StorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> Service<'a, ACS, A, B>
{
    /// Constructor for [`Self`].
    pub fn new(
        access_control: access_control::ServiceExposure<access_control::Service<'a, ACS>>,
        allowances: A,
        balances: B,
        pause: &'a Pause,
        vft: vft::ServiceExposure<vft::Service<'a, A, B>>,
    ) -> Self {
        Self {
            access_control,
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
    ACS: StorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> Service<'a, ACS, A, B>
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
        ensure!(
            self.access_control
                .has_role(DEFAULT_ADMIN_ROLE, Syscall::message_source()),
            BadOrigin
        );

        self.allowances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn append_balances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        ensure!(
            self.access_control
                .has_role(DEFAULT_ADMIN_ROLE, Syscall::message_source()),
            BadOrigin
        );

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
        ensure!(
            self.access_control
                .has_role(DEFAULT_ADMIN_ROLE, Syscall::message_source()),
            BadOrigin
        );

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
        ensure!(
            self.access_control
                .has_role(BURNER_ROLE, Syscall::message_source()),
            BadOrigin
        );

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
        ensure!(
            self.access_control
                .has_role(DEFAULT_ADMIN_ROLE, Syscall::message_source()),
            BadOrigin
        );
        ensure!(self.is_paused(), UnpausedError);

        self.emit_event(Event::Exited(inheritor))
            .map_err(|_| EmitError)?;

        Syscall::exit(inheritor)
    }

    #[export(unwrap_result)]
    pub fn mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        ensure!(
            self.access_control
                .has_role(MINTER_ROLE, Syscall::message_source()),
            BadOrigin
        );

        unsafe {
            self.do_mint(to, value)?;
        }

        self.emit_event(Event::MinterTookPlace)
            .map_err(|_| EmitError)?;

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn pause(&mut self) -> Result<(), Error> {
        ensure!(
            self.access_control
                .has_role(PAUSER_ROLE, Syscall::message_source()),
            BadOrigin
        );

        if self.pause.pause() {
            self.emit_event(Event::Paused).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn resume(&mut self) -> Result<(), Error> {
        ensure!(
            self.access_control
                .has_role(PAUSER_ROLE, Syscall::message_source()),
            BadOrigin
        );

        if self.pause.resume() {
            self.emit_event(Event::Resumed).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    #[export(unwrap_result)]
    pub fn set_expiry_period(&mut self, period: u32) -> Result<(), Error> {
        ensure!(
            self.access_control
                .has_role(DEFAULT_ADMIN_ROLE, Syscall::message_source()),
            BadOrigin
        );

        self.allowances.get_mut()?.set_expiry_period(period);

        self.emit_event(Event::ExpiryPeriodChanged(period))
            .map_err(|_| EmitError)?;

        Ok(())
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
    BurnerTookPlace,
    MinterTookPlace,
    ExpiryPeriodChanged(u32),
    Exited(ActorId),
    Paused,
    Resumed,
}
