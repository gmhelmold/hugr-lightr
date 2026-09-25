use crate::evidence::{AssertionRecord, RawRecord};
use crate::spec::{Assertion, Availability, Fixture, Scenario, Spec, ToolCommand};
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
                metrics: vec![
                    "latency".to_string(),
                    "throughput".to_string(),
                    "cpu".to_string(),
                    "mem".to_string(),
                ],
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
                metrics: vec![
                    "latency".to_string(),
                    "throughput".to_string(),
                    "cpu".to_string(),
                    "mem".to_string(),
                ],
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
            schema_version: 1,
            scenario_id: "scenario-1".to_string(),
            availability: "supported".to_string(),
            tool: "docker".to_string(),
            round: 1,
            outcome: "passed".to_string(),
            started_at_unix_ms: 1000000,
            ended_at_unix_ms: 1001500,
            elapsed_ms: 1500,
            timeout_secs: 300,
            exit_code: Some(0),
            command_sha256: "abc".to_string(),
            stdout_sha256: "abc".to_string(),
            stderr_sha256: "def".to_string(),
            spec_sha256: "spec123".to_string(),
            fixture_tree_sha256: "fixture123".to_string(),
            source_commit: "commit123".to_string(),
            docker_client_version: "28.3.2".to_string(),
            docker_server_version: "28.3.2".to_string(),
            docker_api_version: "1.48".to_string(),
            lightr_version: "0.1.0".to_string(),
            lightr_sha256: "lightr123".to_string(),
            host_os: "linux".to_string(),
            host_arch: "x86_64".to_string(),
            host_kernel: "6.8.0".to_string(),
            assertions: vec![AssertionRecord {
                kind: "exit_code".to_string(),
                passed: true,
            }],
        },
        RawRecord {
            schema_version: 1,
            scenario_id: "scenario-1".to_string(),
            availability: "supported".to_string(),
            tool: "lightr".to_string(),
            round: 1,
            outcome: "passed".to_string(),
            started_at_unix_ms: 1000000,
            ended_at_unix_ms: 1000800,
            elapsed_ms: 800,
            timeout_secs: 300,
            exit_code: Some(0),
            command_sha256: "abc".to_string(),
            stdout_sha256: "abc".to_string(),
            stderr_sha256: "def".to_string(),
            spec_sha256: "spec123".to_string(),
            fixture_tree_sha256: "fixture123".to_string(),
            source_commit: "commit123".to_string(),
            docker_client_version: "28.3.2".to_string(),
            docker_server_version: "28.3.2".to_string(),
            docker_api_version: "1.48".to_string(),
            lightr_version: "0.1.0".to_string(),
            lightr_sha256: "lightr123".to_string(),
            host_os: "linux".to_string(),
            host_arch: "x86_64".to_string(),
            host_kernel: "6.8.0".to_string(),
            assertions: vec![AssertionRecord {
                kind: "exit_code".to_string(),
                passed: true,
            }],
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
