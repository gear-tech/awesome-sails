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
//! This service provides functionality of exchanging native tokens to VFT's.

#![no_std]

use awesome_sails::{error::Error, math::Zero, ok_if, storage::Storage};
use awesome_sails_vft_service::{
    self as vft,
    utils::{Allowances, Balance, Balances},
};
use sails_rs::{ActorId, U256, gstd::msg, prelude::*};

/// Awesome VFT-Native-Exchange service itself.
pub struct Service<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> {
    balances: B,
    vft: vft::ServiceExposure<vft::Service<A, B>>,
}

impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Service<A, B> {
    /// Constructor for [`Self`].
    pub fn new(balances: B, vft: vft::ServiceExposure<vft::Service<A, B>>) -> Self {
        Self { balances, vft }
    }
}

#[service]
impl<A: Storage<Item = Allowances>, B: Storage<Item = Balances>> Service<A, B> {
    #[export(unwrap_result)]
    pub fn burn(&mut self, value: U256) -> Result<CommandReply<()>, Error> {
        ok_if!(value.is_zero());

        let from = msg::source();

        self.balances
            .get_mut()?
            .burn(from.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.vft.emit_event(vft::Event::Transfer {
            from,
            to: ActorId::zero(),
            value,
        });

        Ok(CommandReply::new(()).with_value(value.as_u128()))
    }

    #[export(unwrap_result)]
    pub fn burn_all(&mut self) -> Result<CommandReply<()>, Error> {
        let from = msg::source();

        let value = self.balances.get_mut()?.burn_all(from.try_into()?);

        ok_if!(value.is_zero());

        self.vft.emit_event(vft::Event::Transfer {
            from,
            to: ActorId::zero(),
            value: value.into(),
        });

        Ok(CommandReply::new(()).with_value(value.into()))
    }

    #[export(unwrap_result)]
    pub fn mint(&mut self) -> Result<(), Error> {
        let value = U256::from(msg::value());

        ok_if!(value.is_zero());

        let to = msg::source();

        self.balances
            .get_mut()?
            .mint(to.try_into()?, Balance::try_from(value)?.try_into()?)?;

        self.vft.emit_event(vft::Event::Transfer {
            from: ActorId::zero(),
            to,
            value,
        });

        Ok(())
    }
}
