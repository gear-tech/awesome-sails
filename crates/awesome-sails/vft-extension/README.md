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
use awesome_sails_utils::{pause::{PausableRef, Pause}, storage::StorageRefCell};
use core::cell::RefCell;
use sails_rs::prelude::*;

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

    pub fn vft(&self) -> Vft<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        Vft::new(self.allowances(), self.balances())
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vft_extension(&self) -> VftExtension<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftExtension::new(self.allowances(), self.balances(), self.vft())
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_transfer_all() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, pid) = deploy_program().await;
    let mut vft_extension_service = program.vft_extension();
    let mut vft_service = program.vft();

    // Alice transfers all her tokens to Bob
    let res = vft_extension_service
        .transfer_all(BOB)
        .with_actor_id(ALICE)
        .await;

    assert!(res.is_ok());

    // Alice's balance should be 0
    let balance_alice = vft_service.balance_of(ALICE).await.unwrap();
    assert_eq!(balance_alice, 0.into());
}
```
