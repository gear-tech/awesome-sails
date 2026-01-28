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
use awesome_sails_utils::storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Default)]
pub struct Program {
    access_control: RefCell<RolesStorage>,
    allowances: RefCell<Allowances>,
    balances: RefCell<Balances>,
    pause: Pause,
}

impl Program {
    pub fn access_control(&self) -> AccessControl<'_> {
        AccessControl::new(StorageRefCell::new(&self.access_control))
    }

    pub fn allowances(&self) -> PausableRef<'_, Allowances> {
        PausableRef::new(&self.pause, StorageRefCell::new(&self.allowances))
    }

    pub fn balances(&self) -> PausableRef<'_, Balances> {
        PausableRef::new(&self.pause, StorageRefCell::new(&self.balances))
    }

    pub fn vft(&self) -> Vft<'_> {
        Vft::new(self.allowances(), self.balances())
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vft_admin(&self) -> VftAdmin<'_> {
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

The following example demonstrates how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
mod common; // Helpers defined in tests/common/mod.rs

use awesome_sails::vft_admin::{BURNER_ROLE, MINTER_ROLE, PAUSER_ROLE};
use awesome_sails_test_client::{
    access_control::AccessControl,
    vft::Vft,
    vft_admin::VftAdmin,
    AwesomeSailsTestClient,
};
use common::{ALICE, BOB, deploy_with_data};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_vft_admin() {
    // deploy_with_data is a helper from the common module
    let (program, _env, _pid) = deploy_with_data(Default::default(), Default::default(), 1).await;

    let mut vft_admin = program.vft_admin();
    let vft = program.vft();
    let mut access_control = program.access_control();

    // 1. Grant MINTER_ROLE to ALICE
    access_control
        .grant_role(MINTER_ROLE, ALICE)
        .with_actor_id(ALICE)
        .await
        .expect("Grant MINTER_ROLE failed");

    // 2. Mint tokens to BOB
    vft_admin
        .mint(BOB, 1000.into())
        .with_actor_id(ALICE)
        .await
        .expect("Mint failed");

    // Verify balance
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 1000.into());

    // 3. Grant BURNER_ROLE to ALICE and burn tokens from BOB
    access_control
        .grant_role(BURNER_ROLE, ALICE)
        .with_actor_id(ALICE)
        .await
        .expect("Grant BURNER_ROLE failed");

    vft_admin
        .burn(BOB, 500.into())
        .with_actor_id(ALICE)
        .await
        .expect("Burn failed");

    // Verify balance after burn
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 500.into());

    // 4. Grant PAUSER_ROLE and pause the contract
    access_control
        .grant_role(PAUSER_ROLE, ALICE)
        .with_actor_id(ALICE)
        .await
        .expect("Grant PAUSER_ROLE failed");

    vft_admin
        .pause()
        .with_actor_id(ALICE)
        .await
        .expect("Pause failed");

    // Verify paused state
    let is_paused = vft_admin.is_paused().await.unwrap();
    assert!(is_paused);
}
```
