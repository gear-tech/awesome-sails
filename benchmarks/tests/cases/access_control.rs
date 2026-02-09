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

use crate::common;
use access_control_test_client::{AccessControlTestClient, access_control::AccessControl};
use awesome_sails_benchmarks::{BenchStorage, MeasureGas, ToBenchmarkMap, median};
use sails_rs::{ActorId, prelude::*};
use std::collections::BTreeMap;

struct AccessControlBenchResults {
    grant_role: BTreeMap<u32, u64>,
    has_role: BTreeMap<u32, u64>,
    revoke_role: BTreeMap<u32, u64>,
}

impl ToBenchmarkMap for AccessControlBenchResults {
    fn to_benchmark_map(self) -> BTreeMap<String, u64> {
        let mut map = BTreeMap::new();
        for (count, gas) in self.grant_role {
            map.insert(format!("grant_role.{}", count), gas);
        }
        for (count, gas) in self.has_role {
            map.insert(format!("has_role.{}", count), gas);
        }
        for (count, gas) in self.revoke_role {
            map.insert(format!("revoke_role.{}", count), gas);
        }
        map
    }
}

#[tokio::test]
#[ignore = "bench"]
async fn bench_access_control() {
    let wasm_name = "access_control_test_app.opt.wasm";
    let wasm_path = common::get_wasm_path(wasm_name);

    let member_counts: [u32; 5] = [0, 100, 1000, 5000, 10000];
    let role_id = [1u8; 32];

    let mut grant_role_metrics = BTreeMap::new();
    let mut has_role_metrics = BTreeMap::new();
    let mut revoke_role_metrics = BTreeMap::new();

    for &count in &member_counts {
        let mut grant_samples = Vec::new();
        let mut has_samples = Vec::new();
        let mut revoke_samples = Vec::new();

        for _ in 0..5 {
            let env = common::create_env();
            let program = common::deploy_program(&env, wasm_path.clone()).await;
            let mut service = program.access_control();
            let system = env.system();

            // Setup: populate state
            for j in 0..count {
                let member: ActorId = (j as u64 + 1000).into();
                let _ = service.grant_role(role_id, member).send_one_way().unwrap();
                system.run_next_block();
            }

            let test_member: ActorId = (count as u64 + 50000).into();

            // 1. Measure Grant
            grant_samples.push(
                system
                    .measure_gas(|| {
                        service
                            .grant_role(role_id, test_member)
                            .send_one_way()
                            .unwrap()
                    })
                    .unwrap(),
            );

            // 2. Measure Has (Read)
            has_samples.push(
                system
                    .measure_gas(|| {
                        service
                            .has_role(role_id, test_member)
                            .send_one_way()
                            .unwrap()
                    })
                    .unwrap(),
            );

            // 3. Measure Revoke
            revoke_samples.push(
                system
                    .measure_gas(|| {
                        service
                            .revoke_role(role_id, test_member)
                            .send_one_way()
                            .unwrap()
                    })
                    .unwrap(),
            );
        }

        grant_role_metrics.insert(count, median(grant_samples));
        has_role_metrics.insert(count, median(has_samples));
        revoke_role_metrics.insert(count, median(revoke_samples));
    }

    let results = AccessControlBenchResults {
        grant_role: grant_role_metrics,
        has_role: has_role_metrics,
        revoke_role: revoke_role_metrics,
    };

    BenchStorage::from_default_path()
        .update("access_control", results)
        .expect("Failed to save benchmark data");
}
