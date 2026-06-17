use msg_tracker_test_client::{MsgTrackerTestClientCtors, MsgTrackerTestClientProgram};
use sails_rs::{
    ActorId,
    client::{Actor, GearEnv, GtestEnv},
    gtest::System,
    prelude::*,
};

pub const ALICE: ActorId = ActorId::new([42; 32]);
pub const BOB: ActorId = ActorId::new([43; 32]);
pub const BALANCE: u128 = 100_000_000_000_000;

#[cfg(debug_assertions)]
pub(crate) const WASM_PATH: &str =
    "../../../target/wasm32-gear/debug/msg_tracker_test_app.opt.wasm";
#[cfg(not(debug_assertions))]
pub(crate) const WASM_PATH: &str =
    "../../../target/wasm32-gear/release/msg_tracker_test_app.opt.wasm";

pub async fn deploy_program() -> (Actor<MsgTrackerTestClientProgram, GtestEnv>, GtestEnv) {
    let system = System::new();
    system.init_logger_with_default_filter("gwasm=debug,gtest=info,sails_rs=debug");

    system.mint_to(ALICE, BALANCE);
    system.mint_to(BOB, BALANCE);

    let env = GtestEnv::new(system, ALICE);
    let code_id = env.system().submit_code_file(WASM_PATH);

    let program = env
        .deploy::<MsgTrackerTestClientProgram>(code_id, b"salt".to_vec())
        .new()
        .await
        .unwrap();

    (program, env)
}
