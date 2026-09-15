use crate::spec::{Spec, Scenario, Availability, Assertion, Fixture, ToolCommand};
use crate::evidence::{RawRecord, Phase, Outcome};
use tempfile::TempDir;
use std::fs::File;
use std::io::Write;

pub fn make_test_spec() -> Spec {
    Spec {
        scenarios: vec![
            Scenario {
                id: "test-supported".to_string(),
                category: "run".to_string(),
                availability: Availability::Supported,
                reason: None,
                fixture: Fixture {
                    project: "test".to_string(),
                    path: "test/fixture".to_string(),
                    context: "test/context".to_string(),
                },
                docker: ToolCommand {
                    command: "echo hello".to_string(),
                },
                lightr: ToolCommand {
                    command: "echo hello".to_string(),
                },
                metrics: vec!["latency".to_string(), "throughput".to_string(), "cpu".to_string(), "mem".to_string()],
                tags: vec!["linux".to_string(), "container".to_string()],
                assertions: vec![Assertion::ExitCode { expected: 0 }],
                lightr_evidence: None,
            },
            Scenario {
                id: "test-unsupported".to_string(),
                category: "run".to_string(),
                availability: Availability::Unsupported,
                reason: Some("not implemented".to_string()),
                fixture: Fixture {
                    project: "test".to_string(),
                    path: "test/fixture2".to_string(),
                    context: "test/context2".to_string(),
                },
                docker: ToolCommand {
                    command: "echo hello".to_string(),
                },
                lightr: ToolCommand {
                    command: "echo hello".to_string(),
                },
                metrics: vec!["latency".to_string(), "throughput".to_string(), "cpu".to_string(), "mem".to_string()],
                tags: vec!["linux".to_string(), "container".to_string()],
                assertions: vec![Assertion::ExitCode { expected: 0 }],
                lightr_evidence: None,
            },
        ],
    }
}

pub fn write_spec_yaml(spec: &Spec, path: &std::path::Path) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(spec)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

pub fn make_test_records() -> Vec<RawRecord> {
    vec![
        RawRecord {
            scenario_id: "scenario-1".to_string(),
            tool: "docker".to_string(),
            round: 1,
            phase: Phase::Timed,
            start_ts: 1000.0,
            end_ts: 1001.5,
            timeout: false,
            exit_code: Some(0),
            stdout_sha256: "abc".to_string(),
            stderr_sha256: "def".to_string(),
            outcome: Outcome::Success,
            spec_digest: "spec123".to_string(),
            fixture_tree_digest: "fixture123".to_string(),
            source_commit: "commit123".to_string(),
            docker_client_version: "28.3.2".to_string(),
            docker_server_version: "28.3.2".to_string(),
            docker_api_version: "1.48".to_string(),
            lightr_version: "0.1.0".to_string(),
            lightr_digest: "lightr123".to_string(),
            host: crate::evidence::HostIdentity {
                hostname: "test".to_string(),
                kernel: "6.8.0".to_string(),
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
            },
        },
        RawRecord {
            scenario_id: "scenario-1".to_string(),
            tool: "lightr".to_string(),
            round: 1,
            phase: Phase::Timed,
            start_ts: 1000.0,
            end_ts: 1000.8,
            timeout: false,
            exit_code: Some(0),
            stdout_sha256: "abc".to_string(),
            stderr_sha256: "def".to_string(),
            outcome: Outcome::Success,
            spec_digest: "spec123".to_string(),
            fixture_tree_digest: "fixture123".to_string(),
            source_commit: "commit123".to_string(),
            docker_client_version: "28.3.2".to_string(),
            docker_server_version: "28.3.2".to_string(),
            docker_api_version: "1.48".to_string(),
            lightr_version: "0.1.0".to_string(),
            lightr_digest: "lightr123".to_string(),
            host: crate::evidence::HostIdentity {
                hostname: "test".to_string(),
                kernel: "6.8.0".to_string(),
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
            },
        },
    ]
}

pub fn write_jsonl(records: &[RawRecord], path: &std::path::Path) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    for r in records {
        writeln!(file, "{}", r.to_jsonl())?;
    }
    Ok(())
}