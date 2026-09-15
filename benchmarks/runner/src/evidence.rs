use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawRecord {
    pub schema_version: u32,
    pub scenario_id: String,
    pub availability: String,
    pub tool: String,
    pub round: u32,
    pub outcome: String,
    pub started_at_unix_ms: u64,
    pub ended_at_unix_ms: u64,
    pub elapsed_ms: u64,
    pub timeout_secs: u64,
    pub exit_code: Option<i32>,
    pub command_sha256: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub spec_sha256: String,
    pub fixture_tree_sha256: String,
    pub source_commit: String,
    pub docker_client_version: String,
    pub docker_server_version: String,
    pub docker_api_version: String,
    pub lightr_version: String,
    pub lightr_sha256: String,
    pub host_os: String,
    pub host_arch: String,
    pub host_kernel: String,
    pub assertions: Vec<AssertionRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertionRecord {
    pub kind: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HostIdentity {
    pub hostname: String,
    pub kernel: String,
    pub arch: String,
    pub os: String,
}

impl HostIdentity {
    pub fn current() -> Self {
        let mut identity = Self {
            hostname: "unknown".to_string(),
            kernel: "unknown".to_string(),
            arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
        };
        if let Ok(h) = std::process::Command::new("hostname").output() {
            if h.status.success() {
                identity.hostname = String::from_utf8_lossy(&h.stdout).trim().to_string();
            }
        }
        if let Ok(k) = std::process::Command::new("uname").args(["-r"]).output() {
            if k.status.success() {
                identity.kernel = String::from_utf8_lossy(&k.stdout).trim().to_string();
            }
        }
        identity
    }
}

impl RawRecord {
    pub fn new(
        scenario_id: String,
        availability: String,
        tool: String,
        round: u32,
        spec_digest: String,
        fixture_tree_digest: String,
        source_commit: String,
        docker_client_version: String,
        docker_server_version: String,
        docker_api_version: String,
        lightr_version: String,
        lightr_digest: String,
    ) -> Self {
        let start_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let host = HostIdentity::current();
        Self {
            schema_version: 1,
            scenario_id,
            availability,
            tool,
            round,
            outcome: "failed".to_string(),
            started_at_unix_ms: start_ms,
            ended_at_unix_ms: start_ms,
            elapsed_ms: 0,
            timeout_secs: 300,
            exit_code: None,
            command_sha256: String::new(),
            stdout_sha256: String::new(),
            stderr_sha256: String::new(),
            spec_sha256: spec_digest,
            fixture_tree_sha256: fixture_tree_digest,
            source_commit,
            docker_client_version,
            docker_server_version,
            docker_api_version,
            lightr_version,
            lightr_sha256: lightr_digest,
            host_os: host.os,
            host_arch: host.arch,
            host_kernel: host.kernel,
            assertions: Vec::new(),
        }
    }

    pub fn start_timed(&mut self) {
        let start_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.started_at_unix_ms = start_ms;
    }

    pub fn finish(
        &mut self,
        exit_code: Option<i32>,
        stdout: &[u8],
        stderr: &[u8],
        timed_out: bool,
        command: &str,
    ) {
        let end_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.ended_at_unix_ms = end_ms;
        self.elapsed_ms = end_ms.saturating_sub(self.started_at_unix_ms);
        self.exit_code = exit_code;
        self.stdout_sha256 = Self::sha256_hex(stdout);
        self.stderr_sha256 = Self::sha256_hex(stderr);
        self.command_sha256 = Self::sha256_hex(command.as_bytes());

        self.outcome = if timed_out {
            "timed_out".to_string()
        } else if exit_code == Some(0) {
            "passed".to_string()
        } else {
            "failed".to_string()
        };
    }

    pub fn record_skip(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.started_at_unix_ms = now;
        self.ended_at_unix_ms = now;
        self.elapsed_ms = 0;
        self.exit_code = None;
        self.stdout_sha256 = String::new();
        self.stderr_sha256 = String::new();
        self.command_sha256 = String::new();
        self.outcome = "skipped".to_string();
    }

    pub fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).expect("serialization infallible")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryRecord {
    pub scenario_id: String,
    pub category: String,
    pub availability: String,
    pub tool: String,
    pub rounds: u32,
    pub successful_rounds: u32,
    pub mean_ms: f64,
    pub stddev_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub outcome_counts: std::collections::HashMap<String, u32>,
}

impl SummaryRecord {
    pub fn from_raw(records: &[RawRecord], category: &str, availability: &str) -> Vec<Self> {
        use std::collections::HashMap;
        let mut by_scenario_tool: HashMap<(String, String), Vec<&RawRecord>> = HashMap::new();
        for r in records {
            if r.outcome != "skipped" {
                by_scenario_tool
                    .entry((r.scenario_id.clone(), r.tool.clone()))
                    .or_default()
                    .push(r);
            }
        }

        let mut summaries = Vec::new();
        for ((scenario_id, tool), recs) in by_scenario_tool {
            let mut durations: Vec<f64> = recs
                .iter()
                .filter(|r| r.outcome == "passed")
                .map(|r| r.elapsed_ms as f64)
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
                (mean, stddev, p50, p95, durations[0] as f64, durations[n - 1] as f64)
            };

            let mut outcome_counts = HashMap::new();
            for r in &recs {
                *outcome_counts.entry(r.outcome.clone()).or_insert(0) += 1;
            }

            summaries.push(Self {
                scenario_id,
                category: category.to_string(),
                availability: availability.to_string(),
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
        summaries
    }

    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}",
            self.scenario_id,
            self.category,
            self.availability,
            self.tool,
            self.rounds,
            self.successful_rounds,
            self.mean_ms,
            self.stddev_ms,
            self.p50_ms,
            self.p95_ms,
            self.min_ms,
            self.max_ms
        )
    }
}