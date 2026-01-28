# Awesome Sails

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Awesome Sails is a comprehensive collection of production-ready services and utilities designed for building decentralized applications (dApps) on the Gear Protocol using the Sails Framework. It provides a modular suite of libraries ranging from core token standards to access control and administrative tools. Services such as `vft-admin` and `vft-extension` are designed to seamlessly extend the core `vft` service, allowing developers to compose functionality as needed.

## Modules

The workspace is organized into the following components:

- **[Utils](utils/README.md):** Foundational utilities, math helpers, and storage abstractions.
- **[Access Control](crates/awesome-sails/access-control/README.md):** A flexible Role-Based Access Control (RBAC) system.
- **[VFT](crates/awesome-sails/vft/README.md):** Implementation of the Vara Fungible Token (VFT) standard (ERC-20 analogous).
- **[VFT Admin](crates/awesome-sails/vft-admin/README.md):** Administrative extensions for VFT (minting, burning, pausing).
- **[VFT Extension](crates/awesome-sails/vft-extension/README.md):** Extended VFT functionalities (transfer all, cleanup, enumeration).
- **[VFT Metadata](crates/awesome-sails/vft-metadata/README.md):** Metadata storage for VFTs.
- **[VFT Native Exchange](crates/awesome-sails/vft-native-exchange/README.md):** Native token to VFT exchange mechanism.
- **[VFT Native Exchange Admin](crates/awesome-sails/vft-native-exchange-admin/README.md):** Administrative tools for the native exchange service.

## Usage

You can use individual crates by adding them to your `Cargo.toml`, or use the **[Awesome Sails Pack](crates/awesome-sails/README.md)** meta-crate to access multiple services with feature flags.

```toml
[dependencies]
awesome-sails = { version = "x.y.z", features = ["all"] }
```

Refer to the individual README files linked above for detailed installation and usage instructions.
