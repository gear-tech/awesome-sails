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

    pub fn vft_native_exchange(&self) -> VftNativeExchange<'_, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftNativeExchange::new(self.balances(), self.vft())
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
    vft_native_exchange::VftNativeExchange,
};
use common::{BOB, deploy_with_data};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_vft_native_exchange() {
    // deploy_with_data is a helper from the common module
    let (program, _env, _pid) = deploy_with_data(Default::default(), Default::default(), 1).await;
    
    let mut exchange = program.vft_native_exchange();
    let vft = program.vft();

    // 1. Bob sends 1000 native tokens to mint VFT
    exchange
        .mint()
        .with_actor_id(BOB)
        .with_value(1000)
        .await
        .expect("Mint failed");

    // Verify VFT balance
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 1000.into());

    // 2. Bob burns 400 VFT tokens to get back native value
    exchange
        .burn(400.into())
        .with_actor_id(BOB)
        .await
        .expect("Burn failed");

    // Verify VFT balance after burn
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 600.into());

    // 3. Bob burns all remaining VFT tokens
    exchange
        .burn_all()
        .with_actor_id(BOB)
        .await
        .expect("Burn all failed");

    // Verify VFT balance is 0
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 0.into());
}
```
