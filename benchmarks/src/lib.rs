// This file is part of Gear.

// Copyright (C) 2026 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use fs2::FileExt;
use std::{
    collections::BTreeMap,
    env, fmt,
    fs::OpenOptions,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};

use gprimitives::MessageId;
use gtest::System;

const RELEASE_MODE_ERROR: &str =
    "Benchmarks MUST be run in --release mode to get accurate gas measurements.";

/// Defines an interface for converting data structures into a standardized benchmark map.
pub trait ToBenchmarkMap {
    /// Converts the implementing type into a benchmark map.
    fn to_benchmark_map(self) -> BTreeMap<String, u64>;
}

/// Defines a trait for measuring gas burned during message execution.
pub trait MeasureGas {
    /// Measures the gas burned for a specific message execution.
    fn measure_gas<F>(&self, f: F) -> u64
    where
        F: FnOnce() -> MessageId;

    /// Measures the total gas burned in the block after executing the message.
    /// Useful for cross-program calls.
    fn measure_total_gas<F>(&self, f: F) -> u64
    where
        F: FnOnce() -> MessageId;
}

/// A comprehensive report containing the results of a benchmark comparison.
#[derive(Debug)]
pub struct BenchReport {
    pub diffs: Vec<MetricDiff>,
    pub config: ComparisonConfig,
    pub title: String,
}

/// A builder for constructing a `BenchReport`.
#[derive(Debug)]
pub struct ReportBuilder {
    diffs: Vec<MetricDiff>,
    config: ComparisonConfig,
    title: String,
}

/// Manages the persistent storage of benchmark data.
#[derive(Debug)]
pub struct BenchStorage {
    path: PathBuf,
}

/// A wrapper for `BenchReport` that implements `Display` for Markdown formatting.
#[derive(Debug, Clone, Copy)]
pub struct BenchReportMarkdown<'a>(pub &'a BenchReport);

/// A wrapper for `BenchReport` that implements `Display` for ASCII table formatting.
#[cfg(feature = "ascii-table")]
#[derive(Debug, Clone, Copy)]
pub struct BenchReportAscii<'a>(pub &'a BenchReport);

/// Represents the difference between two specific metric measurements.
#[derive(Debug, Clone)]
pub struct MetricDiff {
    /// The hierarchical path or name of the metric.
    pub path: String,
    /// The current value of the metric.
    pub current: u64,
    /// The baseline value of the metric, if available.
    pub baseline: Option<u64>,
    /// The categorical status of the change.
    pub status: MetricStatus,
    /// The percentage difference, if a baseline exists.
    pub percent: Option<f64>,
}

/// Configuration parameters for benchmark comparisons.
#[derive(Debug, Clone, Copy)]
pub struct ComparisonConfig {
    /// Threshold percentage for a strong improvement.
    pub improved_strong: f64,
    /// Threshold percentage for a minor improvement.
    pub improved_minor: f64,
    /// Threshold percentage for a minor regression.
    pub regressed_minor: f64,
    /// Threshold percentage for a strong regression.
    pub regressed_strong: f64,
    /// Optional character used as a thousand separator in numeric output.
    pub thousand_separator: Option<char>,
}

/// Represents the status of a metric comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricStatus {
    /// Indicates a significant improvement in the metric.
    ImprovedStrong,
    /// Indicates a minor improvement in the metric.
    ImprovedMinor,
    /// Indicates no significant change in the metric.
    Neutral,
    /// Indicates a minor regression in the metric.
    RegressedMinor,
    /// Indicates a significant regression in the metric.
    RegressedStrong,
    /// Indicates the metric is new and was not present in the baseline.
    New,
    /// Indicates the metric was present in the baseline but is missing now.
    Removed,
}

impl ToBenchmarkMap for BTreeMap<String, u64> {
    fn to_benchmark_map(self) -> BTreeMap<String, u64> {
        self
    }
}

impl MeasureGas for System {
    fn measure_gas<F>(&self, f: F) -> u64
    where
        F: FnOnce() -> MessageId,
    {
        if cfg!(debug_assertions) {
            core::panic!("{}", RELEASE_MODE_ERROR);
        }
        let mid = f();
        let res = self.run_next_block();
        if !res.succeed.contains(&mid) {
            core::panic!("Message {:?} failed to execute", mid);
        }
        res.gas_burned.get(&mid).copied().expect("Gas not recorded")
    }

    fn measure_total_gas<F>(&self, f: F) -> u64
    where
        F: FnOnce() -> MessageId,
    {
        if cfg!(debug_assertions) {
            core::panic!("{}", RELEASE_MODE_ERROR);
        }
        let mid = f();
        let res = self.run_next_block();
        if !res.failed.is_empty() {
            core::panic!("One or more messages failed in the block: {:?}", res.failed);
        }
        if !res.succeed.contains(&mid) {
            core::panic!("Trigger message {:?} not found in succeed list", mid);
        }
        res.gas_burned.values().sum()
    }
}

impl BenchReport {
    /// Returns a Markdown representation of the report.
    pub fn as_markdown(&self) -> BenchReportMarkdown<'_> {
        BenchReportMarkdown(self)
    }

    /// Returns an ASCII table representation of the report.
    #[cfg(feature = "ascii-table")]
    pub fn as_ascii(&self) -> BenchReportAscii<'_> {
        BenchReportAscii(self)
    }

    /// Returns true if any metric deviates from baseline more than the threshold (absolute value).
    /// Used for self-check to detect any changes.
    pub fn has_any_deviation(&self, threshold: f64) -> bool {
        self.diffs
            .iter()
            .any(|d| d.percent.is_some_and(|p| p.abs() > threshold))
    }

    /// Checks if the report contains any significant regressions.
    ///
    /// Returns true if any metric has a status of `RegressedStrong`.
    pub fn has_significant_regression(&self) -> bool {
        self.diffs
            .iter()
            .any(|d| d.status == MetricStatus::RegressedStrong)
    }

    fn format_number(&self, n: u64) -> String {
        let s = n.to_string();
        if let Some(sep) = self.config.thousand_separator {
            let mut res = String::new();
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    res.push(sep);
                }
                res.push(c);
            }
            res.chars().rev().collect()
        } else {
            s
        }
    }

    fn format_diff(&self, n: i128) -> String {
        if n == 0 {
            return "0".into();
        }
        let prefix = if n > 0 { "+" } else { "-" };
        format!("{}{}", prefix, self.format_number(n.unsigned_abs() as u64))
    }
}

impl ReportBuilder {
    /// Creates a new `ReportBuilder` with a list of metric differences.
    ///
    /// Initializes with default configuration and a default title.
    pub fn new(diffs: Vec<MetricDiff>) -> Self {
        Self {
            diffs,
            config: ComparisonConfig::default(),
            title: "Benchmark Comparison".to_string(),
        }
    }

    /// Sets the comparison configuration for the report.
    pub fn with_config(mut self, config: ComparisonConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the title of the report.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Consumes the builder and produces a `BenchReport`.
    pub fn build(self) -> BenchReport {
        BenchReport {
            diffs: self.diffs,
            config: self.config,
            title: self.title,
        }
    }
}

impl BenchStorage {
    /// Initializes storage using the default path relative to the manifest directory.
    pub fn from_default_path() -> Self {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        Self {
            path: PathBuf::from(manifest_dir).join("bench_data.json"),
        }
    }

    /// Initializes storage using a specified path.
    pub fn at_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Loads the benchmark data from the file system.
    ///
    /// Returns the data map or an empty map if the file does not exist.
    pub fn load(&self) -> Result<BTreeMap<String, BTreeMap<String, u64>>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let file = OpenOptions::new().read(true).open(&self.path)?;
        Ok(serde_json::from_reader(file).unwrap_or_default())
    }

    /// Updates the benchmark data with new results for a specific section.
    ///
    /// Performs thread-safe file locking to ensure data integrity during write operations.
    pub fn update<T: ToBenchmarkMap>(&self, section: &str, results: T) -> Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;

        file.lock_exclusive()?;
        let mut data = self.load()?;
        data.insert(section.to_string(), results.to_benchmark_map());

        let mut file = file;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        serde_json::to_writer_pretty(&file, &data)?;
        file.unlock()?;
        Ok(())
    }

    /// Compares the current storage data with another storage instance.
    ///
    /// Calculates the differences for matching metrics and identifies new or removed metrics
    pub fn compare(&self, other: &Self, config: &ComparisonConfig) -> Result<Vec<MetricDiff>> {
        let current = self.load()?;
        let baseline = other.load()?;
        let mut results = Vec::new();

        for (section, metrics) in current {
            for (name, cur_val) in metrics {
                let path = format!("{}.{}", section, name);
                let b_val = baseline.get(&section).and_then(|m| m.get(&name)).copied();

                let (status, percent) = if let Some(oth) = b_val {
                    let p = (cur_val as f64 - oth as f64) / oth as f64 * 100.0;
                    (MetricStatus::from_change(p, config), Some(p))
                } else {
                    (MetricStatus::New, None)
                };

                results.push(MetricDiff {
                    path,
                    current: cur_val,
                    baseline: b_val,
                    status,
                    percent,
                });
            }
        }
        Ok(results)
    }
}

impl fmt::Display for BenchReportMarkdown<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report = self.0;
        write!(f, "### 🔬 {}\n\n", report.title)?;
        writeln!(f, "| Metric | Current | Baseline | Change | % | Status |")?;
        writeln!(f, "| :--- | ---: | ---: | ---: | ---: | :--- |")?;

        for d in &report.diffs {
            let (change, p_str) = if let Some(oth) = d.baseline {
                (
                    report.format_diff(d.current as i128 - oth as i128),
                    format!("{:.2}%", d.percent.unwrap_or(0.0)),
                )
            } else {
                ("-".into(), "-".into())
            };
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} |",
                d.path,
                report.format_number(d.current),
                d.baseline
                    .map(|v| report.format_number(v))
                    .unwrap_or_else(|| "-".into()),
                change,
                p_str,
                d.status.as_emoji()
            )?;
        }

        writeln!(f, "\n#### Legend")?;
        writeln!(
            f,
            "- {} Significant improvement (>{:.0}% reduction)",
            MetricStatus::ImprovedStrong.as_emoji(),
            report.config.improved_strong.abs()
        )?;
        writeln!(
            f,
            "- {} Minor improvement (<{:.0}% reduction)",
            MetricStatus::ImprovedMinor.as_emoji(),
            report.config.improved_minor.abs()
        )?;
        writeln!(
            f,
            "- {} No significant change",
            MetricStatus::Neutral.as_emoji()
        )?;
        writeln!(
            f,
            "- {} Minor regression (>{:.0}% increase)",
            MetricStatus::RegressedMinor.as_emoji(),
            report.config.regressed_minor
        )?;
        writeln!(
            f,
            "- {} Significant regression (>{:.0}% increase)",
            MetricStatus::RegressedStrong.as_emoji(),
            report.config.regressed_strong
        )?;
        writeln!(
            f,
            "- {} New metric (not present in baseline)",
            MetricStatus::New.as_emoji()
        )?;
        writeln!(
            f,
            "- {} Removed metric (not present in current)",
            MetricStatus::Removed.as_emoji()
        )
    }
}

#[cfg(feature = "ascii-table")]
impl fmt::Display for BenchReportAscii<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use cli_table::{Cell, Style, Table, format::Justify};

        let report = self.0;
        let mut table_rows = Vec::new();
        for d in &report.diffs {
            let (change, p_str) = if let Some(oth) = d.baseline {
                (
                    report.format_diff(d.current as i128 - oth as i128),
                    format!("{:.2}%", d.percent.unwrap_or(0.0)),
                )
            } else {
                ("-".into(), "-".into())
            };

            table_rows.push(vec![
                d.path.clone().cell(),
                report
                    .format_number(d.current)
                    .cell()
                    .justify(Justify::Right),
                d.baseline
                    .map(|v| report.format_number(v))
                    .unwrap_or_else(|| "-".into())
                    .cell()
                    .justify(Justify::Right),
                change.cell().justify(Justify::Right),
                p_str.cell().justify(Justify::Right),
                format!("{} {}", d.status.as_emoji(), d.status.as_text()).cell(),
            ]);
        }

        let table = table_rows
            .table()
            .title(vec![
                "Metric".cell().bold(true),
                "Current".cell().bold(true),
                "Baseline".cell().bold(true),
                "Change".cell().bold(true),
                "%".cell().bold(true),
                "Status".cell().bold(true),
            ])
            .bold(true);

        writeln!(f, "\n## 🔬 {}\n", report.title)?;
        write!(f, "{}", table.display().map_err(|_| fmt::Error)?)
    }
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            improved_strong: -5.0,
            improved_minor: -1.0,
            regressed_minor: 1.0,
            regressed_strong: 5.0,
            thousand_separator: Some('_'),
        }
    }
}

impl MetricStatus {
    /// Determines the `MetricStatus` based on a percentage change and configuration.
    pub fn from_change(percent: f64, config: &ComparisonConfig) -> Self {
        if percent <= config.improved_strong {
            Self::ImprovedStrong
        } else if percent <= config.improved_minor {
            Self::ImprovedMinor
        } else if percent >= config.regressed_strong {
            Self::RegressedStrong
        } else if percent >= config.regressed_minor {
            Self::RegressedMinor
        } else {
            Self::Neutral
        }
    }

    /// Returns the emoji representation of the status.
    pub fn as_emoji(&self) -> &'static str {
        match self {
            Self::ImprovedStrong => "🚀",
            Self::ImprovedMinor => "👍",
            Self::Neutral => "✅",
            Self::RegressedMinor => "⚠️",
            Self::RegressedStrong => "❌",
            Self::New => "✨",
            Self::Removed => "🗑️",
        }
    }

    /// Returns the textual representation of the status.
    pub fn as_text(&self) -> &'static str {
        match self {
            Self::ImprovedStrong | Self::ImprovedMinor => "Improved",
            Self::Neutral => "Neutral",
            Self::RegressedMinor | Self::RegressedStrong => "Regressed",
            Self::New => "New",
            Self::Removed => "Removed",
        }
    }
}

/// Calculates the median value of a list of unsigned integers.
pub fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    let len = values.len();
    if len == 0 {
        return 0;
    }

    let mid = len / 2;
    if len.is_multiple_of(2) {
        let low = values[mid - 1];
        let high = values[mid];
        low + (high - low) / 2
    } else {
        values[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_odd() {
        let values = vec![10, 20, 30, 40, 50];
        assert_eq!(median(values), 30);
    }

    #[test]
    fn test_median_even() {
        let values = vec![10, 20, 30, 40];
        assert_eq!(median(values), 25);
    }

    #[test]
    fn test_median_unsorted() {
        let values = vec![50, 10, 40, 20, 30];
        assert_eq!(median(values), 30);
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median(vec![]), 0);
    }

    #[test]
    fn test_format_number_with_separator() {
        let config = ComparisonConfig::default();
        let report = BenchReport {
            diffs: vec![],
            config,
            title: "".into(),
        };
        assert_eq!(report.format_number(1_000_000), "1_000_000");
        assert_eq!(report.format_number(123), "123");
        assert_eq!(report.format_number(12345), "12_345");
    }

    #[test]
    fn test_format_diff() {
        let config = ComparisonConfig::default();
        let report = BenchReport {
            diffs: vec![],
            config,
            title: "".into(),
        };
        assert_eq!(report.format_diff(500), "+500");
        assert_eq!(report.format_diff(-500), "-500");
        assert_eq!(report.format_diff(0), "0");
    }

    #[test]
    fn test_metric_status_logic() {
        let config = ComparisonConfig::default();

        assert_eq!(
            MetricStatus::from_change(0.5, &config),
            MetricStatus::Neutral
        );
        assert_eq!(
            MetricStatus::from_change(-0.5, &config),
            MetricStatus::Neutral
        );

        // Improvements
        assert_eq!(
            MetricStatus::from_change(-2.0, &config),
            MetricStatus::ImprovedMinor
        );
        assert_eq!(
            MetricStatus::from_change(-10.0, &config),
            MetricStatus::ImprovedStrong
        );

        // Regressions
        assert_eq!(
            MetricStatus::from_change(2.0, &config),
            MetricStatus::RegressedMinor
        );
        assert_eq!(
            MetricStatus::from_change(10.0, &config),
            MetricStatus::RegressedStrong
        );
    }

    #[test]
    fn test_report_regression_detection() {
        let config = ComparisonConfig::default();
        let diffs = vec![MetricDiff {
            path: "test".into(),
            current: 110,
            baseline: Some(100),
            status: MetricStatus::RegressedStrong,
            percent: Some(10.0),
        }];
        let report = ReportBuilder::new(diffs).with_config(config).build();
        assert!(report.has_significant_regression());
    }

    #[test]
    fn test_report_any_deviation() {
        let diffs = vec![MetricDiff {
            path: "test".into(),
            current: 98,
            baseline: Some(100),
            status: MetricStatus::ImprovedMinor,
            percent: Some(-2.0),
        }];
        let report = ReportBuilder::new(diffs).build();

        assert!(report.has_any_deviation(1.0));
        assert!(!report.has_any_deviation(5.0));
    }
}
