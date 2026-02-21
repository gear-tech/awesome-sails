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

//! Awesome VFT-NativeExchangeAdmin service.
//!
//! This service provides administrative functionality for exchanging native tokens to VFT tokens.
//! It handles cases such as failed transfers and allows admins to burn tokens from users while
//! returning the native value.

#![no_std]

use awesome_sails_access_control::{RolesStorage, error::Error};
use awesome_sails_utils::{
    ok_if,
    storage::{InfallibleStorageMut, StorageMut},
};
use awesome_sails_vft::utils::{Allowances, Balances};
use awesome_sails_vft_admin::{self as vft_admin};
use sails_rs::{gstd, prelude::*};

/// The VFT Native Exchange Admin service struct.
pub struct VftNativeExchangeAdmin<'a, ACS, A, B>
where
    ACS: InfallibleStorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    vft_admin: vft_admin::VftAdminExposure<vft_admin::VftAdmin<'a, ACS, A, B>>,
}

impl<'a, ACS, A, B> VftNativeExchangeAdmin<'a, ACS, A, B>
where
    ACS: InfallibleStorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    /// Creates a new instance of the VFT Native Exchange Admin service.
    ///
    /// # Arguments
    ///
    /// * `vft_admin` - Exposure of the VFT Admin service.
    pub fn new(vft_admin: vft_admin::VftAdminExposure<vft_admin::VftAdmin<'a, ACS, A, B>>) -> Self {
        Self { vft_admin }
    }
}

#[service(events = Event)]
impl<'a, ACS, A, B> VftNativeExchangeAdmin<'a, ACS, A, B>
where
    ACS: InfallibleStorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    /// Handles reply messages from failed token transfers.
    ///
    /// If a native value transfer fails, this handler attempts to re-mint the VFT tokens
    /// to the original sender to ensure no value is lost.
    pub fn handle_reply(&mut self) {
        // TODO(sails): impl getters for reply details.
        let value = Syscall::message_value();

        if value == 0 {
            return;
        };

        let mint_res = unsafe {
            self.vft_admin
                .do_mint(Syscall::message_source(), value.into())
        };

        if mint_res.is_err() {
            self.emit_event(Event::FailedMint {
                to: Syscall::message_source(),
                value: value.into(),
            })
            .expect("failed to emit event");
        }
    }

    /// Burns `value` amount of VFT tokens from `from` account and sends the equivalent
    /// native value to `from`.
    ///
    /// This allows an admin (with burner role) to force an exchange/refund.
    ///
    /// # Arguments
    ///
    /// * `from` - The account to burn tokens from.
    /// * `value` - The amount of tokens to burn.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    #[export(unwrap_result)]
    pub fn burn_from(&mut self, from: ActorId, value: U256) -> Result<(), Error> {
        ok_if!(value.is_zero());

        self.vft_admin.burn(from, value)?;

        // TODO(sails): impl sync Remoting.
        let message_id = gstd::msg::send_bytes(from, [], value.as_u128())
            .map_err(|_| Error::new("failed to send value"))?;
        // TODO: #6
        gstd::exec::reply_deposit(message_id, 5_000_000_000)
            .map_err(|_| Error::new("failed to deposit gas for reply"))?;

        Ok(())
    }
}

/// Events emitted by the VFT Native Exchange Admin service.
#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    /// Emitted when re-minting tokens after a failed transfer fails.
    FailedMint { to: ActorId, value: U256 },
}
