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
use core::cell::RefCell;
use sails_rs::prelude::*;

#[derive(Default)]
pub struct Program {
    allowances: RefCell<Allowances>,
    balances: RefCell<Balances>,
    pause: Pause, // Optional: for pausability
}

impl Program {
    // Helper to create pausable references (or just use StorageRefCell directly)
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

    pub fn vft(&self) -> Vft<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        Vft::new(self.allowances(), self.balances())
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_vft_transfer() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, pid) = deploy_program().await;
    let mut vft_service = program.vft();

    // Alice transfers 100 tokens to Bob
    let res = vft_service
        .transfer(BOB, 100.into())
        .with_actor_id(ALICE)
        .await;

    assert!(res.is_ok());

    // Check Bob's balance
    let balance = vft_service.balance_of(BOB).await.unwrap();
    assert_eq!(balance, 100.into());
}
```
