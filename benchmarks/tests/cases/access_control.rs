use crate::common;
use access_control_test_client::{AccessControlTestClient, access_control::AccessControl};
use awesome_sails_benchmarks::{BenchStorage, ToBenchmarkMap, measure_gas, median};
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
        for (k, v) in self.grant_role {
            map.insert(format!("grant_role.{}", k), v);
        }
        for (k, v) in self.has_role {
            map.insert(format!("has_role.{}", k), v);
        }
        for (k, v) in self.revoke_role {
            map.insert(format!("revoke_role.{}", k), v);
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

    let mut grant_results = BTreeMap::new();
    let mut has_results = BTreeMap::new();
    let mut revoke_results = BTreeMap::new();

    for &count in &member_counts {
        let mut grant_samples = Vec::new();
        let mut has_samples = Vec::new();
        let mut revoke_samples = Vec::new();

        for _ in 0..5 {
            let env = common::create_env();
            let program = common::deploy_program(&env, wasm_path.clone()).await;
            let mut service = program.access_control();
            let system = env.system();

            for j in 0..count {
                let member: ActorId = (j as u64 + 1000).into();
                let _ = service.grant_role(role_id, member).send_one_way().unwrap();
                system.run_next_block();
            }

            let test_member: ActorId = (count as u64 + 50000).into();

            grant_samples.push(measure_gas(system, || {
                service
                    .grant_role(role_id, test_member)
                    .send_one_way()
                    .unwrap()
            }));

            has_samples.push(measure_gas(system, || {
                service
                    .has_role(role_id, test_member)
                    .send_one_way()
                    .unwrap()
            }));

            revoke_samples.push(measure_gas(system, || {
                service
                    .revoke_role(role_id, test_member)
                    .send_one_way()
                    .unwrap()
            }));
        }

        grant_results.insert(count, median(grant_samples));
        has_results.insert(count, median(has_samples));
        revoke_results.insert(count, median(revoke_samples));
    }

    BenchStorage::new()
        .update(
            "access_control",
            AccessControlBenchResults {
                grant_role: grant_results,
                has_role: has_results,
                revoke_role: revoke_results,
            },
        )
        .expect("Failed to save benchmark data");
}
