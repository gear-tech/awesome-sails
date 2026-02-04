use anyhow::{Context, Result};
use fs2::FileExt;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    env,
    fs::OpenOptions,
    io::{Seek, SeekFrom},
    path::PathBuf,
};

/// A persistent storage for benchmark results backed by a JSON file.
/// Handles concurrent access automatically via file locking.
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
    /// Creates a new storage interface pointing to `bench_data.json` in the crate root.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates a specific section of the benchmark data.
    ///
    /// This method is atomic: it locks the file, reads current state, updates the specific key,
    /// and writes back immediately.
    ///
    /// # Arguments
    /// * `section_key` - The unique key for the benchmark (e.g., "access_control").
    /// * `data` - The data to store (must be serializable).
    pub fn update<T: Serialize>(&self, section_key: &str, data: T) -> Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .context("Failed to open bench data file")?;

        // 1. Lock for safety
        file.lock_exclusive()
            .context("Failed to lock bench data file")?;

        // 2. Read existing
        let mut full_data: BTreeMap<String, Value> = if file.metadata()?.len() > 0 {
            serde_json::from_reader(&file).unwrap_or_default()
        } else {
            BTreeMap::new()
        };

        // 3. Modify
        let new_value = serde_json::to_value(data).context("Failed to serialize new bench data")?;
        full_data.insert(section_key.to_string(), new_value);

        // 4. Write back
        let mut file = file;
        file.set_len(0).context("Failed to truncate file")?;
        file.seek(SeekFrom::Start(0))
            .context("Failed to seek to start")?;
        serde_json::to_writer_pretty(&file, &full_data).context("Failed to write JSON")?;

        // 5. Unlock
        file.unlock().context("Failed to unlock file")?;

        Ok(())
    }
}

/// Helper utility to calculate median gas usage.
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
