mod common;

use common::{assert_str_panic, deploy_program};
use msg_tracker_test_client::{
    MsgTrackerTestClient, OpStatus, dynamic_counter::DynamicCounter as _,
    fixed_counter::FixedCounter as _,
};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_fixed_counter_overflow_panic() {
    let (program, _env) = deploy_program().await;
    let mut fixed_counter = program.fixed_counter();

    for _ in 0..5 {
        fixed_counter.request_increment().await.unwrap();
    }

    let res = fixed_counter.request_increment().await;

    assert!(res.is_err());
    if let Err(e) = res {
        assert_str_panic(e, "CapacityReached");
    }
}

#[tokio::test]
async fn test_dynamic_counter_flow() {
    let (program, _env) = deploy_program().await;
    let mut dynamic_counter = program.dynamic_counter();

    let msg_id = dynamic_counter.request_increment().await.unwrap();

    let status = dynamic_counter.get_status(msg_id).await.unwrap();
    assert_eq!(status, Some(OpStatus::Pending));

    dynamic_counter.confirm_increment(msg_id).await.unwrap();

    assert_eq!(dynamic_counter.get_val().await.unwrap(), 1);
    assert_eq!(
        dynamic_counter.get_status(msg_id).await.unwrap(),
        Some(OpStatus::Completed)
    );
}

#[tokio::test]
async fn test_fixed_counter_removal_and_reuse() {
    let (program, _env) = deploy_program().await;
    let mut fixed_counter = program.fixed_counter();

    let mut last_id = MessageId::zero();
    for _ in 0..5 {
        last_id = fixed_counter.request_increment().await.unwrap();
    }

    let res = fixed_counter.request_increment().await;
    assert!(res.is_err());

    let removed = fixed_counter.remove_fixed(last_id).await.unwrap();
    assert_eq!(removed, Some(OpStatus::Pending));

    fixed_counter.request_increment().await.unwrap();
}

#[tokio::test]
async fn test_update_non_existent() {
    let (program, _env) = deploy_program().await;
    let mut dynamic_counter = program.dynamic_counter();

    let random_id = MessageId::from([1u8; 32]);
    let updated = dynamic_counter
        .update_dynamic(random_id, OpStatus::Completed)
        .await
        .unwrap();
    assert!(!updated);
}

#[tokio::test]
async fn test_fixed_storage_mixed_workload() {
    let (program, _env) = deploy_program().await;
    let mut fixed_counter = program.fixed_counter();

    let id1 = fixed_counter.request_increment().await.unwrap();
    let id2 = fixed_counter.request_increment().await.unwrap();
    let id3 = fixed_counter.request_increment().await.unwrap();

    let updated = fixed_counter
        .update_fixed(id2, OpStatus::Completed)
        .await
        .unwrap();
    assert!(updated);
    assert_eq!(
        fixed_counter.get_status(id2).await.unwrap(),
        Some(OpStatus::Completed)
    );

    fixed_counter.remove_fixed(id1).await.unwrap();

    fixed_counter.request_increment().await.unwrap();
    fixed_counter.request_increment().await.unwrap();
    fixed_counter.request_increment().await.unwrap();

    assert!(fixed_counter.request_increment().await.is_err());
}

#[tokio::test]
async fn test_clear_and_reutilize() {
    let (program, _env) = deploy_program().await;
    let mut fixed_counter = program.fixed_counter();

    for _ in 0..5 {
        fixed_counter.request_increment().await.unwrap();
    }
    assert!(fixed_counter.request_increment().await.is_err());

    fixed_counter.clear_fixed().await.unwrap();

    for _ in 0..5 {
        fixed_counter.request_increment().await.unwrap();
    }
}
