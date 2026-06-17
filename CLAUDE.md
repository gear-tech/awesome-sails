# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Awesome Sails is a workspace of production-ready services and utilities for building
decentralized applications (dApps) on the **Gear Protocol** using the **Sails framework**
(`sails-rs`). All on-chain crates are `#![no_std]` and compile to `wasm32`. Study the
[Sails documentation](https://docs.rs/sails-rs/latest/sails_rs/) before making changes.

## Commands

```sh
# Build everything (compiles services to wasm via build scripts)
cargo build --workspace

# Run all tests (gtest-based integration tests + unit tests)
cargo test --workspace

# Run a single test crate / single test
cargo test -p awesome-sails-test-app
cargo test -p awesome-sails-test-app allowance

# Lint exactly as CI does (warnings are errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check
```

Toolchain is pinned in `rust-toolchain.toml` (stable, edition 2024, targets
`wasm32-unknown-unknown` + `wasm32v1-none`). CI (`.github/workflows/ci.yml`) runs fmt,
clippy `--all-features -D warnings`, and `cargo test --workspace` — match these locally
before declaring work done.

## Architecture

### Crate layout

- `utils/` — `awesome-sails-utils`: foundational `no_std` building blocks: `error`,
  `macros` (`ensure!`, `ok_if!`), `map`, `math` (`NonZero`, `Zero`, `Max`), and `pause`.
- `crates/awesome-sails/` — the `awesome-sails` **meta-crate**. It re-exports each service
  crate behind a Cargo feature (see feature graph in its `Cargo.toml`). `default = ["all"]`.
- `crates/awesome-sails/<service>/` — individual service crates (`vft`, `vft-admin`,
  `vft-extension`, `vft-metadata`, `vft-native-exchange`, `vft-native-exchange-admin`,
  `access-control`, `msg-tracker`, plus `vft/utils`).
- `tests/<name>-test/` — each has an `app/` (a real Sails `#[program]`) and a `client/`
  (generated IDL client). Integration tests live in `app/tests/`.
- `benchmarks/` — gas/performance measurement suite.

### Service composition pattern

Services are plain structs constructed per-message, not singletons. The `#[program]`
struct owns storage in `RefCell`s and a `Pause`; each service accessor method
(`fn vft()`, `fn vft_admin()`, …) builds a fresh service borrowing that storage. See
`tests/awesome-sails-test/app/src/lib.rs` for the canonical wiring. Services compose by
construction: `vft-admin` is built from an `AccessControl` + `vft`; `vft-extension` and
`vft-native-exchange` wrap a `vft`; `vft-native-exchange-admin` wraps a `vft-admin`.
The meta-crate's feature flags enforce these dependencies (e.g. `vft-admin` pulls in
`vft` and `access-control`).

### Storage and the `State` / `StateMut` abstraction

Services are generic over storage backends implementing `State` / `StateMut`. The
common concrete backend is `PausableRef<'a, T>` (= `Pausable<&RefCell<T>, &Pause>`),
which wraps a `RefCell` and gates **all mutating** access on the global `Pause` switch —
`write()` returns `PausableError::Paused` when paused; reads still succeed. Genericity
over storage is what makes services unit-testable with mock backends (`mockall`).

### Pause mechanism

`Pause` is a `Cell<bool>` with interior mutability, owned once by the `#[program]` and
shared by reference into every `PausableRef`. Pausing the program freezes all writes
across every service at once.

### Sails-specific conventions

- Service methods are exported with `#[export(unwrap_result)]` — a `Result` return is
  unwrapped at the ABI boundary so the on-chain interface exposes the `Ok` type.
- Events are `#[event]` enums deriving `Encode, TypeInfo, ReflectHash`; emit with
  `self.emit_event(...)`.
- App crates have a `build.rs` calling `sails_rs::build_wasm()`; client crates have a
  `build.rs` that generates a typed client from the app's IDL. Integration tests drive
  the wasm program through the generated client on `gtest`.
- `access-control` uses const generics for capacity (`AccessControlState<N, M, RS, MS>`)
  with `SmallVec` for stack-first storage; the super-admin role
  (`default_admin_role()`, all-zero id) is a master key passing every `require_role` check.

## Notes

- The meta-crate's `test` feature enables test-only helpers in service crates; test apps
  depend on `awesome-sails` with `features = ["all", "test"]`.
