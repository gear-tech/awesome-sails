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

use access_control_test_app::{MEMBERS_LIMIT, ROLES_LIMIT};
use access_control_test_client::{
    AccessControlTestClient,
    access_control::Error,
    access_control::Pagination,
    access_control::{AccessControl, events::AccessControlEvents},
};
use awesome_sails::access_control::{RoleId, default_admin_role};
use awesome_sails_utils::assert_ok;
use common::{ALICE, BOB, CHARLIE, DAVE, deploy_program};
use futures::StreamExt;
use sails_rs::prelude::*;

const MINTER_ROLE: RoleId = [1; 32];
const MODERATOR_ROLE: RoleId = [2; 32];
const PAUSER_ROLE: RoleId = [3; 32];

#[tokio::test]
async fn initial_admin_role_granted() {
    let (program, _env, _pid) = deploy_program().await;
    let access_control_service = program.access_control();

    // Alice should have default_admin_role()
    let has_role = access_control_service
        .has_role(default_admin_role(), ALICE)
        .await;
    assert_ok!(has_role, true);

    // Bob should not have default_admin_role()
    let has_role = access_control_service
        .has_role(default_admin_role(), BOB)
        .await;
    assert_ok!(has_role, false);
}

#[tokio::test]
async fn grant_and_revoke_role_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    // Alice (default_admin_role()) grants MINTER_ROLE to Bob
    let _ = access_control_service
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
    let _ = access_control_service
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
        .await
        .unwrap();
    assert_eq!(
        res.unwrap_err(),
        Error(format!(
            "Access denied: account {:?} does not have role {:?}",
            CHARLIE,
            default_admin_role()
        ))
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
    let _ = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Charlie tries to revoke MINTER_ROLE from Bob (unauthorized)
    let res = access_control_service
        .revoke_role(MINTER_ROLE, BOB)
        .with_actor_id(CHARLIE)
        .await
        .unwrap();
    assert_eq!(
        res.unwrap_err(),
        Error(format!(
            "Access denied: account {:?} does not have role {:?}",
            CHARLIE,
            default_admin_role()
        ))
    );

    // Bob should still have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, true);
}

#[tokio::test]
async fn renounce_role_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    // Alice grants PAUSER_ROLE to Charlie
    let _ = access_control_service
        .grant_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Charlie renounces PAUSER_ROLE himself
    let _ = access_control_service
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
    let _ = access_control_service
        .grant_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Bob tries to renounce PAUSER_ROLE for Charlie (unauthorized)
    let res = access_control_service
        .renounce_role(PAUSER_ROLE, CHARLIE)
        .with_actor_id(BOB)
        .await
        .unwrap();
    assert_eq!(
        res.unwrap_err(),
        Error(format!(
            "Not account owner: account {:?}, message source {:?}",
            CHARLIE, BOB
        ))
    );

    // Charlie should still have PAUSER_ROLE
    let has_role = access_control_service.has_role(PAUSER_ROLE, CHARLIE).await;
    assert_ok!(has_role, true);
}

#[tokio::test]
async fn set_role_admin_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    // Initial admin for MINTER_ROLE is default_admin_role() (Alice)
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, default_admin_role());

    // Alice (as default_admin_role()) grants MODERATOR_ROLE to Dave
    let _ = access_control_service
        .grant_role(MODERATOR_ROLE, DAVE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Alice (as default_admin_role()) sets MODERATOR_ROLE as admin for MINTER_ROLE
    let _ = access_control_service
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
            previous_admin_role_id: default_admin_role(),
            new_admin_role_id: MODERATOR_ROLE,
            sender: ALICE,
        }
    );

    // Now, admin for MINTER_ROLE should be MODERATOR_ROLE
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, MODERATOR_ROLE);

    // Dave (as MODERATOR_ROLE) should be able to grant MINTER_ROLE
    let _ = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(DAVE)
        .await
        .expect("Failed for Dave to grant MINTER_ROLE to Bob");
    events.next().await.unwrap(); // Consume RoleGranted event

    // Bob should have MINTER_ROLE
    let has_role = access_control_service.has_role(MINTER_ROLE, BOB).await;
    assert_ok!(has_role, true);

    // Alice (as default_admin_role()) should STILL be able to grant MINTER_ROLE (because she is super admin)
    let _ = access_control_service
        .grant_role(MINTER_ROLE, CHARLIE)
        .with_actor_id(ALICE)
        .await
        .expect("Alice (super admin) should still be able to grant roles");
    events.next().await.unwrap(); // Consume RoleGranted event

    let has_role = access_control_service.has_role(MINTER_ROLE, CHARLIE).await;
    assert_ok!(has_role, true);

    // Revert admin role to default_admin_role()
    let _ = access_control_service
        .set_role_admin(MINTER_ROLE, default_admin_role())
        .with_actor_id(DAVE) // Dave is MODERATOR_ROLE, which is admin for MINTER_ROLE
        .await
        .expect("Failed for Dave to revert admin role");
    events.next().await.unwrap(); // Consume RoleAdminChanged event

    // Alice should now be able to grant MINTER_ROLE again
    let _ = access_control_service
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

    // Charlie tries to set admin for MINTER_ROLE (unauthorized, only default_admin_role() can do it initially)
    let res = access_control_service
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(CHARLIE)
        .await
        .unwrap();
    assert_eq!(
        res.unwrap_err(),
        Error(format!(
            "Access denied: account {:?} does not have role {:?}",
            CHARLIE,
            default_admin_role()
        ))
    );

    // Admin for MINTER_ROLE should still be default_admin_role()
    let admin_role = access_control_service.get_role_admin(MINTER_ROLE).await;
    assert_ok!(admin_role, default_admin_role());
}

#[tokio::test]
async fn multiple_roles() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    // Alice grants MINTER_ROLE to Bob
    let _ = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap(); // Consume RoleGranted event

    // Alice grants PAUSER_ROLE to Bob
    let _ = access_control_service
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
async fn self_admin_role_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Alice sets MINTER_ROLE as its own admin
    let _ = access_control_service
        .set_role_admin(MINTER_ROLE, MINTER_ROLE)
        .with_actor_id(ALICE)
        .await
        .expect("Failed to set self-admin role");

    // 1. Alice (Super Admin) should still be able to grant MINTER_ROLE to Bob
    // even though she doesn't have MINTER_ROLE herself.
    let _ = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Super Admin should be able to grant self-administered role");

    assert_ok!(
        access_control_service.has_role(MINTER_ROLE, BOB).await,
        true
    );

    // 2. Bob (who has MINTER_ROLE) should now be able to grant it to Charlie
    // because MINTER_ROLE is the admin for MINTER_ROLE.
    let _ = access_control_service
        .grant_role(MINTER_ROLE, CHARLIE)
        .with_actor_id(BOB)
        .await
        .expect("Member of self-administered role should be able to grant it to others");

    assert_ok!(
        access_control_service.has_role(MINTER_ROLE, CHARLIE).await,
        true
    );
}

#[tokio::test]
async fn batch_grant_success() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    let roles = vec![MINTER_ROLE, MODERATOR_ROLE, PAUSER_ROLE];

    let _ = access_control_service
        .grant_roles_batch(roles.clone(), BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Batch grant failed");

    for role_id in roles.clone() {
        let (actor, event) = events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            AccessControlEvents::RoleGranted {
                role_id,
                target_account: BOB,
                sender: ALICE,
            }
        );
    }

    for role in roles {
        assert_ok!(access_control_service.has_role(role, BOB).await, true);
    }
}

#[tokio::test]
async fn batch_revoke_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    let roles = vec![MINTER_ROLE, MODERATOR_ROLE, PAUSER_ROLE];
    let _ = access_control_service
        .grant_roles_batch(roles.clone(), BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    let _ = access_control_service
        .revoke_roles_batch(roles.clone(), BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Batch revoke failed");

    for role in roles {
        assert_ok!(access_control_service.has_role(role, BOB).await, false);
    }
}

#[tokio::test]
async fn enumeration_roles_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    let roles = vec![MINTER_ROLE, MODERATOR_ROLE, PAUSER_ROLE];
    let _ = access_control_service
        .grant_roles_batch(roles.clone(), BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // count: admin + 3 new = 4
    assert_ok!(access_control_service.get_role_count().await, 4);

    let all_roles = access_control_service.get_roles(None).await.unwrap();
    assert_eq!(all_roles.len(), 4);
    assert!(all_roles.contains(&default_admin_role()));
    for r in &roles {
        assert!(all_roles.contains(r));
    }
}

#[tokio::test]
async fn enumeration_members_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    let members = vec![BOB, CHARLIE, DAVE];
    for &m in &members {
        let _ = access_control_service
            .grant_role(MINTER_ROLE, m)
            .with_actor_id(ALICE)
            .await
            .unwrap();
    }

    assert_ok!(
        access_control_service
            .get_role_member_count(MINTER_ROLE)
            .await,
        3
    );

    let all_members = access_control_service
        .get_role_members(MINTER_ROLE, None)
        .await
        .unwrap();
    assert_eq!(all_members.len(), 3);
    for m in &members {
        assert!(all_members.contains(m));
    }
}

#[tokio::test]
async fn enumeration_member_roles_success() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    let roles = vec![MINTER_ROLE, MODERATOR_ROLE, PAUSER_ROLE];
    let _ = access_control_service
        .grant_roles_batch(roles.clone(), BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    assert_ok!(access_control_service.get_member_role_count(BOB).await, 3);

    let bob_roles = access_control_service
        .get_member_roles(BOB, None)
        .await
        .unwrap();
    assert_eq!(bob_roles.len(), 3);
    for r in &roles {
        assert!(bob_roles.contains(r));
    }
}

#[tokio::test]
async fn batch_grant_atomic_failure() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // 1. Setup: Bob is admin only for MINTER_ROLE
    let _ = access_control_service
        .grant_role(MODERATOR_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    let _ = access_control_service
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // 2. Bob tries to grant [MINTER_ROLE, PAUSER_ROLE] to Charlie.
    // He has rights for the first, but NOT for the second.
    let roles = vec![MINTER_ROLE, PAUSER_ROLE];
    let res = access_control_service
        .grant_roles_batch(roles, CHARLIE)
        .with_actor_id(BOB)
        .await
        .unwrap();

    // 3. Must fail
    assert!(res.is_err());

    // 4. Verification: Charlie must have NO roles (even MINTER_ROLE)
    let charlie_roles = access_control_service
        .get_member_roles(CHARLIE, None)
        .await
        .unwrap();
    assert!(charlie_roles.is_empty());
}

#[tokio::test]
async fn enumeration_pagination_logic() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // 1. Grant 10 roles to Bob (Total roles: 1 admin + 10 new = 11)
    let roles: Vec<RoleId> = (1..=10).map(|i| [i as u8; 32]).collect();
    let _ = access_control_service
        .grant_roles_batch(roles, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // 2. Case: None (Get all)
    let all = access_control_service.get_roles(None).await.unwrap();
    assert_eq!(all.len(), 11);

    // 3. Case: Offset 0, Limit 5 (First page)
    let page1 = access_control_service
        .get_roles(Some(Pagination {
            offset: 0,
            limit: 5,
        }))
        .await
        .unwrap();
    assert_eq!(page1.len(), 5);
    assert_eq!(page1, all[0..5]);

    // 4. Case: Offset 5, Limit 2 (Middle small page)
    let page2 = access_control_service
        .get_roles(Some(Pagination {
            offset: 5,
            limit: 2,
        }))
        .await
        .unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2, all[5..7]);

    // 5. Case: Limit > Offset (e.g. Offset 2, Limit 10)
    // Works fine: starts at index 2 and tries to take 10.
    let page3 = access_control_service
        .get_roles(Some(Pagination {
            offset: 2,
            limit: 10,
        }))
        .await
        .unwrap();
    assert_eq!(page3.len(), 9); // only 9 left from index 2 to 10
    assert_eq!(page3, all[2..11]);

    // 6. Case: Offset at the end, Large Limit
    let last = access_control_service
        .get_roles(Some(Pagination {
            offset: 10,
            limit: 100,
        }))
        .await
        .unwrap();
    assert_eq!(last.len(), 1);
    assert_eq!(last[0], all[10]);

    // 7. Case: Offset out of bounds
    let empty = access_control_service
        .get_roles(Some(Pagination {
            offset: 100,
            limit: 10,
        }))
        .await
        .unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn stress_test_max_members() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    let mut members = Vec::with_capacity(MEMBERS_LIMIT);
    for i in 1..=MEMBERS_LIMIT {
        let mut id = [0u8; 32];
        id[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        members.push(ActorId::from(id));
    }

    for &member in &members {
        let _ = access_control_service
            .grant_role(MINTER_ROLE, member)
            .with_actor_id(ALICE)
            .await
            .unwrap();
    }

    let count = access_control_service
        .get_role_member_count(MINTER_ROLE)
        .await
        .unwrap();
    assert_eq!(count, MEMBERS_LIMIT as u32);

    for &member in &members {
        let has_role = access_control_service
            .has_role(MINTER_ROLE, member)
            .await
            .unwrap();
        assert!(has_role, "Member {:?} should have the role", member);
    }

    let contract_members = access_control_service
        .get_role_members(MINTER_ROLE, None)
        .await
        .unwrap();

    assert_eq!(contract_members.len(), MEMBERS_LIMIT);
    for i in 0..MEMBERS_LIMIT {
        assert_eq!(contract_members[i], members[i], "Mismatch at index {}", i);
    }
}

#[tokio::test]
async fn roles_capacity_exceeded() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // 1st role is default admin, so we can add custom roles up to the limit
    for i in 1..ROLES_LIMIT {
        let mut rid = [0u8; 32];
        rid[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        access_control_service
            .grant_role(rid, BOB)
            .with_actor_id(ALICE)
            .await
            .unwrap()
            .expect("Failed to grant a valid role");
    }

    assert_eq!(
        access_control_service.get_role_count().await.unwrap(),
        ROLES_LIMIT as u32
    );

    // Try to add one more role beyond the limit
    let mut extra_rid = [0u8; 32];
    extra_rid[0..4].copy_from_slice(&(ROLES_LIMIT as u32 + 1).to_le_bytes());
    let res = access_control_service
        .grant_role(extra_rid, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("grant_role should return service result");

    assert_eq!(res.unwrap_err(), Error("Capacity exceeded".into()));
}

#[tokio::test]
async fn members_capacity_exceeded() {
    let (program, _env, pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    for i in 1..=MEMBERS_LIMIT {
        let mut id = [0u8; 32];
        id[0..4].copy_from_slice(&(i as u32 + 1000).to_le_bytes());
        let member = ActorId::from(id);

        let _ = access_control_service
            .grant_role(MINTER_ROLE, member)
            .with_actor_id(ALICE)
            .await
            .unwrap();

        let (actor, event) = events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            AccessControlEvents::RoleGranted {
                role_id: MINTER_ROLE,
                target_account: member,
                sender: ALICE,
            }
        );
    }

    let count = access_control_service
        .get_role_member_count(MINTER_ROLE)
        .await
        .unwrap();
    assert_eq!(count, MEMBERS_LIMIT as u32);

    // Try to add one more member beyond the limit
    let res = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("grant_role should return service result");

    assert_eq!(res.unwrap_err(), Error("Capacity exceeded".into()));
}

/// Tests that require_role works correctly for default_admin_role.
///
/// This verifies that:
/// 1. Super-admin (default admin) can grant roles (fast-path: has_member(admin))
/// 2. Super-admin can set_role_admin (internal require_role(default_admin_role(), ...))
/// 3. Non-admin cannot grant roles (internal require_role fails)
#[tokio::test]
async fn require_role_admin_path_works() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();
    let mut events = access_control_service.listen().await.unwrap();

    let _ = access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap();

    let res = access_control_service
        .grant_role(MODERATOR_ROLE, CHARLIE)
        .with_actor_id(BOB)
        .await
        .unwrap();
    assert_eq!(
        res.unwrap_err(),
        Error(format!(
            "Access denied: account {:?} does not have role {:?}",
            BOB,
            default_admin_role()
        ))
    );

    let _ = access_control_service
        .grant_role(MODERATOR_ROLE, DAVE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap();

    let _ = access_control_service
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(ALICE)
        .await
        .unwrap();
    events.next().await.unwrap();

    let _ = access_control_service
        .grant_role(MINTER_ROLE, CHARLIE)
        .with_actor_id(DAVE)
        .await
        .unwrap();
    events.next().await.unwrap();

    let has_role = access_control_service
        .has_role(MINTER_ROLE, CHARLIE)
        .await
        .unwrap();
    assert!(has_role);
}
