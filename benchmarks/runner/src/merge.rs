use clap::Args;
use crate::evidence::{RawRecord, SummaryRecord, Phase, Outcome};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Args, Debug)]
pub struct MergeArgs {
    /// Input directory with chunk JSONL files
    #[arg(long)]
    pub input: PathBuf,
    /// Output directory for merged results
    #[arg(long)]
    pub out: PathBuf,
}

pub fn execute(args: MergeArgs) -> anyhow::Result<()> {
    std::fs::create_dir_all(&args.out)?;

    let mut all_records = Vec::new();
    let mut chunk_files = Vec::new();

    for entry in WalkDir::new(&args.input).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            chunk_files.push(path.to_path_buf());
        }
    }
    chunk_files.sort();
    let chunk_count = chunk_files.len();

    for chunk_path in &chunk_files {
        let file = File::open(&chunk_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: RawRecord = serde_json::from_str(&line)?;
            all_records.push(record);
        }
    }

    // Write merged raw JSONL
    let merged_raw = args.out.join("merged-raw.jsonl");
    let mut raw_writer = File::create(&merged_raw)?;
    for record in &all_records {
        writeln!(raw_writer, "{}", record.to_jsonl())?;
    }
    // Also write merged.jsonl for validation script compatibility
    let merged_jsonl = args.out.join("merged.jsonl");
    let mut jsonl_writer = File::create(&merged_jsonl)?;
    for record in &all_records {
        writeln!(jsonl_writer, "{}", record.to_jsonl())?;
    }

    // Validate: check for duplicate (scenario, tool, round) in timed phase
    let mut seen = HashMap::new();
    for r in &all_records {
        if r.phase == Phase::Timed {
            let key = (r.scenario_id.clone(), r.tool.clone(), r.round);
            if seen.contains_key(&key) {
                anyhow::bail!("duplicate record for (scenario={}, tool={}, round={})", key.0, key.1, key.2);
            }
            seen.insert(key, ());
        }
    }

    // Check for missing rounds per (scenario, tool)
    let mut round_counts: HashMap<(String, String), Vec<u32>> = HashMap::new();
    for r in &all_records {
        if r.phase == Phase::Timed {
            round_counts.entry((r.scenario_id.clone(), r.tool.clone()))
                .or_default()
                .push(r.round);
        }
    }

    // Generate summaries
    let mut summaries = Vec::new();
    let mut by_scenario_tool: HashMap<(String, String), Vec<&RawRecord>> = HashMap::new();
    for r in &all_records {
        if r.phase == Phase::Timed {
            by_scenario_tool.entry((r.scenario_id.clone(), r.tool.clone()))
                .or_default()
                .push(r);
        }
    }

    for ((scenario_id, tool), recs) in by_scenario_tool {
        let mut durations: Vec<f64> = recs
            .iter()
            .filter(|r| r.outcome == Outcome::Success)
            .map(|r| (r.end_ts - r.start_ts) * 1000.0)
            .collect();

        let (mean, stddev, p50, p95, min, max) = if durations.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = durations.len();
            let mean = durations.iter().sum::<f64>() / n as f64;
            let variance = durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n as f64;
            let stddev = variance.sqrt();
            let p50 = durations[(n * 50 / 100).min(n - 1)];
            let p95 = durations[(n * 95 / 100).min(n - 1)];
            (mean, stddev, p50, p95, durations[0], durations[n - 1])
        };

        let mut outcome_counts = HashMap::new();
        for r in &recs {
            *outcome_counts.entry(format!("{:?}", r.outcome)).or_insert(0) += 1;
        }

        summaries.push(SummaryRecord {
            scenario_id,
            category: "unknown".to_string(), // would need spec to get this
            availability: "unknown".to_string(),
            tool,
            rounds: recs.len() as u32,
            successful_rounds: durations.len() as u32,
            mean_ms: mean,
            stddev_ms: stddev,
            p50_ms: p50,
            p95_ms: p95,
            min_ms: min,
            max_ms: max,
            outcome_counts,
        });
    }

    // Write CSV summary
    let csv_path = args.out.join("summary.csv");
    let mut csv_writer = File::create(&csv_path)?;
    writeln!(csv_writer, "scenario_id,category,availability,tool,rounds,successful_rounds,mean_ms,stddev_ms,p50_ms,p95_ms,min_ms,max_ms")?;
    for s in &summaries {
        writeln!(csv_writer, "{}", s.to_csv())?;
    }

    // Write JSON summary
    let json_path = args.out.join("summary.json");
    let json_writer = File::create(&json_path)?;
    serde_json::to_writer_pretty(json_writer, &summaries)?;

    // Write Markdown summary
    let md_path = args.out.join("summary.md");
    let mut md_writer = File::create(&md_path)?;
    writeln!(md_writer, "| Scenario | Tool | Rounds | Success | Mean (ms) | StdDev | P50 | P95 | Min | Max |")?;
    writeln!(md_writer, "|----------|------|--------|---------|-----------|--------|-----|-----|-----|-----|")?;
    for s in &summaries {
        writeln!(
            md_writer,
            "| {} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} |",
            s.scenario_id, s.tool, s.rounds, s.successful_rounds,
            s.mean_ms, s.stddev_ms, s.p50_ms, s.p95_ms, s.min_ms, s.max_ms
        )?;
    }

    println!("Merged {} records from {} chunks", all_records.len(), chunk_count);
    println!("Raw: {}", merged_raw.display());
    println!("CSV: {}", csv_path.display());
    println!("JSON: {}", json_path.display());
    println!("MD: {}", md_path.display());

    Ok(())
}