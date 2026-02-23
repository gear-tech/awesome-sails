# Awesome Sails VFT Service

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A service that implements the Vara Fungible Token (VFT) standard, which is analogous to the ERC-20 standard on Ethereum. It provides core functionalities for token transfers, approvals, and balance queries.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the VFT service, your program needs to manage storage for allowances and balances, and then instantiate the `Vft` service.

```rust
#![no_std]

use awesome_sails_vft::{Vft, utils::{Allowances, Balances}};
use awesome_sails_utils::{pause::{PausableRef, Pause}, storage::StorageRefCell};
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
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vft(&self) -> Vft<'_> {
        Vft::new(self.allowances(), self.balances())
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
};
use awesome_sails_utils::assert_ok;
use common::{ALICE, BOB, deploy_with_data};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_vft() {
    // deploy_with_data is a helper from the common module
    // Deploy with ALICE having 2000 tokens
    let (program, _env, _pid) = deploy_with_data(Default::default(), vec![(ALICE, 2000.into())], 1).await;
    let mut vft = program.vft();

    // 1. Transfer from ALICE to BOB
    let res = vft.transfer(BOB, 1000.into())
        .with_actor_id(ALICE)
        .await;
    assert_ok!(res, true);

    // Verify balance
    let balance_alice = vft.balance_of(ALICE).await.unwrap();
    assert_eq!(balance_alice, 1000.into());

    // 2. Approve BOB to spend 500 tokens
    let res = vft.approve(BOB, 500.into())
        .with_actor_id(ALICE)
        .await;
    assert_ok!(res, true);

    // Check allowance
    let remaining = vft.allowance(ALICE, BOB).await.unwrap();
    assert_eq!(remaining, 500.into());

    // 3. Transfer from ALICE to BOB using allowance (called by BOB as spender)
    let res = vft.transfer_from(ALICE, BOB, 200.into())
        .with_actor_id(BOB)
        .await;
    assert_ok!(res, true);

    // Verify final balance
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 1200.into());

    // 4. Verify total supply
    let supply = vft.total_supply().await.unwrap();
    assert_eq!(supply, 2000.into());
}
```
