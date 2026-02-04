use anyhow::{Context, Result};
use clap::Parser;
use cli_table::{Cell, Style, Table, format::Justify};
use serde_json::Value;
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
}

struct MetricResult {
    path: String,
    current: String,
    baseline: String,
    change: String,
    percent: String,
    status: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let current_json: Value = serde_json::from_str(&fs::read_to_string(&cli.current)?)?;
    let other_json: Value = serde_json::from_str(&fs::read_to_string(&cli.other)?)?;

    let mut current_metrics = BTreeMap::new();
    let mut other_metrics = BTreeMap::new();

    flatten(&current_json, String::new(), &mut current_metrics);
    flatten(&other_json, String::new(), &mut other_metrics);

    let mut results = Vec::new();

    for (path, cur_val) in &current_metrics {
        if let Some(oth_val) = other_metrics.get(path) {
            let diff = *cur_val as i128 - *oth_val as i128;
            let percent = (diff as f64 / *oth_val as f64) * 100.0;

            let (st_text, emoji) = if percent < -5.0 {
                ("Improved", "🚀")
            } else if percent < -1.0 {
                ("Improved", "👍")
            } else if percent > 5.0 {
                ("Regressed", "❌")
            } else if percent > 1.0 {
                ("Regressed", "⚠️")
            } else {
                ("Neutral", "✅")
            };

            results.push(MetricResult {
                path: path.clone(),
                current: cur_val.to_string(),
                baseline: oth_val.to_string(),
                change: format!("{:+#}", diff),
                percent: format!("{:.2}%", percent),
                status: format!("{} {}", emoji, st_text),
            });
        } else {
            results.push(MetricResult {
                path: path.clone(),
                current: cur_val.to_string(),
                baseline: "-".to_string(),
                change: "-".to_string(),
                percent: "-".to_string(),
                status: "✨ New".to_string(),
            });
        }
    }

    for (path, oth_val) in &other_metrics {
        if !current_metrics.contains_key(path) {
            results.push(MetricResult {
                path: path.clone(),
                current: "-".to_string(),
                baseline: oth_val.to_string(),
                change: "-".to_string(),
                percent: "-".to_string(),
                status: "🗑️ Removed".to_string(),
            });
        }
    }

    // 1. CLI Output (Pretty ASCII Table)
    let table_rows: Vec<_> = results
        .iter()
        .map(|r| {
            vec![
                r.path.clone().cell(),
                r.current.clone().cell().justify(Justify::Right),
                r.baseline.clone().cell().justify(Justify::Right),
                r.change.clone().cell().justify(Justify::Right),
                r.percent.clone().cell().justify(Justify::Right),
                r.status.clone().cell(),
            ]
        })
        .collect();

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

    // 2. File Output (Clean Markdown Table)
    if let Some(out_path) = cli.output {
        let mut report = String::from("## 🔬 Benchmark Comparison\n\n");
        report.push_str("| Metric | Current | Baseline | Change | % | Status |\n");
        report.push_str("| :--- | ---: | ---: | ---: | ---: | :--- |\n");

        for r in results {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                r.path, r.current, r.baseline, r.change, r.percent, r.status
            ));
        }
        fs::write(out_path, report).context("Failed to write report file")?;
    }

    Ok(())
}

fn flatten(val: &Value, prefix: String, res: &mut BTreeMap<String, u64>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten(v, new_prefix, res);
            }
        }
        Value::Number(num) => {
            if let Some(n) = num.as_u64() {
                res.insert(prefix, n);
            }
        }
        _ => {}
    }
}
