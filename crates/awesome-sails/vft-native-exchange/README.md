# Awesome Sails VFT Native Exchange

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A service that facilitates the exchange between native tokens (Value) and VFT tokens. Sending native value to `mint` creates VFT tokens, while calling `burn` destroys VFT tokens and returns the native value.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft-native-exchange = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the Native Exchange service, instantiate it with references to the balance storage and the base VFT service.

```rust
#![no_std]

use awesome_sails_vft_native_exchange::VftNativeExchange;
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

    pub fn vft_native_exchange(&self) -> VftNativeExchange<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftNativeExchange::new(self.balances(), self.vft())
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_native_exchange() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, pid) = deploy_program().await;
    let mut exchange_service = program.vft_native_exchange();
    let mut vft_service = program.vft();

    // Bob sends 100 native tokens to mint VFT
    exchange_service
        .mint()
        .with_actor_id(BOB)
        .with_value(100.into())
        .await
        .expect("Mint failed");

    // Bob should have 100 VFT tokens
    let balance = vft_service.balance_of(BOB).await.unwrap();
    assert_eq!(balance, 100.into());

    // Bob burns 50 VFT tokens to get back native value
    exchange_service
        .burn(50.into())
        .with_actor_id(BOB)
        .await
        .expect("Burn failed");

    // Bob burns all remaining VFT tokens
    exchange_service
        .burn_all()
        .with_actor_id(BOB)
        .await
        .expect("Burn all failed");
}
```
