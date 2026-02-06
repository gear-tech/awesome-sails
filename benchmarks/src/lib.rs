use anyhow::{Context, Result};
use fs2::FileExt;
use sails_rs::gtest::System;
use sails_rs::prelude::*;
use std::{
    collections::BTreeMap,
    env,
    fs::OpenOptions,
    io::{Seek, SeekFrom},
    path::PathBuf,
};

/// Trait for types that can be converted into a flat benchmark map.
pub trait ToBenchmarkMap {
    fn to_benchmark_map(self) -> BTreeMap<String, u64>;
}

impl ToBenchmarkMap for BTreeMap<String, u64> {
    fn to_benchmark_map(self) -> BTreeMap<String, u64> {
        self
    }
}

pub struct BenchStorage {
    path: PathBuf,
}

impl Default for BenchStorage {
    fn default() -> Self {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        let path = PathBuf::from(manifest_dir).join("bench_data.json");
        Self { path }
    }
}

impl BenchStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the storage using the ToBenchmarkMap trait.
    pub fn update<T: ToBenchmarkMap>(&self, section_key: &str, results: T) -> Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .context("Failed to open bench data file")?;

        file.lock_exclusive().context("Failed to lock file")?;

        let mut full_data: BTreeMap<String, BTreeMap<String, u64>> = if file.metadata()?.len() > 0 {
            serde_json::from_reader(&file).unwrap_or_default()
        } else {
            BTreeMap::new()
        };

        full_data.insert(section_key.to_string(), results.to_benchmark_map());

        let mut file = file;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        serde_json::to_writer_pretty(&file, &full_data)?;

        file.unlock()?;
        Ok(())
    }
}

pub fn measure_gas<F>(system: &System, f: F) -> u64
where
    F: FnOnce() -> MessageId,
{
    let mid = f();
    let res = system.run_next_block();
    *res.gas_burned.get(&mid).expect("Gas not recorded")
}

pub fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    if values.is_empty() {
        return 0;
    }
    let len = values.len();
    if len.is_multiple_of(2) {
        (values[len / 2 - 1] + values[len / 2]) / 2
    } else {
        values[len / 2]
    }
}
