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
        let mut manifest_dir =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
        manifest_dir.pop();

        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };

        manifest_dir
            .join("target")
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
    if cfg!(debug_assertions) {
        core::panic!("Benchmarks MUST be run in --release mode.");
    }
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
