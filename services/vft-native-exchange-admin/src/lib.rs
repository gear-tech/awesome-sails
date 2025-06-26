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

use awesome_sails::{
    error::Error,
    ok_if,
    pause::Pausable,
    storage::{InfallibleStorage, Storage},
};
use awesome_sails_vft_admin_service::{self as vft_admin, Authorities};
use awesome_sails_vft_service::utils::{Allowances, Balances};
use core::cell::RefCell;
use sails_rs::{gstd, prelude::*};

/// Awesome VFT-Native-Exchange-Admin service itself.
pub struct Service<
    'a,
    S: InfallibleStorage<Item = Authorities> = RefCell<Authorities>,
    A: Storage<Item = Allowances> = Pausable<RefCell<Allowances>>,
    B: Storage<Item = Balances> = Pausable<RefCell<Balances>>,
> {
    vft_admin: vft_admin::ServiceExposure<vft_admin::Service<'a, S, A, B>>,
}

impl<
    'a,
    S: InfallibleStorage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> Service<'a, S, A, B>
{
    /// Constructor for [`Self`].
    pub fn new(vft_admin: vft_admin::ServiceExposure<vft_admin::Service<'a, S, A, B>>) -> Self {
        Self { vft_admin }
    }
}

impl<
    'a,
    S: InfallibleStorage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> ServiceExposure<Service<'a, S, A, B>>
{
    /// Reply handler for failed token transfers.
    pub fn handle_reply(&mut self) {
        // TODO(sails): impl getters for reply details.
        let value = Syscall::message_value();

        if value == 0 {
            return;
        };

        let mint_res = unsafe {
            self.inner
                .vft_admin
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
}

#[service(events = Event)]
impl<
    S: InfallibleStorage<Item = Authorities>,
    A: Storage<Item = Allowances>,
    B: Storage<Item = Balances>,
> Service<'_, S, A, B>
{
    #[export(unwrap_result)]
    pub fn burn_from(&mut self, from: ActorId, value: U256) -> Result<(), Error> {
        ok_if!(value.is_zero());

        self.vft_admin.burn(from, value)?;

        // TODO(sails): impl sync Remoting.
        // TODO: #6
        gstd::msg::send_bytes_for_reply(from, [], value.as_u128(), 5_000_000_000)
            .map_err(|_| Error::new("failed to send value"))?;

        Ok(())
    }
}

#[derive(Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    FailedMint { to: ActorId, value: U256 },
}
