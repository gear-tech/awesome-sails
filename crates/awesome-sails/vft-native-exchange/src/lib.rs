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

//! Awesome VFT-NativeExchange service.
//!
//! This service allows exchanging native tokens (Value) for VFT tokens and vice versa.
//! Sending native value to `mint` creates VFT tokens. Calling `burn` destroys VFT tokens
//! and sends back native value.

#![no_std]

use awesome_sails_utils::{
    error::{EmitError, Error},
    math::Zero,
    ok_if,
    storage::StorageMut,
};
use awesome_sails_vft::{
    self as vft,
    utils::{Allowances, Balance, Balances},
};
use sails_rs::prelude::*;

/// The VFT Native Exchange service struct.
pub struct VftNativeExchange<'a, A, B>
where
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    balances: B,
    vft: vft::VftExposure<vft::Vft<'a, A, B>>,
}

impl<'a, A, B> VftNativeExchange<'a, A, B>
where
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    /// Creates a new instance of the VFT Native Exchange service.
    ///
    /// # Arguments
    ///
    /// * `balances` - Storage backend for balances.
    /// * `vft` - Exposure of the base VFT service.
    pub fn new(balances: B, vft: vft::VftExposure<vft::Vft<'a, A, B>>) -> Self {
        Self { balances, vft }
    }
}

#[service]
impl<'a, A: StorageMut<Item = Allowances>, B: StorageMut<Item = Balances>>
    VftNativeExchange<'a, A, B>
{
    /// Burns `value` amount of VFT tokens and returns the equivalent amount of native value to the caller.
    ///
    /// # Arguments
    ///
    /// * `value` - The amount of VFT tokens to burn.
    ///
    /// # Returns
    ///
    /// A `CommandReply` containing the unit value `()` and transferring the native value.
    #[export(unwrap_result)]
    pub fn burn(&mut self, value: U256) -> Result<CommandReply<()>, Error> {
        ok_if!(value.is_zero());

        let from = Syscall::message_source();

        self.balances
            .get_mut()?
            .burn(from.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.vft
            .emit_event(vft::Event::Transfer {
                from,
                to: ActorId::zero(),
                value,
            })
            .map_err(|_| EmitError)?;

        Ok(CommandReply::new(()).with_value(value.as_u128()))
    }

    /// Burns all VFT tokens owned by the caller and returns the equivalent amount of native value.
    ///
    /// # Returns
    ///
    /// A `CommandReply` containing the unit value `()` and transferring the native value.
    #[export(unwrap_result)]
    pub fn burn_all(&mut self) -> Result<CommandReply<()>, Error> {
        let from = Syscall::message_source();

        let value = self.balances.get_mut()?.burn_all(from.try_into()?);

        ok_if!(value.is_zero());

        self.vft
            .emit_event(vft::Event::Transfer {
                from,
                to: ActorId::zero(),
                value: value.into(),
            })
            .map_err(|_| EmitError)?;

        Ok(CommandReply::new(()).with_value(value.into()))
    }

    /// Mints VFT tokens to the caller equal to the amount of native value attached to the message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    #[export(unwrap_result)]
    pub fn mint(&mut self) -> Result<(), Error> {
        let value = U256::from(Syscall::message_value());

        ok_if!(value.is_zero());

        let to = Syscall::message_source();

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
