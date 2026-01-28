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

The `math` module provides traits for checked arithmetic, custom fixed-size integer types (`LeBytes`), and a `NonZero` wrapper.

#### Checked Arithmetic

```rust
use awesome_sails_utils::math::{CheckedMath, OverflowError, UnderflowError};

fn example_checked_math() {
    let max = u8::MAX;
    let one = 1u8;

    // Standard checked addition (returns Option)
    assert_eq!(max.checked_add(one), None);

    // Error-based checked addition (returns Result)
    assert_eq!(max.checked_add_err(one), Err(OverflowError));

    // Valid operation
    assert_eq!(10u8.checked_add_err(5u8), Ok(15u8));
}
```

#### Custom Fixed-Size Integers (`LeBytes`)

`LeBytes<N>` provides a way to define integers with a specific number of bytes, which is useful for compact SCALE encoding.

```rust
use awesome_sails_utils::math::LeBytes;
use parity_scale_codec::{Encode, Decode};

// Define a 72-bit (9-byte) unsigned integer
type Uint72 = LeBytes<9>;

fn example_le_bytes() {
    let val = Uint72::try_from(1000u128).unwrap();
    let encoded = val.encode();
    
    // Encoded length is exactly 9 bytes
    assert_eq!(encoded.len(), 9);

    let decoded = Uint72::decode(&mut &encoded[..]).unwrap();
    assert_eq!(val, decoded);
}
```

#### Non-Zero Wrapper

The `NonZero<T>` wrapper ensures that the contained value is never zero, providing safe arithmetic and casting.

```rust
use awesome_sails_utils::math::{NonZero, LeBytes};

fn example_nonzero() {
    let one = NonZero::try_new(1u64).unwrap();
    let two = one.try_add(one).expect("Addition failed");

    assert_eq!(two.into_inner(), 2);

    // Casting between types
    let as_u128: u128 = one.try_cast().unwrap();
    assert_eq!(as_u128, 1u128);
}
```

### Storage Abstractions

The `storage` module provides traits to abstract over different storage backends (e.g., `RefCell` for testing or persistent storage).

```rust
use awesome_sails_utils::storage::{InfallibleStorage, InfallibleStorageMut, StorageRefCell};
use core::cell::RefCell;

fn example_storage() {
    let storage_inner = RefCell::new(42u32);
    let mut storage = StorageRefCell::new(&storage_inner);

    // InfallibleStorageMut provides replace, take, etc.
    let old_value = InfallibleStorageMut::replace(&mut storage, 10);
    assert_eq!(old_value, 42);

    // Access the value using InfallibleStorage
    let current = InfallibleStorage::get(&storage);
    assert_eq!(*current, 10);
}
```
