# Awesome Sails VFT Admin

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Administrative service for the Awesome Sails VFT. This service provides privileged operations such as minting, burning, pausing the contract, and managing allowance shards. It relies on the Access Control service for permission management.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft-admin = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the VFT Admin service, you must instantiate it with references to the Access Control, VFT, and storage components.

```rust
#![no_std]

use awesome_sails_vft_admin::VftAdmin;
use awesome_sails_access_control::{AccessControl, RolesStorage};
use awesome_sails_vft::Vft;
use awesome_sails_vft::utils::{Allowances, Balances};
use awesome_sails_utils::{pause::{PausableRef, Pause}, storage::StorageRefCell};
use core::cell::RefCell;
use sails_rs::prelude::*;

#[derive(Default)]
pub struct Program {
    access_control: RefCell<RolesStorage>,
    allowances: RefCell<Allowances>,
    balances: RefCell<Balances>,
    pause: Pause,
}

impl Program {
    pub fn access_control(&self) -> AccessControl<'_, StorageRefCell<'_, RolesStorage>> {
        AccessControl::new(StorageRefCell::new(&self.access_control))
    }

    pub fn allowances(&self) -> PausableRef<'_, Allowances> {
        PausableRef::new(&self.pause, StorageRefCell::new(&self.allowances))
    }

    pub fn balances(&self) -> PausableRef<'_, Balances> {
        PausableRef::new(&self.pause, StorageRefCell::new(&self.balances))
    }

    pub fn vft(&self) -> Vft<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        Vft::new(self.allowances(), self.balances())
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vft_admin(&self) -> VftAdmin<'_, StorageRefCell<'_, RolesStorage>, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftAdmin::new(
            self.access_control(),
            self.allowances(),
            self.balances(),
            &self.pause,
            self.vft(),
        )
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

The admin service requires specific roles to be granted to the caller. The role IDs are available as public constants in the crate (e.g., `MINTER_ROLE`, `BURNER_ROLE`, `PAUSER_ROLE`).

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
use awesome_sails_vft_admin::MINTER_ROLE;

#[tokio::test]
async fn test_minting() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, pid) = deploy_program().await;
    let mut vft_admin_service = program.vft_admin();
    let mut access_control = program.access_control();

    // Grant MINTER_ROLE to Alice
    // MINTER_ROLE is a public constant from the awesome-sails-vft-admin crate
    access_control.grant_role(MINTER_ROLE, ALICE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // Alice mints tokens to Bob
    let res = vft_admin_service
        .mint(BOB, 1000.into())
        .with_actor_id(ALICE)
        .await;

    assert!(res.is_ok());
}
```
