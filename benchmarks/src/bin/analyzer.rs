use anyhow::Result;
use awesome_sails_benchmarks::{BenchStorage, ComparisonConfig, ReportBuilder};
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(name = "bench-analyzer")]
struct Cli {
    #[arg(long)]
    current: PathBuf,
    #[arg(long)]
    other: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Custom regression threshold for failure (e.g. 5.0)
    #[arg(long)]
    threshold: Option<f64>,
    /// Enable strict mode (fail on ANY deviation > threshold). Requires --threshold.
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
