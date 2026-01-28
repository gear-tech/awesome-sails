# Awesome Sails VFT Native Exchange Admin

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Administrative extensions for the VFT Native Exchange service. This service handles recovery from failed value transfers (via `handle_reply`) and allows administrators to force a burn of VFT tokens, returning the native value to the user.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft-native-exchange-admin = "x.y.z"
```

## Usage

### On-Chain: Service Integration

To use the Native Exchange Admin service, instantiate it with the VFT Admin service. Crucially, you must invoke `handle_reply` in your program's `handle_reply` entry point to ensure failed transfers are processed.

```rust
#![no_std]

use awesome_sails_vft_native_exchange_admin::VftNativeExchangeAdmin;
use awesome_sails_vft_admin::VftAdmin;
use awesome_sails_vft::Vft;
use awesome_sails_access_control::{AccessControl, RolesStorage};
use awesome_sails_vft::utils::{Allowances, Balances};
use awesome_sails_utils::{pause::{PausableRef, Pause}, storage::StorageRefCell};
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

    pub fn vft_admin(&self) -> VftAdmin<'_> {
        VftAdmin::new(
            self.access_control(),
            self.allowances(),
            self.balances(),
            &self.pause,
            self.vft(),
        )
    }

    pub fn vft_native_exchange_admin(&self) -> VftNativeExchangeAdmin<'_> {
        VftNativeExchangeAdmin::new(self.vft_admin())
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_reply(&mut self) {
        self.vft_native_exchange_admin().handle_reply();
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following example demonstrates how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
mod common; // Helpers defined in tests/common/mod.rs

use awesome_sails::vft_admin::BURNER_ROLE;
use awesome_sails_test_client::{
    access_control::AccessControl,
    AwesomeSailsTestClient,
    vft::Vft,
    vft_native_exchange::VftNativeExchange,
    vft_native_exchange_admin::VftNativeExchangeAdmin,
};
use common::{ALICE, BOB, deploy_with_data};
use sails_rs::prelude::*;

#[tokio::test]
async fn test_vft_native_exchange_admin() {
    // deploy_with_data is a helper from the common module
    let (program, _env, _pid) = deploy_with_data(Default::default(), Default::default(), 1).await;
    
    let mut exchange_admin = program.vft_native_exchange_admin();
    let mut exchange = program.vft_native_exchange();
    let vft = program.vft();
    let mut access_control = program.access_control();

    // 1. Bob mints some VFT first
    exchange
        .mint()
        .with_actor_id(BOB)
        .with_value(1000)
        .await
        .unwrap();

    // 2. Grant BURNER_ROLE to ALICE
    access_control
        .grant_role(BURNER_ROLE, ALICE)
        .with_actor_id(ALICE)
        .await
        .unwrap();

    // 3. Admin (ALICE) burns tokens from BOB
    exchange_admin
        .burn_from(BOB, 400.into())
        .with_actor_id(ALICE)
        .await
        .expect("Burn from failed");

    // Verify VFT balance
    let balance_bob = vft.balance_of(BOB).await.unwrap();
    assert_eq!(balance_bob, 600.into());
}
```
