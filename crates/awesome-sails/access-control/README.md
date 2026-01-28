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
    pub fn access_control(&self) -> AccessControl<'_, StorageRefCell<'_, RolesStorage>> {
        AccessControl::new(StorageRefCell::new(&self.roles))
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_access_control() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, _pid) = deploy_program().await;
    let mut access_control_service = program.access_control();

    // Define roles (32-byte arrays)
    const MINTER_ROLE: [u8; 32] = [1u8; 32];

    // Grant MINTER_ROLE to Bob
    access_control_service
        .grant_role(MINTER_ROLE, BOB)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Check if Bob has the role
    let has_role = access_control_service
        .has_role(MINTER_ROLE, BOB)
        .await
        .unwrap();

    assert!(has_role);
}
```
