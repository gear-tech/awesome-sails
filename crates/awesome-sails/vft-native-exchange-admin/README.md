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
    // ... helpers for storage references ...
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

    pub fn vft_admin(&self) -> VftAdmin<'_, StorageRefCell<'_, RolesStorage>, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftAdmin::new(self.access_control(), self.allowances(), self.balances(), &self.pause, self.vft())
    }

    pub fn vft_native_exchange_admin(&self) -> VftNativeExchangeAdmin<'_, StorageRefCell<'_, RolesStorage>, PausableRef<'_, Allowances>, PausableRef<'_, Balances>> {
        VftNativeExchangeAdmin::new(self.vft_admin())
    }
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    // Important: Hook up handle_reply
    pub fn handle_reply(&mut self) {
        self.vft_native_exchange_admin().handle_reply();
    }

    // ... expose services ...
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_burn_from() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, pid) = deploy_program().await;
    let mut exchange_admin_service = program.vft_native_exchange_admin();

    // Grant BURNER_ROLE to Admin first...

    // Admin burns 50 tokens from Bob, sending native value to Bob
    let res = exchange_admin_service
        .burn_from(BOB, 50.into())
        .with_actor_id(ALICE)
        .await;

    assert!(res.is_ok());
}
```
