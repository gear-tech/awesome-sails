# Awesome Sails Access Control

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A role-based access control (RBAC) service for Awesome Sails. This service implements a mechanism with support for role hierarchies, enumeration, and batch operations, allowing for granular permission management within your dApp.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-access-control = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the Access Control service in your Sails program, you need to include its storage in your program struct and expose the service.

```rust
#![no_std]

use awesome_sails_access_control::{AccessControl, RolesStorage};
use awesome_sails_utils::storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Default)]
pub struct Program {
    roles: RefCell<RolesStorage>,
}

#[program]
impl Program {
    pub fn new() -> Self {
        let mut storage = RolesStorage::default();
        let deployer = Syscall::message_source();

        // Grant the deployer the default admin role
        storage.grant_initial_admin(deployer);

        Self {
            roles: RefCell::new(storage),
        }
    }

    // Expose the Access Control service
    pub fn access_control(&self) -> AccessControl<'_> {
        AccessControl::new(StorageRefCell::new(&self.roles))
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following example demonstrates how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
mod common; // Helpers defined in tests/common/mod.rs

use access_control_test_client::{
    AccessControlTestClient, Pagination,
    access_control::AccessControl,
};
use awesome_sails::access_control::RoleId;
use common::{ALICE, BOB, CHARLIE, deploy_program};
use sails_rs::prelude::*;

const MINTER_ROLE: RoleId = [1u8; 32];
const BURNER_ROLE: RoleId = [2u8; 32];
const MODERATOR_ROLE: RoleId = [3u8; 32];

#[tokio::test]
async fn test_access_control() {
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control = program.access_control();

    // 1. Grant a role
    access_control
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Grant failed");

    // Verify Bob has the role
    let has_role = access_control.has_role(MINTER_ROLE, BOB).await.unwrap();
    assert!(has_role);

    // 2. Grant multiple roles at once
    access_control
        .grant_roles_batch(vec![MINTER_ROLE, BURNER_ROLE], CHARLIE)
        .with_actor_id(ALICE)
        .await
        .expect("Batch grant failed");

    // Verify Charlie has roles
    assert!(access_control.has_role(MINTER_ROLE, CHARLIE).await.unwrap());
    assert!(access_control.has_role(BURNER_ROLE, CHARLIE).await.unwrap());

    // 3. Set MODERATOR_ROLE as the admin for MINTER_ROLE
    access_control
        .set_role_admin(MINTER_ROLE, MODERATOR_ROLE)
        .with_actor_id(ALICE)
        .await
        .expect("Set admin failed");

    // Verify admin role
    let admin = access_control.get_role_admin(MINTER_ROLE).await.unwrap();
    assert_eq!(admin, MODERATOR_ROLE);

    // 4. Enumeration
    let count = access_control.get_role_count().await.unwrap();
    // Default admin, MINTER_ROLE (granted to BOB), BURNER_ROLE (granted to CHARLIE)
    assert_eq!(count, 3);

    // Get roles with pagination
    let roles = access_control
        .get_roles(Some(Pagination {
            offset: 0,
            limit: 10,
        }))
        .await
        .unwrap();
    assert_eq!(roles.len(), 3);

    // Get members of MINTER_ROLE
    let members = access_control
        .get_role_members(MINTER_ROLE, None)
        .await
        .unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&BOB));
    assert!(members.contains(&CHARLIE));
}
```
