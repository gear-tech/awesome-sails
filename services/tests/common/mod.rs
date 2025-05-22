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

use sails_rs::{
    ActorId, U256,
    calls::*,
    errors::{Error, RtlError},
    gtest::{System, calls::*},
};
use test_bin::client::{self, traits::*};

const fn actor_id(id: u8) -> ActorId {
    let mut bytes = [0; 32];
    bytes[12] = id;
    ActorId::new(bytes)
}

/// Alice account id. Alice is admin of the program.
pub const ALICE: ActorId = actor_id(42);

/// Bob account id.
pub const BOB: ActorId = actor_id(43);

/// Charlie account id.
pub const CHARLIE: ActorId = actor_id(44);

/// Dave account id.
pub const DAVE: ActorId = actor_id(45);

/// Initial balance for the actor. 100_000 * 10**12.
pub const BALANCE: u128 = 100_000_000_000_000_000;

/// Deploys a new program in the test environment and returns the remoting instance and program ID.
pub async fn deploy() -> (GTestRemoting, ActorId) {
    // Creating a new system instance.
    let system = System::new();

    // Initializing the logger with default filter settings.
    system.init_logger_with_default_filter("gwasm=debug,gtest=info,sails_rs=debug");

    // Minting a lot of tokens of tokens to the actor ID.
    system.mint_to(ALICE, BALANCE);
    system.mint_to(BOB, BALANCE);
    system.mint_to(CHARLIE, BALANCE);
    system.mint_to(DAVE, BALANCE);

    // Creating a new remoting instance for the system.
    let remoting = GTestRemoting::new(system, ALICE);

    // Submit program code into the system
    let program_code_id = remoting.system().submit_code(test_bin::WASM_BINARY);

    // Creating a new program factory instance.
    let program_factory = client::TestBinFactory::new(remoting.clone());

    // Deploying the program and getting its ID.
    let program_id = program_factory
        .new()
        .send_recv(program_code_id, b"salt")
        .await
        .expect("failed to deploy program");

    let mut vft_extension = client::VftExtension::new(remoting.clone());

    // Allocating underlying shards.
    while vft_extension
        .allocate_next_balances_shard()
        .send_recv(program_id)
        .await
        .expect("failed to allocate next balances shard")
    {}

    while vft_extension
        .allocate_next_allowances_shard()
        .send_recv(program_id)
        .await
        .expect("failed to allocate next balances shard")
    {}

    // Returning the remoting instance and the program ID.
    (remoting, program_id)
}

pub async fn deploy_with_data(
    allowances: Vec<(ActorId, ActorId, U256, u32)>,
    balances: Vec<(ActorId, U256)>,
    minimum_balance: U256,
    expiry_period: u32,
) -> (GTestRemoting, ActorId) {
    let (remoting, program_id) = deploy().await;

    let mut test_service = client::Test::new(remoting.clone());

    test_service
        .set(allowances, balances, minimum_balance, expiry_period)
        .send_recv(program_id)
        .await
        .expect("failed to set data");

    (remoting, program_id)
}

#[track_caller]
pub fn assert_str_panic(e: Error, exp: impl core::error::Error) {
    match e {
        Error::Rtl(RtlError::ReplyHasError(_, res)) => {
            let exp = format!("panicked with 'called `Result::unwrap()` on an `Err` value: {exp}'");
            assert_eq!(String::from_utf8_lossy(res.as_slice()), exp);
        }
        _ => panic!("not an error reply"),
    }
}
