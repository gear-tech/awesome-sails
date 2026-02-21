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
//! This service provides administrative functionalities for the VFT, such as minting, burning,
//! pausing, and managing allowances/balances shards, secured by Role-Based Access Control (RBAC).

#![no_std]

use awesome_sails_access_control::{
    self as access_control, DEFAULT_ADMIN_ROLE, RoleId, RolesStorage, ensure,
    error::{EmitError, Error},
};
use awesome_sails_utils::{
    math::{Max, NonZero, Zero},
    ok_if,
    pause::{PausableRef, Pause, UnpausedError},
    storage::{InfallibleStorageMut, StorageMut, StorageRefCell},
};
use awesome_sails_vft::{
    self as vft,
    utils::{Allowance, Allowances, Balance, Balances},
};
use sails_rs::prelude::*;

/// Role identifier for accounts allowed to mint tokens.
pub const MINTER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"MINTER_ROLE")
    .finalize();

/// Role identifier for accounts allowed to burn tokens.
pub const BURNER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"BURNER_ROLE")
    .finalize();

/// Role identifier for accounts allowed to pause/resume the contract.
pub const PAUSER_ROLE: RoleId = keccak_const::Keccak256::new()
    .update(b"PAUSER_ROLE")
    .finalize();

/// The VFT Admin service struct.
///
/// Combines access control, VFT storage (allowances and balances), and pause state
/// to provide administrative actions.
pub struct VftAdmin<
    'a,
    ACS: InfallibleStorageMut<Item = RolesStorage> = StorageRefCell<'a, RolesStorage>,
    A: StorageMut<Item = Allowances> = PausableRef<'a, Allowances>,
    B: StorageMut<Item = Balances> = PausableRef<'a, Balances>,
> {
    access_control: access_control::AccessControlExposure<access_control::AccessControl<'a, ACS>>,
    allowances: A,
    balances: B,
    pause: &'a Pause,
    vft: vft::VftExposure<vft::Vft<'a, A, B>>,
}

impl<
    'a,
    ACS: InfallibleStorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> VftAdmin<'a, ACS, A, B>
{
    /// Creates a new instance of the VFT Admin service.
    ///
    /// # Arguments
    ///
    /// * `access_control` - Exposure of the access control service.
    /// * `allowances` - Storage backend for allowances.
    /// * `balances` - Storage backend for balances.
    /// * `pause` - Reference to the pause switch.
    /// * `vft` - Exposure of the VFT service.
    pub fn new(
        access_control: access_control::AccessControlExposure<
            access_control::AccessControl<'a, ACS>,
        >,
        allowances: A,
        balances: B,
        pause: &'a Pause,
        vft: vft::VftExposure<vft::Vft<'a, A, B>>,
    ) -> Self {
        Self {
            access_control,
            allowances,
            balances,
            pause,
            vft,
        }
    }

    /// Internal function to mint VFTs to the specified address.
    ///
    /// # Safety
    /// This function bypasses some checks and should only be called by authorized methods (e.g. `mint`).
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
    ACS: InfallibleStorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
> VftAdmin<'a, ACS, A, B>
{
    /// Mints VFTs to the specified address (exposed as unsafe to allow internal reuse).
    ///
    /// # Safety
    /// This method is equivalent to `do_mint` but exposed on the service trait.
    pub unsafe fn do_mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        unsafe { self.inner.do_mint(to, value) }
    }

    /// Appends a new shard to the allowances storage map.
    ///
    /// # Requirements
    /// * Caller must have `DEFAULT_ADMIN_ROLE`.
    ///
    /// # Arguments
    /// * `capacity` - The capacity of the new shard.
    #[export(unwrap_result)]
    pub fn append_allowances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        self.access_control
            .require_role(DEFAULT_ADMIN_ROLE, Syscall::message_source())?;

        self.allowances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    /// Appends a new shard to the balances storage map.
    ///
    /// # Requirements
    /// * Caller must have `DEFAULT_ADMIN_ROLE`.
    ///
    /// # Arguments
    /// * `capacity` - The capacity of the new shard.
    #[export(unwrap_result)]
    pub fn append_balances_shard(&mut self, capacity: u32) -> Result<(), Error> {
        self.access_control
            .require_role(DEFAULT_ADMIN_ROLE, Syscall::message_source())?;

        self.balances
            .get_mut()?
            .try_append_shard(capacity as usize)?;

        Ok(())
    }

    /// Approves `spender` to spend `value` from `owner`'s account.
    ///
    /// This is an admin function allowing the admin to set approvals arbitrarily.
    ///
    /// # Requirements
    /// * Caller must have `DEFAULT_ADMIN_ROLE`.
    ///
    /// # Arguments
    /// * `owner` - The account owning the tokens.
    /// * `spender` - The account to be approved.
    /// * `value` - The amount to approve.
    #[export(unwrap_result)]
    pub fn approve_from(
        &mut self,
        owner: ActorId,
        spender: ActorId,
        value: U256,
    ) -> Result<bool, Error> {
        self.access_control
            .require_role(DEFAULT_ADMIN_ROLE, Syscall::message_source())?;

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

    /// Burns `value` tokens from `from` account.
    ///
    /// # Requirements
    /// * Caller must have `BURNER_ROLE`.
    ///
    /// # Arguments
    /// * `from` - The account to burn tokens from.
    /// * `value` - The amount to burn.
    #[export(unwrap_result)]
    pub fn burn(&mut self, from: ActorId, value: U256) -> Result<(), Error> {
        self.access_control
            .require_role(BURNER_ROLE, Syscall::message_source())?;

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

    /// Terminates the program and sends value to `inheritor`.
    ///
    /// # Requirements
    /// * Caller must have `DEFAULT_ADMIN_ROLE`.
    /// * Program must be paused.
    #[export(unwrap_result)]
    pub fn exit(&mut self, inheritor: ActorId) -> Result<(), Error> {
        self.access_control
            .require_role(DEFAULT_ADMIN_ROLE, Syscall::message_source())?;
        ensure!(self.is_paused(), UnpausedError);

        self.emit_event(Event::Exited(inheritor))
            .map_err(|_| EmitError)?;

        Syscall::exit(inheritor)
    }

    /// Mints `value` tokens to `to` account.
    ///
    /// # Requirements
    /// * Caller must have `MINTER_ROLE`.
    ///
    /// # Arguments
    /// * `to` - The recipient of the minted tokens.
    /// * `value` - The amount to mint.
    #[export(unwrap_result)]
    pub fn mint(&mut self, to: ActorId, value: U256) -> Result<(), Error> {
        self.access_control
            .require_role(MINTER_ROLE, Syscall::message_source())?;

        unsafe {
            self.do_mint(to, value)?;
        }

        self.emit_event(Event::MinterTookPlace)
            .map_err(|_| EmitError)?;

        Ok(())
    }

    /// Pauses the contract.
    ///
    /// # Requirements
    /// * Caller must have `PAUSER_ROLE`.
    #[export(unwrap_result)]
    pub fn pause(&mut self) -> Result<(), Error> {
        self.access_control
            .require_role(PAUSER_ROLE, Syscall::message_source())?;

        if self.pause.pause() {
            self.emit_event(Event::Paused).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Resumes the contract.
    ///
    /// # Requirements
    /// * Caller must have `PAUSER_ROLE`.
    #[export(unwrap_result)]
    pub fn resume(&mut self) -> Result<(), Error> {
        self.access_control
            .require_role(PAUSER_ROLE, Syscall::message_source())?;

        if self.pause.resume() {
            self.emit_event(Event::Resumed).map_err(|_| EmitError)?;
        }

        Ok(())
    }

    /// Sets the expiry period for allowances.
    ///
    /// # Requirements
    /// * Caller must have `DEFAULT_ADMIN_ROLE`.
    ///
    /// # Arguments
    /// * `period` - The new expiry period in blocks.
    #[export(unwrap_result)]
    pub fn set_expiry_period(&mut self, period: u32) -> Result<(), Error> {
        self.access_control
            .require_role(DEFAULT_ADMIN_ROLE, Syscall::message_source())?;

        self.allowances.get_mut()?.set_expiry_period(period);

        self.emit_event(Event::ExpiryPeriodChanged(period))
            .map_err(|_| EmitError)?;

        Ok(())
    }

    /// Returns `true` if the contract is paused.
    #[export]
    pub fn is_paused(&self) -> bool {
        self.pause.is_paused()
    }
}

/// Events emitted by the VFT Admin service.
#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    /// Emitted when a burn operation occurs.
    BurnerTookPlace,
    /// Emitted when a mint operation occurs.
    MinterTookPlace,
    /// Emitted when the allowance expiry period is changed.
    ExpiryPeriodChanged(u32),
    /// Emitted when the program exits.
    Exited(ActorId),
    /// Emitted when the contract is paused.
    Paused,
    /// Emitted when the contract is resumed.
    Resumed,
}
