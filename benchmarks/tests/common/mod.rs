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

use access_control_test_client::{AccessControlTestClientCtors, AccessControlTestClientProgram};
use sails_rs::{
    client::{Actor, GearEnv, GtestEnv},
    gtest::System,
    prelude::*,
};
use std::path::PathBuf;

pub const ALICE: u64 = 42;

pub fn get_wasm_path(file_name: &str) -> PathBuf {
    let path = PathBuf::from(file_name);
    let final_path = if path.is_absolute() || path.exists() {
        path
    } else {
        let mut root =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
        // Go up from benchmarks to project root
        root.pop();

        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };

        root.join("target")
            .join("wasm32-gear")
            .join(profile)
            .join(file_name)
    };

    if !final_path.exists() {
        core::panic!(
            "Wasm file not found at {:?}. Run `cargo build --release -p <package>` first.",
            final_path
        );
    }

    final_path
}

pub fn create_env() -> GtestEnv {
    let system = System::new();
    system.mint_to(ALICE, 100_000_000_000_000_000);
    GtestEnv::new(system, ALICE.into())
}

pub async fn deploy_program(
    env: &GtestEnv,
    wasm_path: PathBuf,
) -> Actor<AccessControlTestClientProgram, GtestEnv> {
    let code_id = env.system().submit_local_code_file(wasm_path);

    static SALT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let salt = SALT_COUNTER
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        .to_le_bytes()
        .to_vec();

    env.deploy::<AccessControlTestClientProgram>(code_id, salt)
        .new()
        .with_value(10_000_000_000_000)
        .await
        .expect("Failed to deploy program")
}
