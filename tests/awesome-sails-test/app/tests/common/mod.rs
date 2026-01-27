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

use awesome_sails_test_client::{
    AwesomeSailsTestClient, // Import AwesomeSailsTestClient trait
    AwesomeSailsTestClientCtors,
    AwesomeSailsTestClientProgram,
    test::Test,                  // Restore Test service import
    vft_extension::VftExtension, // Restore VftExtension import
};
use sails_rs::{
    ActorId, U256,
    client::{Actor, GearEnv, GtestEnv, GtestError},
    gtest::System,
    prelude::*,
};

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

#[cfg(debug_assertions)]
pub(crate) const DEMO_WASM_PATH: &str =
    "../../../target/wasm32-gear/debug/awesome_sails_test_app.opt.wasm";
#[cfg(not(debug_assertions))]
pub(crate) const DEMO_WASM_PATH: &str =
    "../../../target/wasm32-gear/release/awesome_sails_test_app.opt.wasm";

/// Deploys a new program in the test environment and returns the program client, GtestEnv and program ID.
pub fn deploy_env() -> (GtestEnv, CodeId, GasUnit) {
    let system = System::new();

    system.init_logger_with_default_filter("gwasm=debug,gtest=info,sails_rs=debug");

    system.mint_to(ALICE, BALANCE);
    system.mint_to(BOB, BALANCE);
    system.mint_to(CHARLIE, BALANCE);
    system.mint_to(DAVE, BALANCE);

    let env = GtestEnv::new(system, ALICE);
    let program_code_id = env.system().submit_code_file(DEMO_WASM_PATH);
    let gas_limit = sails_rs::gtest::constants::MAX_USER_GAS_LIMIT;

    (env, program_code_id, gas_limit)
}

pub async fn deploy_with_data(
    allowances: Vec<(ActorId, ActorId, U256, u32)>,
    balances: Vec<(ActorId, U256)>,
    expiry_period: u32,
) -> (
    Actor<AwesomeSailsTestClientProgram, GtestEnv>,
    GtestEnv,
    ActorId,
) {
    let (env, code_id, _gas_limit) = deploy_env();

    let program = env
        .deploy::<AwesomeSailsTestClientProgram>(code_id, b"salt".to_vec())
        .new()
        .await
        .expect("failed to deploy program");

    let program_id = program.id();

    let mut vft_extension = program.vft_extension();

    while vft_extension
        .allocate_next_balances_shard()
        .await
        .expect("failed to allocate next balances shard")
    {}

    while vft_extension
        .allocate_next_allowances_shard()
        .await
        .expect("failed to allocate next balances shard")
    {}

    program
        .test()
        .set(allowances, balances, expiry_period)
        .await
        .expect("failed to set data");

    (program, env, program_id)
}

#[track_caller]
pub fn assert_str_panic(e: GtestError, exp: &str) {
    match e {
        GtestError::ReplyHasError(
            ErrorReplyReason::Execution(SimpleExecutionError::UserspacePanic),
            res,
        ) => {
            let actual = String::from_utf8_lossy(&res);
            let expected =
                format!("panicked with 'called `Result::unwrap()` on an `Err` value: {exp}'");
            assert_eq!(actual, expected);
        }
        _ => core::panic!("not an expected error reply type: {e:?}"),
    }
}
