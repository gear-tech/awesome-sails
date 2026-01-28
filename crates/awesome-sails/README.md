# Awesome Sails Pack

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A meta-crate that bundles various services and utilities for building dApps on the Gear Protocol using the Sails Framework. It allows you to selectively enable features to include only the components you need for your project.

## Installation

Add the following to your `Cargo.toml`. You can enable specific features based on your requirements.

```toml
[dependencies]
awesome-sails = { version = "x.y.z", features = ["all"] }
```

## Available Services

| Service Name              | Crate Name                                | Feature Flag                | Description                                              |
| ------------------------- | ----------------------------------------- | --------------------------- | -------------------------------------------------------- |
| Access Control            | `awesome-sails-access-control`            | `access-control`            | Role-Based Access Control (RBAC) service.                |
| VFT                       | `awesome-sails-vft`                       | `vft`                       | Core Vara Fungible Token implementation.                 |
| VFT Admin                 | `awesome-sails-vft-admin`                 | `vft-admin`                 | Administrative functionality (mint, burn, pause).        |
| VFT Extension             | `awesome-sails-vft-extension`             | `vft-extension`             | Extended features (transfer all, cleanup, enumeration).  |
| VFT Metadata              | `awesome-sails-vft-metadata`              | `vft-metadata`              | Metadata service (name, symbol, decimals).               |
| VFT Native Exchange       | `awesome-sails-vft-native-exchange`       | `vft-native-exchange`       | Native token to VFT exchange service.                    |
| VFT Native Exchange Admin | `awesome-sails-vft-native-exchange-admin` | `vft-native-exchange-admin` | Administrative recovery for Native Exchange.             |
| VFT Utils                 | `awesome-sails-vft-utils`                 | `vft-utils`                 | Shared utilities for VFT storage (Allowances, Balances). |

## Usage

When using this meta-crate, you can import services directly from the `awesome_sails` namespace.

### On-Chain: Service Integration

```rust
#![no_std]

// Import services from the pack
use awesome_sails::vft::Vft;
use awesome_sails::access_control::AccessControl;
// ... other imports

// Integration follows the same patterns as using individual crates.
```

### Testing (Off-Chain Interaction via Gtest)

The following examples demonstrate how to verify service logic using the gtest framework.

> **Note:** For more details on testing with `gtest`, refer to the [gtest documentation](https://docs.rs/gtest/latest/gtest/).

Tests can also utilize the re-exported modules.

```rust
use awesome_sails::vft::Vft;
// ...
```
