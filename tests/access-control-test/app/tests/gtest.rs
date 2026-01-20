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

mod common;

use access_control_test_client::{
    AccessControlTestClient,
    access_control::{AccessControl, events::AccessControlEvents},
};
use awesome_sails_access_control_service::{DEFAULT_ADMIN_ROLE, RoleId};
use awesome_sails_utils::assert_ok;
use common::{ALICE, BOB, CHARLIE, DAVE, assert_str_panic, deploy_program};
use futures::StreamExt;
use sails_rs::prelude::*;

const MINTER_ROLE: RoleId = [1; 32];
const MODERATOR_ROLE: RoleId = [2; 32];
const PAUSER_ROLE: RoleId = [3; 32];

#[tokio::test]
async fn initial_admin_role_granted() {
    let (program, _env, _pid) = deploy_program().await;
    let access_control_service = program.access_control();

    // Alice should have DEFAULT_ADMIN_ROLE
    let has_role = access_control_service
        .has_role(DEFAULT_ADMIN_ROLE, ALICE)
        .await;
    assert_ok!(has_role, true);

    // Bob should not have DEFAULT_ADMIN_ROLE
    let has_role = access_control_service
        .has_role(DEFAULT_ADMIN_ROLE, BOB)
        .await;
    assert_ok!(has_role, false);
}

#[tokio::test]
async fn grant_and_revoke_role_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let listener = access_control_service.listener();
    let mut events = listener.listen().await.unwrap();

    // Alice (DEFAULT_ADMIN_ROLE) grants MINTER_ROLE to Bob
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Failed to grant MINTER_ROLE to Bob");

    let (actor, event) = events.next().await.unwrap();
    assert_eq!(actor, pid);
    assert_eq!(
        event,
        AccessControlEvents::RoleGranted {
            role_id: MINTER_ROLE,
            target_account: BOB,
            sender: ALICE,
        }
    );

    // Bob should now have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, true);

    // Alice revokes MINTER_ROLE from Bob
    access_control_service
        .revoke_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Failed to revoke MINTER_ROLE from Bob");

    let (actor, event) = events.next().await.unwrap();
    assert_eq!(actor, pid);
    assert_eq!(
        event,
        AccessControlEvents::RoleRevoked {
            role_id: MINTER_ROLE,
            target_account: BOB,
            sender: ALICE,
        }
    );

    // Bob should no longer have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, false);
}

#[tokio::test]
async fn grant_role_fail_unauthorized() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Charlie tries to grant MINTER_ROLE to Dave (unauthorized)
    let res = access_control_service
        .grant_role(MINTER_ROLE, DAVE)
        .with_actor_id(CHARLIE)
        .await;
    assert_str_panic(
        res.unwrap_err(),
        "Access denied: account 0x0000000000000000000000002c00000000000000000000000000000000000000 does not have role [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]",
    );

    // Dave should not have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, DAVE).await;
    assert_ok!(has_role, false);
}

#[tokio::test]
async fn revoke_role_fail_unauthorized() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Alice grants MINTER_ROLE to Bob
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Charlie tries to revoke MINTER_ROLE from Bob (unauthorized)
    let res = access_control_service
        .revoke_role(MINTER_ROLE, BOB)
        .with_actor_id(CHARLIE)
        .await;
    assert_str_panic(
        res.unwrap_err(),
        "Access denied: account 0x0000000000000000000000002c00000000000000000000000000000000000000 does not have role [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]",
    );

    // Bob should still have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, true);
}

#[tokio::test]
async fn renounce_role_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let listener = access_control_service.listener();
    let mut events = listener.listen().await.unwrap();

    // Alice grants PAUSER_ROLE to Charlie
    access_control_service
        .grant_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Charlie renounces PAUSER_ROLE himself
    access_control_service
        .renounce_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(CHARLIE)
        .await
        .expect("Failed for Charlie to renounce PAUSER_ROLE");

    let (actor, event) = events.next().await.unwrap();
    assert_eq!(actor, pid);
    assert_eq!(
        event,
        AccessControlEvents::RoleRevoked {
            role_id: PAUSER_ROLE,
            target_account: CHARLIE,
            sender: CHARLIE,
        }
    );

    // Charlie should no longer have PAUSER_ROLE
    let has_role = access_control_service.has_role(PAUSER_ROLE, CHARLIE).await;
    assert_ok!(has_role, false);
}

#[tokio::test]
async fn renounce_role_fail_other_account() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Alice grants PAUSER_ROLE to Charlie
    access_control_service
        .grant_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Bob tries to renounce PAUSER_ROLE for Charlie (unauthorized)
    let res = access_control_service
        .renounce_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(BOB)
        .await;
    assert_str_panic(
        res.unwrap_err(),
        "Not account owner: account 0x0000000000000000000000002c00000000000000000000000000000000000000, message source 0x0000000000000000000000002b00000000000000000000000000000000000000",
    );

    // Charlie should still have PAUSER_ROLE
    let has_role = access_control_service.has_role(PAUSER_ROLE, CHARLIE).await;
    assert_ok!(has_role, true);
}

#[tokio::test]
async fn set_role_admin_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let listener = access_control_service.listener();
    let mut events = listener.listen().await.unwrap();

    // Initial admin for MINTER_ROLE is DEFAULT_ADMIN_ROLE (Alice)
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, DEFAULT_ADMIN_ROLE);

    // Alice (as DEFAULT_ADMIN_ROLE) grants MODERATOR_ROLE to Dave
    access_control_service
        .grant_role(MODERATOR_ROLE, DAVE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Alice (as DEFAULT_ADMIN_ROLE) sets MODERATOR_ROLE as admin for MINTER_ROLE
    access_control_service
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(ALICE)
        .await
        .expect("Failed for Alice to set MODERATOR_ROLE as admin for MINTER_ROLE");

    let (actor, event) = events.next().await.unwrap();
    assert_eq!(actor, pid);
    assert_eq!(
        event,
        AccessControlEvents::RoleAdminChanged {
            role_id: MINTER_ROLE,
            previous_admin_role_id: DEFAULT_ADMIN_ROLE,
            new_admin_role_id: MODERATOR_ROLE,
        }
    );

    // Now, admin for MINTER_ROLE should be MODERATOR_ROLE
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, MODERATOR_ROLE);

    // Dave (as MODERATOR_ROLE) should be able to grant MINTER_ROLE
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(DAVE)
        .await
        .expect("Failed for Dave to grant MINTER_ROLE to Bob");
    events.next().await.unwrap(); // Consume RoleGranted event

    // Bob should have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, true);

    // Alice (as DEFAULT_ADMIN_ROLE) should STILL be able to grant MINTER_ROLE (because she is super admin)
    access_control_service
        .grant_role(MINTER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .expect("Alice (super admin) should still be able to grant roles");
    events.next().await.unwrap(); // Consume RoleGranted event

    let has_role = access_control_service.has_role(MINTER_ROLE, CHARLIE).await;
    assert_ok!(has_role, true);

    // Revert admin role to DEFAULT_ADMIN_ROLE
    access_control_service
        .set_role_admin(MINTER_ROLE, DEFAULT_ADMIN_ROLE)
        .with_actor_id(DAVE) // Dave is MODERATOR_ROLE, which is admin for MINTER_ROLE
        .await
        .expect("Failed for Dave to revert admin role");
    events.next().await.unwrap(); // Consume RoleAdminChanged event

    // Alice should now be able to grant MINTER_ROLE again
    access_control_service
        .grant_role(MINTER_ROLE, DAVE)
        .with_actor_id(ALICE)
        .await
        .expect("Failed for Alice to grant MINTER_ROLE to Dave after revert");
    events.next().await.unwrap(); // Consume RoleGranted event
    let has_role = access_control_service.has_role(MINTER_ROLE, DAVE).await;
    assert_ok!(has_role, true);
}

#[tokio::test]
async fn set_role_admin_fail_unauthorized() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Charlie tries to set admin for MINTER_ROLE (unauthorized, only DEFAULT_ADMIN_ROLE can do it initially)
    let res = access_control_service
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(CHARLIE)
        .await;
    assert_str_panic(
        res.unwrap_err(),
        "Access denied: account 0x0000000000000000000000002c00000000000000000000000000000000000000 does not have role [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]",
    );

    // Admin for MINTER_ROLE should still be DEFAULT_ADMIN_ROLE
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, DEFAULT_ADMIN_ROLE);
}

#[tokio::test]
async fn multiple_roles() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let listener = access_control_service.listener();
    let mut events = listener.listen().await.unwrap();

    // Alice grants MINTER_ROLE to Bob
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Alice grants PAUSER_ROLE to Bob
    access_control_service
        .grant_role(PAUSER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Bob should have both roles
    let has_minter_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_minter_role, true);

    let has_pauser_role = access_control_service.has_role(PAUSER_ROLE, BOB).await;
    assert_ok!(has_pauser_role, true);
}

#[tokio::test]
async fn enumeration_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Initial roles should contain at least DEFAULT_ADMIN_ROLE
    let role_count = access_control_service.get_role_count().await;
    assert_ok!(role_count, 1);

    let role_id = access_control_service.get_role_id(0).await;
    assert_ok!(role_id, Some(DEFAULT_ADMIN_ROLE));

    // Alice should be the only member of DEFAULT_ADMIN_ROLE
    let member_count = access_control_service
        .get_role_member_count(DEFAULT_ADMIN_ROLE)
        .await;
    assert_ok!(member_count, 1);

    let member = access_control_service
        .get_role_member(DEFAULT_ADMIN_ROLE, 0)
        .await;
    assert_ok!(member, Some(ALICE));

    // Alice grants MINTER_ROLE to Bob and Charlie
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    access_control_service
        .grant_role(MINTER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Now MINTER_ROLE should have 2 members
    let member_count = access_control_service
        .get_role_member_count(MINTER_ROLE)
        .await;
    assert_ok!(member_count, 2);

    // Check members by index (BTreeMap/BTreeSet ensures order)
    let m0 = access_control_service
        .get_role_member(MINTER_ROLE, 0)
        .await
        .expect("get_role_member failed");
    let m1 = access_control_service
        .get_role_member(MINTER_ROLE, 1)
        .await
        .expect("get_role_member failed");

    let mut members = [m0.unwrap(), m1.unwrap()];
    members.sort(); // ActorIds in BTreeSet are sorted

    let mut expected = [BOB, CHARLIE];
    expected.sort();

    assert_eq!(members, expected);
}
