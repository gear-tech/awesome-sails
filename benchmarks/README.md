# Awesome Sails Benchmarks

> **Note:** Built for the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A benchmarking suite for Sails programs. This crate provides tools to measure gas consumption and a CLI utility to analyze performance regressions.

## Features

- `gtest` (**default**): Enables built-in support for the Gear `gtest` framework and provides the `MeasureGas` implementation for `System`.
- `ascii-table`: Enables ASCII table output for terminal reports.
- `cli`: Enables the `bench-analyzer` CLI utility.

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
# Standard usage (with gtest support)
awesome-sails-benchmarks = { version = "x.y.z", features = ["ascii-table"] }

# If you have version conflicts with gtest, disable default features:
awesome-sails-benchmarks = { version = "x.y.z", default-features = false }
```

## Usage

### 1. Gas Measurement

Use the `MeasureGas` trait to capture gas during tests. Benchmarks **MUST** be run in `--release` mode.

The `MeasureGas` trait is always available, but the implementation for the Gear `gtest::System` requires the `gtest` feature (enabled by default).

```rust
use awesome_sails_benchmarks::{MeasureGas, BenchStorage, ToBenchmarkMap};
use std::collections::BTreeMap;

struct MyServiceResults {
    op_gas: BTreeMap<u32, u64>,
}

impl ToBenchmarkMap for MyServiceResults {
    fn to_benchmark_map(self) -> BTreeMap<String, u64> {
        let mut map = BTreeMap::new();
        for (count, gas) in self.op_gas {
            map.insert(format!("my_op.{}", count), gas);
        }
        map
    }
}

#[tokio::test]
#[ignore]
async fn bench_my_service() {
    let env = create_env();
    // System from gtest implements MeasureGas if the 'gtest' feature is enabled.
    let system = env.system();
    let mut service = deploy_program(&env).await;

    let mut op_gas = BTreeMap::new();
    for &count in &[0, 100, 1000] {
        let gas = system.measure_gas(|| {
            service.operation(count).send_one_way().unwrap()
        });
        op_gas.insert(count, gas);
    }

    BenchStorage::from_default_path()
        .update("my_service", MyServiceResults { op_gas })
        .expect("Failed to save benchmarks");
}
```

### 2. Analysis

The `bench-analyzer` utility compares generated results with a baseline.

#### Installation

Install the tool globally via cargo:

```bash
cargo install awesome-sails-benchmarks --features cli
```

#### Comparison Modes

- **Regression Check (Default)**: Fails if any metric _increases_ beyond the `--threshold`.
- **Strict Mode (`--strict`)**: Fails if any metric _deviates_ (increases OR decreases) by more than the threshold. Useful for ensuring benchmark stability.

#### Usage

```bash
bench-analyzer \
  --current benchmarks/bench_data.json \
  --other benchmarks/baseline.json \
  --threshold 5.0 \
  --output report.md
```

## CLI Reference

```text
A CLI tool to compare two benchmark JSON files and detect performance regressions.

Usage: bench-analyzer [OPTIONS] --current <CURRENT> --other <OTHER>

Options:
      --current <CURRENT>      Path to the current benchmark results
      --other <OTHER>          Path to the baseline benchmark results
      --output <OUTPUT>        Path to save the comparison report in Markdown format
      --threshold <THRESHOLD>  Custom regression threshold percentage (e.g. 5.0)
      --strict                 Enable strict mode: fail if ANY metric deviates from baseline
  -h, --help                   Print help
```
