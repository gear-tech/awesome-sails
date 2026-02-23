# Awesome Sails Storage

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

Defines storage abstractions for the `awesome-sails` workspace. This crate provides traits and types for abstracting data access, allowing code to be written generically over different storage backends (e.g., in-memory `RefCell`, persistent storage).

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-storage = "x.y.z"
```

## Usage

The `storage` module provides traits to abstract over different storage backends (e.g., `RefCell` for testing or persistent storage).

```rust
use awesome_sails_storage::{InfallibleStorage, InfallibleStorageMut, StorageRefCell};
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

### Core Traits

*   **`Storage` / `StorageMut`**: Standard traits representing fallible storage mechanisms. Implementors provide access to a stored item, allowing potential failures (e.g., database access errors).
*   **`InfallibleStorage` / `InfallibleStorageMut`**: Specialized traits for storage mechanisms that cannot fail on access. These are particularly useful for in-memory backends like `RefCell` during testing or for simple state management.
*   **`StorageRefCell`**: A convenient wrapper around `RefCell<T>` that implements the infallible storage traits, allowing it to be used as a backend for services that require a storage abstraction.
