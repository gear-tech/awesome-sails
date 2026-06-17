// This file is part of Gear.

// Copyright (C) 2026 Gear Technologies Inc.
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

//! End-to-end demo of the `GasTrace` qualitative profiler.
//!
//! Fires a real message at the access-control test program, captures the
//! resulting `BlockRunResult`, and renders it as a gas-annotated tree.

use crate::common;
use access_control_test_client::{AccessControlTestClient, access_control::AccessControl};
use awesome_sails_benchmarks::GasTrace;
use sails_rs::{ActorId, prelude::*};

#[tokio::test]
#[ignore = "bench"]
async fn gas_trace_demo_access_control() {
    let wasm_name = "access_control_test_app.opt.wasm";
    let wasm_path = common::get_wasm_path(wasm_name);

    let env = common::create_env();
    let program = common::deploy_program(&env, wasm_path).await;
    let mut service = program.access_control();
    let system = env.system();

    // Send one grant_role call and capture the resulting block.
    let role_id = [1u8; 32];
    let member: ActorId = 1234u64.into();
    service
        .grant_role(role_id, member)
        .send_one_way()
        .expect("send failed");
    let block = system.run_next_block();

    // Build a trace tree. No registry — raw entries print as
    // `{interface_id}#{entry_id}`, which is still useful for validation.
    let tree = GasTrace::new(&block)
        .with_actor_name(common::ALICE.into(), "alice")
        .build();

    assert!(!tree.roots.is_empty(), "expected at least one root message");
    assert!(tree.total_gas > 0, "expected gas > 0");
    assert_eq!(tree.total_messages, block.log().len());

    // Human-readable output (visible with `cargo test -- --nocapture`).
    std::println!("{}", tree);
}
