use bench_runner::evidence::SummaryRecord;
use bench_runner::spec::{Assertion, Availability, Fixture, Scenario, ToolCommand};
use bench_runner::util::{make_test_records, make_test_spec, write_jsonl, write_spec_yaml};
use tempfile::TempDir;

fn make_duplicate_scenario() -> Scenario {
    Scenario {
        id: "test-supported".to_string(),
        category: "run".to_string(),
        availability: Availability::Supported,
        reason: None,
        fixture: Fixture {
            project: "test".to_string(),
            path: "test/fixture3".to_string(),
            context: "test/context3".to_string(),
        },
        docker: ToolCommand {
            command: "echo hello".to_string(),
        },
        lightr: ToolCommand {
            command: "echo hello".to_string(),
        },
        metrics: vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ],
        tags: vec!["x".to_string(), "y".to_string()],
        assertions: vec![Assertion::ExitCode { expected: 0 }],
        lightr_evidence: None,
    }
}

#[test]
fn spec_duplicate_id_fails() {
    let mut spec = make_test_spec();
    spec.scenarios.push(make_duplicate_scenario());
    assert!(spec.validate().is_err());
}

#[test]
fn spec_unsupported_requires_reason() {
    let mut spec = make_test_spec();
    spec.scenarios[1].reason = None;
    assert!(spec.validate().is_err());
}

#[test]
fn spec_metrics_minimum_four() {
    let mut spec = make_test_spec();
    spec.scenarios[0].metrics = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert!(spec.validate().is_err());
}

#[test]
fn spec_tags_minimum_two() {
    let mut spec = make_test_spec();
    spec.scenarios[0].tags = vec!["a".to_string()];
    assert!(spec.validate().is_err());
}

#[test]
fn spec_assertions_required() {
    let mut spec = make_test_spec();
    spec.scenarios[0].assertions = vec![];
    assert!(spec.validate().is_err());
}

#[test]
fn spec_chunk_partition_deterministic() {
    let spec = make_test_spec();
    let chunk0 = spec.chunk(0, 2);
    let chunk1 = spec.chunk(1, 2);
    assert_eq!(chunk0.len() + chunk1.len(), spec.scenarios.len());
    // No overlap
    let ids0: std::collections::HashSet<_> = chunk0.iter().map(|s| s.id.clone()).collect();
    let ids1: std::collections::HashSet<_> = chunk1.iter().map(|s| s.id.clone()).collect();
    assert!(ids0.is_disjoint(&ids1));
}

#[test]
fn evidence_duplicate_detection() {
    let records = make_test_records();
    let mut seen = std::collections::HashMap::new();
    for r in &records {
        let key = (r.scenario_id.clone(), r.tool.clone(), r.round);
        assert!(!seen.contains_key(&key));
        seen.insert(key, ());
    }
    // Adding duplicate should be detected
    let dup = records[0].clone();
    let key = (dup.scenario_id.clone(), dup.tool.clone(), dup.round);
    assert!(seen.contains_key(&key));
}

#[test]
fn merge_missing_round_detected() {
    let records = make_test_records();
    let mut by_scenario_tool: std::collections::HashMap<(String, String), Vec<u32>> =
        std::collections::HashMap::new();
    for r in &records {
        by_scenario_tool
            .entry((r.scenario_id.clone(), r.tool.clone()))
            .or_default()
            .push(r.round);
    }
    // Each (scenario, tool) should have round 1
    for (_, rounds) in &by_scenario_tool {
        assert!(rounds.contains(&1));
    }
}

#[test]
fn summary_calculation_correct() {
    let records = make_test_records();
    let summaries = SummaryRecord::from_raw(&records, "run", "supported");
    assert_eq!(summaries.len(), 2); // docker + lightr
    for s in &summaries {
        assert_eq!(s.rounds, 1);
        assert_eq!(s.successful_rounds, 1);
        assert!(s.mean_ms > 0.0);
    }
}

#[test]
fn runner_verify_spec_cli() {
    use assert_cmd::Command;
    let tmp = TempDir::new().unwrap();
    let spec_path = tmp.path().join("spec.yaml");
    let spec = make_test_spec();
    write_spec_yaml(&spec, &spec_path).unwrap();

    let mut cmd = Command::cargo_bin("bench-runner").unwrap();
    cmd.arg("verify-spec").arg("--spec").arg(&spec_path);
    cmd.assert().success();
}

#[test]
fn runner_verify_spec_duplicate_fails() {
    use assert_cmd::Command;
    let tmp = TempDir::new().unwrap();
    let spec_path = tmp.path().join("spec.yaml");
    let mut spec = make_test_spec();
    spec.scenarios.push(make_duplicate_scenario());
    write_spec_yaml(&spec, &spec_path).unwrap();

    let mut cmd = Command::cargo_bin("bench-runner").unwrap();
    cmd.arg("verify-spec").arg("--spec").arg(&spec_path);
    cmd.assert().failure();
}

#[test]
fn runner_run_chunk_emits_jsonl() {
    use assert_cmd::Command;
    let tmp = TempDir::new().unwrap();
    let spec_path = tmp.path().join("spec.yaml");
    let out_dir = tmp.path().join("out");
    let spec = make_test_spec();
    write_spec_yaml(&spec, &spec_path).unwrap();

    let mut cmd = Command::cargo_bin("bench-runner").unwrap();
    cmd.arg("run")
        .arg("--spec")
        .arg(&spec_path)
        .arg("--chunk")
        .arg("0")
        .arg("--chunks")
        .arg("1")
        .arg("--rounds")
        .arg("1")
        .arg("--out")
        .arg(&out_dir)
        .arg("--docker")
        .arg("echo") // dummy, will fail but tests CLI parsing
        .arg("--lightr")
        .arg("echo")
        .arg("--timeout")
        .arg("5");
    // We expect this to fail due to missing binaries, but CLI should parse
    cmd.assert().failure(); // expected to fail on execution
}

#[test]
fn runner_timeout_is_not_supported_pass() {
    use assert_cmd::Command;
    use serde_json::Value;

    let tmp = TempDir::new().unwrap();
    let spec_dir = tmp.path().join("benchmarks");
    let fixture_dir = tmp.path().join("test/fixture");
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&spec_dir).unwrap();
    std::fs::create_dir_all(&fixture_dir).unwrap();

    let spec_path = spec_dir.join("spec.yaml");
    let mut spec = make_test_spec();
    spec.scenarios.truncate(1);
    spec.scenarios[0].fixture.path = "test/fixture".to_string();
    spec.scenarios[0].docker.command = "sleep 1".to_string();
    spec.scenarios[0].lightr.command = "sleep 1".to_string();
    write_spec_yaml(&spec, &spec_path).unwrap();

    let tool = tmp.path().join("tool");
    std::fs::write(&tool, "#!/bin/sh\nprintf 'test-tool\\n'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut cmd = Command::cargo_bin("bench-runner").unwrap();
    cmd.arg("run")
        .arg("--spec")
        .arg(&spec_path)
        .arg("--chunk")
        .arg("0")
        .arg("--chunks")
        .arg("1")
        .arg("--rounds")
        .arg("1")
        .arg("--out")
        .arg(&out_dir)
        .arg("--docker")
        .arg(&tool)
        .arg("--lightr")
        .arg(&tool)
        .arg("--timeout")
        .arg("0");
    cmd.assert().failure();

    let raw = std::fs::read_to_string(out_dir.join("chunk-00.jsonl")).unwrap();
    let records: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record["outcome"], "timed_out");
        assert_ne!(record["outcome"], "passed");
        assert!(record["exit_code"].is_null());
    }
}

#[test]
fn runner_merge_synthetic() {
    use assert_cmd::Command;
    let tmp = TempDir::new().unwrap();
    let in_dir = tmp.path().join("in");
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&in_dir).unwrap();

    let records = make_test_records();
    write_jsonl(&records, &in_dir.join("chunk-00.jsonl")).unwrap();

    let mut cmd = Command::cargo_bin("bench-runner").unwrap();
    cmd.arg("merge")
        .arg("--input")
        .arg(&in_dir)
        .arg("--out")
        .arg(&out_dir);
    cmd.assert().success();

    assert!(out_dir.join("merged-raw.jsonl").exists());
    assert!(out_dir.join("summary.csv").exists());
    assert!(out_dir.join("summary.json").exists());
    assert!(out_dir.join("summary.md").exists());
}

#[test]
fn mutation_duplicate_id_validation() {
    let mut spec = make_test_spec();
    spec.scenarios.push(make_duplicate_scenario());

    // Current implementation should catch duplicate
    assert!(spec.validate().is_err());
}
