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
//! This service provides admin functionality of exchanging native tokens to VFT's.

#![no_std]

use awesome_sails_access_control_service::RolesStorage;
use awesome_sails_utils::{error::Error, ok_if, storage::StorageMut};
use awesome_sails_vft_admin_service::{self as vft_admin};
use awesome_sails_vft_service::utils::{Allowances, Balances};
use sails_rs::{gstd, prelude::*};

/// Awesome VFT-Native-Exchange-Admin service itself.
pub struct Service<'a, ACS, A, B>
where
    ACS: StorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    vft_admin: vft_admin::ServiceExposure<vft_admin::Service<'a, ACS, A, B>>,
}

impl<'a, ACS, A, B> Service<'a, ACS, A, B>
where
    ACS: StorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    /// Constructor for [`Self`].
    pub fn new(vft_admin: vft_admin::ServiceExposure<vft_admin::Service<'a, ACS, A, B>>) -> Self {
        Self { vft_admin }
    }
}

#[service(events = Event)]
impl<'a, ACS, A, B> Service<'a, ACS, A, B>
where
    ACS: StorageMut<Item = RolesStorage>,
    A: StorageMut<Item = Allowances>,
    B: StorageMut<Item = Balances>,
{
    /// Reply handler for failed token transfers.
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

#[event]
#[derive(Clone, Debug, PartialEq, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    FailedMint { to: ActorId, value: U256 },
}
