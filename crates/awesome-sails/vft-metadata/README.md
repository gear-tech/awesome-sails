# Awesome Sails VFT Metadata

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A service that provides access to the metadata (name, symbol, decimals) of a Volatile Fungible Token (VFT), following the ERC-20 metadata standard.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-vft-metadata = "x.y.z"
```

## Usage

### On-Chain: Service Integration

Include the metadata struct in your program and expose the service.

```rust
#![no_std]

use awesome_sails_vft_metadata::{VftMetadata, Metadata};
use sails_rs::prelude::*;

#[derive(Default)]
pub struct Program {
    metadata: Metadata,
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self {
            metadata: Metadata::default(),
        }
    }

    pub fn vft_metadata(&self) -> VftMetadata<&Metadata> {
        VftMetadata::new(&self.metadata)
    }
}
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

```rust
#[tokio::test]
async fn test_metadata() {
    // Note: deploy_program() is a helper function typically defined in tests/common/mod.rs
    let (program, _env, _pid) = deploy_program().await;
    let service = program.vft_metadata();

    let name = service.name().await.unwrap();
    assert_eq!(name, "Unit");

    let symbol = service.symbol().await.unwrap();
    assert_eq!(symbol, "UNIT");

    let decimals = service.decimals().await.unwrap();
    assert_eq!(decimals, 12);
}
```
