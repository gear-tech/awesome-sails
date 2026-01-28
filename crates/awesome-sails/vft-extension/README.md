# Awesome Sails VFT Extension

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Extended functionality for the Awesome Sails VFT. This service provides features such as cleaning up expired allowances, transferring entire balances, enumerating allowances and balances, and explicit shard management.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft-extension = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the VFT Extension service, instantiate it with references to the allowance and balance storage, and the base VFT service.

```rust
#![no_std]

use awesome_sails_vft_extension::VftExtension;
use awesome_sails_vft::Vft;
use awesome_sails_vft::utils::{Allowances, Balances};
use awesome_sails_utils::storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Default)]
pub struct Program {
    allowances: RefCell<Allowances>,
    balances: RefCell<Balances>,
    pause: Pause,
}

impl Program {
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

    pub fn vft_extension(&self) -> VftExtension<'_> {
        VftExtension::new(self.allowances(), self.balances(), self.vft())
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following example demonstrates how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
mod common; // Helpers defined in tests/common/mod.rs

use awesome_sails_test_client::{
    AwesomeSailsTestClient,
    vft::Vft,
    vft_extension::VftExtension,
};
use common::{ALICE, BOB, deploy_with_data};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_vft_extension() {
    // deploy_with_data is a helper from the common module
    // Deploy with ALICE having 1000 tokens
    let (program, _env, _pid) = deploy_with_data(Default::default(), vec![(ALICE, 1000.into())], 1).await;
    
    let mut vft_extension = program.vft_extension();
    let vft = program.vft();

    // 1. Transfer all tokens from ALICE to BOB
    vft_extension
        .transfer_all(BOB)
        .with_actor_id(ALICE)
        .await
        .expect("Transfer all failed");

    // Verify balances
    let balance_alice = vft.balance_of(ALICE).await.unwrap();
    assert_eq!(balance_alice, 0.into());
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 1000.into());

    // 2. Check detailed allowance
    let mut vft_mut = program.vft();
    vft_mut.approve(BOB, 500.into())
        .with_actor_id(ALICE)
        .await
        .unwrap();

    let allowance_detail = vft_extension.allowance_of(ALICE, BOB).await.unwrap();
    assert!(allowance_detail.is_some());
    let (amount, _expiry) = allowance_detail.unwrap();
    assert_eq!(amount, 500.into());

    // 3. List balances with pagination
    let balances = vft_extension.balances(0, 10).await.unwrap();
    assert!(balances.iter().any(|(id, amt)| *id == BOB && *amt == 1000.into()));
}
```
