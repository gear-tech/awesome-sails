use crate::common;
use access_control_test_client::{AccessControlTestClient, access_control::AccessControl};
use awesome_sails_benchmarks::{BenchStorage, median};
use sails_rs::{ActorId, prelude::*};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct AccessControlBenchResults {
    grant_role: BTreeMap<u32, u64>,
}

#[tokio::test]
async fn bench_access_control_grant_role() {
    let wasm_name = "access_control_test_app.opt.wasm";
    let wasm_path = common::get_wasm_path(wasm_name);

    let member_counts: [u32; 5] = [0, 10, 100, 500, 1000];
    let role_id = [1u8; 32];

    let mut results_map = BTreeMap::new();

    for &count in &member_counts {
        let mut gas_samples = Vec::new();

        for _ in 0..5 {
            let env = common::create_env();
            let program = common::deploy_program(&env, wasm_path.clone()).await;

            let mut service = program.access_control();
            let system = env.system();

            for i in 0..count {
                let member: ActorId = ((1000 + i) as u64).into();
                service.grant_role(role_id, member).send_one_way().unwrap();
                system.run_next_block();
            }

            let new_member: ActorId = ((2000 + count) as u64).into();
            let mid = service
                .grant_role(role_id, new_member)
                .send_one_way()
                .unwrap();

            let res = system.run_next_block();
            let gas = *res.gas_burned.get(&mid).expect("Gas not recorded");
            gas_samples.push(gas);
        }

        let med = median(gas_samples);
        results_map.insert(count, med);
        println!("Members: {}, Median Gas: {}", count, med);
    }

    BenchStorage::new()
        .update(
            "access_control",
            AccessControlBenchResults {
                grant_role: results_map,
            },
        )
        .expect("Failed to save benchmark data");
}
