# Awesome Sails Utils

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Foundational utilities for the `awesome-sails` workspace. This crate provides shared functionality, including error handling, macros, data structures, mathematical operations, pausable functionality, and storage helpers used across various services in the ecosystem.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-utils = "x.y.z"
```

## Usage

This crate is a library of utilities and does not provide a standalone service. Below are examples of how to use some of the provided modules.

### Math Utilities

The `math` module provides traits for checked arithmetic and custom fixed-size integer types.

```rust
use awesome_sails_utils::math::{CheckedMath, OverflowError, UnderflowError};

fn example_checked_math() {
    let max = u8::MAX;
    let one = 1u8;

    // Checked addition returning an Option
    assert_eq!(max.checked_add(one), None);

    // Checked addition returning a Result
    assert_eq!(max.checked_add_err(one), Err(OverflowError));

    // Checked subtraction returning a Result
    let min = u8::MIN;
    assert_eq!(min.checked_sub_err(one), Err(UnderflowError));
}
```

### Storage Abstractions

The `storage` module provides traits to abstract over different storage backends (e.g., `RefCell` for testing or persistent storage).

```rust
use awesome_sails_utils::storage::StorageMut;
use core::cell::RefCell;

fn example_storage(mut storage: RefCell<u32>) {
    // RefCell implements InfallibleStorageMut, which implements StorageMut
    let old_value = storage.replace(10).unwrap();

    // Access the value
    let current = storage.get_mut().unwrap();
    assert_eq!(*current, 10);
}
```
