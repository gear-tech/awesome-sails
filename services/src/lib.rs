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

//! Awesome sails set of common services.
//!
//! This crate is a collection of reexported services crates.

#![no_std]

#[cfg(feature = "vft")]
pub use awesome_sails_vft_service as vft;

#[cfg(feature = "vft-admin")]
pub use awesome_sails_vft_admin_service as vft_admin;

#[cfg(feature = "vft-extension")]
pub use awesome_sails_vft_extension_service as vft_extension;

#[cfg(feature = "vft-metadata")]
pub use awesome_sails_vft_metadata_service as vft_metadata;

#[cfg(feature = "vft-native-exchange")]
pub use awesome_sails_vft_native_exchange_service as vft_native_exchange;

#[cfg(feature = "vft-native-exchange-admin")]
pub use awesome_sails_vft_native_exchange_admin_service as vft_native_exchange_admin;

#[cfg(all(feature = "all", feature = "test"))]
pub mod test {
    use crate::{
        vft::utils::{Allowances, Balances},
        vft_admin::{
            Authorities,
            utils::{Pausable, Pause},
        },
        vft_metadata::Metadata,
        *,
    };
    use awesome_sails::{error::Error, storage::Storage};
    use awesome_sails_vft_service::utils::{Allowance, Balance};
    use core::{cell::RefCell, ops::DerefMut};
    use sails_rs::prelude::*;

    pub struct TestService<'a> {
        allowances: &'a Pausable<RefCell<Allowances>>,
        balances: &'a Pausable<RefCell<Balances>>,
    }

    #[service]
    impl TestService<'_> {
        #[export(unwrap_result)]
        pub fn set(
            &mut self,
            new_allowances: Vec<(ActorId, ActorId, U256, u32)>,
            new_balances: Vec<(ActorId, U256)>,
            minimum_balance: U256,
            expiry_period: u32,
        ) -> Result<(), Error> {
            let mut a = self.allowances.get_mut()?;

            a.set_expiry_period(expiry_period);

            let allowances = a.deref_mut();
            allowances.clear_shards();

            for (owner, spender, amount, bn) in new_allowances {
                unsafe {
                    allowances.try_insert_new(
                        (owner.try_into()?, spender.try_into()?),
                        (Allowance::try_from(amount)?.try_into()?, bn),
                    )?;
                }
            }

            let mut b = self.balances.get_mut()?;

            b.set_minimum_balance(Balance::try_from(minimum_balance)?);
            b.set_unused_value(U256::zero());

            let balances = b.deref_mut();
            balances.clear_shards();

            let mut total_supply = U256::zero();

            for (owner, amount) in new_balances {
                total_supply += amount;

                unsafe {
                    balances.try_insert_new(
                        owner.try_into()?,
                        Balance::try_from(amount)?.try_into()?,
                    )?;
                }
            }

            b.set_total_supply(total_supply);

            Ok(())
        }
    }

    pub struct TestProgram {
        authorities: RefCell<Authorities>,
        allowances: Pausable<RefCell<Allowances>>,
        balances: Pausable<RefCell<Balances>>,
        metadata: RefCell<Metadata>,
        pause: Pause,
    }

    #[program]
    impl TestProgram {
        // Program's constructor
        pub fn new() -> Self {
            let pause = Pause::default();

            Self {
                authorities: RefCell::new(Authorities::from_one(Syscall::message_source())),
                allowances: Pausable::new(&pause, RefCell::new(Allowances::default())),
                balances: Pausable::new(&pause, RefCell::new(Balances::default())),
                metadata: RefCell::new(Metadata::default()),
                pause,
            }
        }

        /// Program's reply handler.
        pub fn handle_reply(&mut self) {
            self.vft_native_exchange_admin().handle_reply();
        }

        pub fn test(&self) -> TestService<'_> {
            TestService {
                allowances: &self.allowances,
                balances: &self.balances,
            }
        }

        pub fn vft(&self) -> vft::Service<'_> {
            vft::Service::new(&self.allowances, &self.balances)
        }

        pub fn vft_admin(&self) -> vft_admin::Service<'_> {
            vft_admin::Service::new(
                &self.authorities,
                &self.allowances,
                &self.balances,
                &self.pause,
                self.vft().emitter(),
            )
        }

        pub fn vft_extension(&self) -> vft_extension::Service<'_> {
            vft_extension::Service::new(&self.allowances, &self.balances, self.vft().emitter())
        }

        pub fn vft_metadata(&self) -> vft_metadata::Service<'_> {
            vft_metadata::Service::new(&self.metadata)
        }

        pub fn vft_native_exchange(&self) -> vft_native_exchange::Service<'_> {
            vft_native_exchange::Service::new(&self.balances, self.vft().emitter())
        }

        pub fn vft_native_exchange_admin(&self) -> vft_native_exchange_admin::Service<'_> {
            vft_native_exchange_admin::Service::new(self.vft_admin())
        }
    }

    impl Default for TestProgram {
        fn default() -> Self {
            Self::new()
        }
    }
}
