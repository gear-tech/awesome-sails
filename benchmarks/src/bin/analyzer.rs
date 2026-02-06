use anyhow::Result;
use clap::Parser;
use cli_table::{Cell, Style, Table, format::Justify};
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Parser)]
#[command(name = "bench-analyzer")]
struct Cli {
    #[arg(long)]
    current: PathBuf,
    #[arg(long)]
    other: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Threshold percentage for failure
    #[arg(long)]
    threshold: Option<f64>,
}

struct MetricResult {
    path: String,
    current: u64,
    baseline: Option<u64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let current_raw = fs::read_to_string(&cli.current)?;
    let other_raw = fs::read_to_string(&cli.other)?;

    let current_json: BTreeMap<String, BTreeMap<String, u64>> = serde_json::from_str(&current_raw)?;
    let other_json: BTreeMap<String, BTreeMap<String, u64>> =
        serde_json::from_str(&other_raw).unwrap_or_default();

    let mut results = Vec::new();
    let mut threshold_failed = false;

    for (section, metrics) in current_json {
        for (name, cur_val) in metrics {
            let path = format!("{}.{}", section, name);
            let oth_val = other_json.get(&section).and_then(|m| m.get(&name)).copied();

            if let (Some(oth), Some(t)) = (oth_val, cli.threshold) {
                let diff_p = (cur_val as f64 - oth as f64) / oth as f64 * 100.0;
                if diff_p.abs() > t {
                    threshold_failed = true;
                }
            }

            results.push(MetricResult {
                path,
                current: cur_val,
                baseline: oth_val,
            });
        }
    }

    // 1. CLI Output
    let mut table_rows = Vec::new();
    for r in &results {
        let (change, percent, status) = if let Some(oth) = r.baseline {
            let diff = r.current as i128 - oth as i128;
            let p = (diff as f64 / oth as f64) * 100.0;
            let (st, emoji) = if p < -5.0 {
                ("Improved", "🚀")
            } else if p < -1.0 {
                ("Improved", "👍")
            } else if p > 5.0 {
                ("Regressed", "❌")
            } else if p > 1.0 {
                ("Regressed", "⚠️")
            } else {
                ("Neutral", "✅")
            };
            (
                format_diff(diff),
                format!("{:.2}%", p),
                format!("{} {}", emoji, st),
            )
        } else {
            ("-".to_string(), "-".to_string(), "✨ New".to_string())
        };

        table_rows.push(vec![
            r.path.clone().cell(),
            format_number(r.current).cell().justify(Justify::Right),
            r.baseline
                .map(format_number)
                .unwrap_or_else(|| "-".into())
                .cell()
                .justify(Justify::Right),
            change.cell().justify(Justify::Right),
            percent.cell().justify(Justify::Right),
            status.cell(),
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

    println!("\n## 🔬 Benchmark Comparison\n");
    let _ = cli_table::print_stdout(table);

    // 2. Markdown Output
    if let Some(out_path) = cli.output {
        let mut report = String::from("### 🔬 Benchmark Comparison Results\n\n");
        report.push_str("| Metric | Current | Baseline | Change | % | Status |\n");
        report.push_str("| :--- | ---: | ---: | ---: | ---: | :--- |\n");

        for r in &results {
            let (change, percent, status) = if let Some(oth) = r.baseline {
                let diff = r.current as i128 - oth as i128;
                let p = (diff as f64 / oth as f64) * 100.0;
                let emoji = if p < -5.0 {
                    "🚀"
                } else if p < -1.0 {
                    "👍"
                } else if p > 5.0 {
                    "❌"
                } else if p > 1.0 {
                    "⚠️"
                } else {
                    "✅"
                };
                (format_diff(diff), format!("{:.2}%", p), emoji)
            } else {
                ("-".to_string(), "-".to_string(), "✨")
            };
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                r.path,
                format_number(r.current),
                r.baseline.map(format_number).unwrap_or_else(|| "-".into()),
                change,
                percent,
                status
            ));
        }

        report.push_str("\n#### Legend\n");
        report.push_str("- 🚀 Significant improvement (>5% reduction)\n");
        report.push_str("- 👍 Minor improvement (<5% reduction)\n");
        report.push_str("- ✅ No significant change\n");
        report.push_str("- ⚠️ Minor regression (<5% increase)\n");
        report.push_str("- ❌ Significant regression (>5% increase)\n");

        fs::write(out_path, report)?;
    }

    if let (Some(t), true) = (cli.threshold, threshold_failed) {
        core::panic!("Benchmark failure: deviation exceeds threshold of {}%!", t);
    }

    Ok(())
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut res = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            res.push('_');
        }
        res.push(c);
    }
    res.chars().rev().collect()
}

fn format_diff(n: i128) -> String {
    if n == 0 {
        return "0".into();
    }
    let prefix = if n > 0 { "+" } else { "-" };
    format!("{}{}", prefix, format_number(n.unsigned_abs() as u64))
}
