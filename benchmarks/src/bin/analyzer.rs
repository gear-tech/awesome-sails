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
use awesome_sails_benchmarks::{BenchStorage, ComparisonConfig, ReportBuilder};
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "bench-analyzer",
    about = "A CLI tool to compare two benchmark JSON files and detect performance regressions."
)]
struct Cli {
    /// Path to the current benchmark results (the ones you just generated).
    #[arg(long)]
    current: PathBuf,
    /// Path to the baseline benchmark results (the reference file to compare against).
    #[arg(long)]
    other: PathBuf,
    /// Path to save the comparison report in Markdown format.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Custom regression threshold percentage (e.g. 5.0).
    /// If a metric grows beyond this, the tool will exit with an error.
    #[arg(long)]
    threshold: Option<f64>,
    /// Enable strict mode: the tool will fail if ANY metric deviates from baseline
    /// by more than the threshold (including improvements).
    /// Useful for CI self-checks to ensure benchmark stability.
    #[arg(long, requires = "threshold")]
    strict: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let storage_cur = BenchStorage::at_path(&cli.current);
    let storage_oth = BenchStorage::at_path(&cli.other);

    let mut config = ComparisonConfig::default();
    if let Some(t) = cli.threshold {
        config.regressed_strong = t;
    }

    let diffs = storage_cur.compare(&storage_oth, &config)?;

    let report = ReportBuilder::new(diffs).with_config(config).build();

    // 1. CLI Output (ASCII Table) - Uses the idiomatic Display impl
    println!("{}", report.as_ascii());

    // 2. Markdown Output - Uses the idiomatic Display impl
    if let Some(out_path) = cli.output {
        fs::write(out_path, report.as_markdown().to_string())?;
    }

    let failed = if cli.strict {
        report.has_any_deviation(cli.threshold.unwrap())
    } else {
        report.has_significant_regression()
    };

    if failed {
        core::panic!("Benchmark failure: deviation exceeds the allowed threshold!");
    }

    Ok(())
}
