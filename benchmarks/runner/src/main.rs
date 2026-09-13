use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TIMEOUT_SECS: u64 = 300;
const SCHEMA_VERSION: u32 = 1;

#[derive(Parser)]
#[command(name = "bench-runner")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    VerifySpec { #[arg(long)] spec: PathBuf },
    Run {
        #[arg(long)] spec: PathBuf,
        #[arg(long)] chunk: usize,
        #[arg(long)] chunks: usize,
        #[arg(long)] rounds: usize,
        #[arg(long)] out: PathBuf,
        #[arg(long)] docker: PathBuf,
        #[arg(long)] lightr: PathBuf,
    },
    Merge { #[arg(long)] input: PathBuf, #[arg(long)] out: PathBuf },
}

#[derive(Debug, Deserialize)]
struct Spec {
    taxonomy: Taxonomy,
    source_evidence: SourceEvidence,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Taxonomy {
    category: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SourceEvidence {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Clone)]
struct Project {
    id: String,
    repo: String,
    commit: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Scenario {
    id: String,
    category: String,
    availability: Availability,
    reason: Option<String>,
    fixture: Option<Fixture>,
    docker: Option<ToolCommand>,
    lightr: Option<ToolCommand>,
    metrics: Vec<String>,
    tags: Vec<String>,
    #[serde(default)]
    assertions: Vec<Assertion>,
    lightr_evidence: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct Fixture {
    project: String,
    path: Option<String>,
    context: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ToolCommand { command: Option<String> }

#[derive(Debug, Deserialize, Clone)]
struct Assertion {
    kind: String,
    #[serde(default)]
    expected: Value,
    #[serde(default)]
    scope: String,
    command: Option<String>,
    path: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Availability { Supported, Unsupported, HardwareGated, OutOfScope }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RawRecord {
    schema_version: u32,
    scenario_id: String,
    availability: Availability,
    tool: String,
    round: usize,
    outcome: String,
    started_at_unix_ms: u128,
    ended_at_unix_ms: u128,
    elapsed_ms: u128,
    timeout_secs: u64,
    exit_code: Option<i32>,
    command_sha256: String,
    stdout_sha256: String,
    stderr_sha256: String,
    spec_sha256: String,
    fixture_tree_sha256: Option<String>,
    source_commit: Option<String>,
    docker_client_version: Option<String>,
    docker_server_version: Option<String>,
    docker_api_version: Option<String>,
    lightr_version: Option<String>,
    lightr_sha256: Option<String>,
    host_os: String,
    host_arch: String,
    host_kernel: String,
    assertions: Vec<AssertionResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AssertionResult {
    kind: String,
    passed: bool,
    detail: String,
}

struct CommandResult {
    status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
    error: Option<String>,
    started: u128,
    ended: u128,
}

struct Versions {
    docker_client: Option<String>,
    docker_server: Option<String>,
    docker_api: Option<String>,
    lightr: Option<String>,
    lightr_hash: Option<String>,
    error: Option<String>,
}

fn main() {
    let result = match Cli::parse().command {
        Action::VerifySpec { spec } => load_spec(&spec).and_then(|(spec, _)| validate_spec(&spec)),
        Action::Run { spec, chunk, chunks, rounds, out, docker, lightr } => run(&spec, chunk, chunks, rounds, &out, &docker, &lightr),
        Action::Merge { input, out } => merge(&input, &out),
    };
    if let Err(error) = result {
        eprintln!("bench-runner: {error}");
        std::process::exit(1);
    }
}

fn load_spec(path: &Path) -> Result<(Spec, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("read spec {}: {e}", path.display()))?;
    let spec = serde_yaml::from_slice(&bytes).map_err(|e| format!("parse spec {}: {e}", path.display()))?;
    validate_spec(&spec)?;
    Ok((spec, bytes))
}

fn validate_spec(spec: &Spec) -> Result<(), String> {
    if spec.scenarios.len() != 250 { return Err(format!("corpus must contain 250 scenarios, got {}", spec.scenarios.len())); }
    let known_availability = [Availability::Supported, Availability::Unsupported, Availability::HardwareGated, Availability::OutOfScope];
    let categories: BTreeSet<_> = spec.taxonomy.category.iter().collect();
    let tags: BTreeSet<_> = spec.taxonomy.category.iter().chain(spec.taxonomy.evidence.iter()).collect();
    let projects: BTreeSet<_> = spec.source_evidence.projects.iter().map(|p| p.id.as_str()).collect();
    let mut ids = BTreeSet::new();
    for scenario in &spec.scenarios {
        if !ids.insert(&scenario.id) { return Err(format!("duplicate scenario id: {}", scenario.id)); }
        if scenario.id.is_empty() || !scenario.id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') { return Err(format!("unsafe scenario id: {}", scenario.id)); }
        if !categories.contains(&scenario.category) { return Err(format!("unknown category for {}: {}", scenario.id, scenario.category)); }
        if !known_availability.contains(&scenario.availability) { return Err(format!("unknown availability for {}", scenario.id)); }
        if scenario.tags.len() < 2 || scenario.tags.iter().any(|tag| !tags.contains(tag)) {
            return Err(format!("unknown or insufficient tags for {}", scenario.id));
        }
        if scenario.metrics.len() < 4 { return Err(format!("missing metrics for {}", scenario.id)); }
        if scenario.availability != Availability::Supported {
            if scenario.reason.as_deref().unwrap_or("").trim().is_empty() { return Err(format!("missing non-supported reason for {}", scenario.id)); }
            if scenario.docker.as_ref().and_then(|v| v.command.as_deref()).is_some() || scenario.lightr.as_ref().and_then(|v| v.command.as_deref()).is_some() {
                return Err(format!("non-supported scenario has executable command: {}", scenario.id));
            }
            continue;
        }
        let fixture = scenario.fixture.as_ref().ok_or_else(|| format!("missing fixture for {}", scenario.id))?;
        if !projects.contains(fixture.project.as_str()) || fixture.path.as_deref().unwrap_or("").is_empty() || fixture.context.as_deref().unwrap_or("").is_empty() { return Err(format!("missing supported fixture path/context for {}", scenario.id)); }
        if scenario.docker.as_ref().and_then(|v| v.command.as_deref()).unwrap_or("").is_empty() || scenario.lightr.as_ref().and_then(|v| v.command.as_deref()).unwrap_or("").is_empty() { return Err(format!("missing supported commands for {}", scenario.id)); }
        if scenario.assertions.is_empty() || scenario.lightr_evidence.is_none() { return Err(format!("missing supported assertions/source evidence for {}", scenario.id)); }
        for assertion in &scenario.assertions {
            if !["exit_code", "stdout_exact", "stdout_regex", "stderr_regex", "file_sha256", "http_status", "command"].contains(&assertion.kind.as_str()) {
                return Err(format!("unknown assertion kind for {}: {}", scenario.id, assertion.kind));
            }
        }
    }
    Ok(())
}

fn run(spec_path: &Path, chunk: usize, chunks: usize, rounds: usize, out: &Path, docker: &Path, lightr: &Path) -> Result<(), String> {
    if chunks == 0 || chunk >= chunks || rounds == 0 { return Err("require chunks > 0, chunk < chunks, and rounds > 0".into()); }
    let (spec, bytes) = load_spec(spec_path)?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut records = File::create(out.join("records.jsonl")).map_err(|e| e.to_string())?;
    let spec_hash = sha256(&bytes);
    let selected: Vec<_> = select_scenarios(&spec.scenarios, chunk, chunks);
    let mut failed = false;
    for scenario in selected {
        if scenario.availability != Availability::Supported {
            write_record(&mut records, &skip_record(scenario, &spec_hash))?;
            continue;
        }
        let project = scenario.fixture.as_ref().and_then(|f| spec.source_evidence.projects.iter().find(|p| p.id == f.project));
        let fixture = materialize_fixture(spec_path, scenario, project, out);
        let versions = probe_versions(docker, lightr);
        for round in round_indices(rounds) {
            let scenario_out = out.join("scenarios").join(&scenario.id).join(round.to_string());
            if let Err(error) = fs::create_dir_all(&scenario_out) {
                failed = true;
                eprintln!("bench-runner: create scenario output {}: {error}", scenario_out.display());
            }
            let context = match &fixture {
                Ok((path, hash, commit)) => Some((path.as_path(), hash.as_str(), commit.as_str())),
                Err(_) => None,
            };
            let failure = fixture.as_ref().err().cloned().or_else(|| versions.error.clone());
            let (mut docker_record, docker_result) = execute_record(scenario, "docker", round, context, &scenario_out, &versions, &spec_hash, docker, lightr, failure.as_deref());
            let (mut lightr_record, lightr_result) = execute_record(scenario, "lightr", round, context, &scenario_out, &versions, &spec_hash, docker, lightr, failure.as_deref());
            if let Some((fixture_dir, _, _)) = context {
                apply_assertions(&mut docker_record, scenario, "docker", &docker_result, docker, lightr, fixture_dir, &scenario_out);
                apply_assertions(&mut lightr_record, scenario, "lightr", &lightr_result, docker, lightr, fixture_dir, &scenario_out);
            }
            failed |= docker_record.outcome != "passed" || lightr_record.outcome != "passed";
            write_record(&mut records, &docker_record)?;
            write_record(&mut records, &lightr_record)?;
        }
    }
    if failed { Err("one or more selected supported scenarios failed".into()) } else { Ok(()) }
}

fn execute_record(s: &Scenario, tool: &str, round: usize, fixture: Option<(&Path, &str, &str)>, output_dir: &Path, versions: &Versions, spec_hash: &str, docker: &Path, lightr: &Path, preflight_error: Option<&str>) -> (RawRecord, CommandResult) {
    let now = unix_ms();
    let command = if tool == "docker" { s.docker.as_ref().and_then(|v| v.command.as_deref()) } else { s.lightr.as_ref().and_then(|v| v.command.as_deref()) }.unwrap_or("");
    let (result, expanded) = if let Some(error) = preflight_error {
        (failed_result(error, now), Err(error.to_string()))
    } else if let Some((fixture_dir, _, _)) = fixture {
        let expanded = expand(command, docker, lightr, fixture_dir, output_dir);
        match &expanded { Ok(command) => (run_shell(command, Duration::from_secs(TIMEOUT_SECS), Some(&output_dir.join("lightr-home"))), expanded), Err(error) => (failed_result(error, now), expanded) }
    } else { (failed_result("fixture unavailable", now), Err("fixture unavailable".into())) };
    let passed = result.error.is_none() && !result.timed_out && result.status.as_ref().is_some_and(|status| status.success());
    let outcome = if result.timed_out { "timed_out" } else if passed { "passed" } else { "failed" };
    if !passed {
        eprintln!(
            "bench-runner: {}/{} command failed: {}",
            s.id,
            tool,
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    let record = record_base(s, tool, round, outcome, &result, expanded.as_deref().unwrap_or(command), spec_hash, fixture.map(|v| v.1.to_string()), fixture.map(|v| v.2.to_string()), versions, Vec::new());
    (record, result)
}

fn apply_assertions(record: &mut RawRecord, scenario: &Scenario, tool: &str, result: &CommandResult, docker: &Path, lightr: &Path, fixture: &Path, output: &Path) {
    for assertion in scenario.assertions.iter().filter(|assertion| applies(&assertion.scope, tool)) {
        record.assertions.push(evaluate_assertion(assertion, result, docker, lightr, fixture, output));
    }
    if record.outcome == "passed" && record.assertions.iter().any(|assertion| !assertion.passed) {
        for assertion in record.assertions.iter().filter(|assertion| !assertion.passed) {
            eprintln!("bench-runner: {}/{} assertion {} failed: {}", scenario.id, tool, assertion.kind, assertion.detail);
        }
        record.outcome = "failed".into();
    }
}

fn materialize_fixture(spec_path: &Path, scenario: &Scenario, project: Option<&Project>, out: &Path) -> Result<(PathBuf, String, String), String> {
    let project = project.ok_or_else(|| format!("unknown fixture project for {}", scenario.id))?;
    if project.repo != "local" { return Err(format!("unsupported project type for {}: {}", scenario.id, project.repo)); }
    let commit = project.commit.as_deref().ok_or_else(|| format!("missing fixture commit for {}", scenario.id))?;
    let fixture = scenario.fixture.as_ref().and_then(|f| f.path.as_deref()).ok_or_else(|| format!("missing fixture path for {}", scenario.id))?;
    let context = scenario.fixture.as_ref().and_then(|f| f.context.as_deref()).ok_or_else(|| format!("missing fixture context for {}", scenario.id))?;
    let fixture_path = Path::new(fixture);
    if fixture_path.is_absolute() || fixture_path.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { return Err(format!("unsafe fixture path for {}", scenario.id)); }
    let root = spec_path.parent().and_then(Path::parent).ok_or_else(|| "spec must have benchmarks parent".to_string())?;
    let verified = Command::new("git").args(["-C"]).arg(root).args(["rev-parse", "--verify", &format!("{commit}^{{commit}}")]).output().map_err(|e| format!("verify fixture commit: {e}"))?;
    if !verified.status.success() { return Err(format!("missing local fixture commit: {commit}")); }
    let destination = out.join("fixtures").join(&scenario.id);
    if destination.exists() { fs::remove_dir_all(&destination).map_err(|e| e.to_string())?; }
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let archive = Command::new("git").args(["-C"]).arg(root).args(["archive", "--format=tar", commit, fixture]).output().map_err(|e| format!("git archive: {e}"))?;
    if !archive.status.success() { return Err(format!("git archive fixture {} failed", fixture)); }
    let components = fixture_path.components().count().to_string();
    let mut tar = Command::new("tar").args(["-x", "-C"]).arg(&destination).arg("--strip-components").arg(components).stdin(Stdio::piped()).spawn().map_err(|e| format!("extract fixture: {e}"))?;
    tar.stdin.take().ok_or_else(|| "extract fixture stdin unavailable".to_string())?.write_all(&archive.stdout).map_err(|e| e.to_string())?;
    if !tar.wait().map_err(|e| e.to_string())?.success() { return Err(format!("extract fixture {} failed", fixture)); }
    if fs::read_dir(&destination).map_err(|e| e.to_string())?.next().is_none() { return Err(format!("fixture path absent at commit: {}", fixture)); }
    let context_path = Path::new(context);
    let relative_context = context_path.strip_prefix(fixture_path).map_err(|_| format!("fixture context escapes fixture path for {}", scenario.id))?;
    let materialized_context = destination.join(relative_context);
    if !materialized_context.is_dir() { return Err(format!("fixture context absent at commit: {}", context)); }
    let hash = tree_sha256(&destination)?;
    Ok((materialized_context, hash, commit.to_string()))
}

fn probe_versions(docker: &Path, lightr: &Path) -> Versions {
    let lightr_hash = fs::read(lightr).ok().map(|b| sha256(&b));
    let docker_result = run_command(Command::new(docker).args(["version", "--format", "{{json .}}"]), Duration::from_secs(TIMEOUT_SECS));
    let lightr_result = run_command(Command::new(lightr).arg("--version"), Duration::from_secs(TIMEOUT_SECS));
    if docker_result.error.is_some() || docker_result.timed_out || !docker_result.status.as_ref().is_some_and(|s| s.success()) || lightr_hash.is_none() || lightr_result.error.is_some() || lightr_result.timed_out || !lightr_result.status.as_ref().is_some_and(|s| s.success()) {
        return Versions { docker_client: None, docker_server: None, docker_api: None, lightr: None, lightr_hash, error: Some("version probe failed".into()) };
    }
    let docker_json: Value = match serde_json::from_slice(&docker_result.stdout) { Ok(v) => v, Err(_) => return Versions { docker_client: None, docker_server: None, docker_api: None, lightr: None, lightr_hash, error: Some("docker version output was not JSON".into()) } };
    let get = |path: &[&str]| path.iter().fold(&docker_json, |v, key| &v[*key]).as_str().map(str::to_string);
    let docker_client = get(&["Client", "Version"]);
    let docker_server = get(&["Server", "Version"]);
    let docker_api = get(&["Client", "ApiVersion"]);
    if docker_client.is_none() || docker_server.is_none() || docker_api.is_none() { return Versions { docker_client, docker_server, docker_api, lightr: None, lightr_hash, error: Some("docker version output missing required fields".into()) }; }
    Versions { docker_client, docker_server, docker_api, lightr: Some(String::from_utf8_lossy(&lightr_result.stdout).trim().to_string()), lightr_hash, error: None }
}

fn expand(command: &str, docker: &Path, lightr: &Path, fixture: &Path, output: &Path) -> Result<String, String> {
    let mut result = String::new();
    let bytes = command.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' { result.push(bytes[i] as char); i += 1; continue; }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') { end += 1; }
        if start == end { result.push('$'); i += 1; continue; }
        let value = match &command[start..end] {
            "DOCKER" => docker.display().to_string(), "LIGHTR" => lightr.display().to_string(), "FIXTURE_DIR" => fixture.display().to_string(), "OUTPUT_DIR" => output.display().to_string(),
            name => return Err(format!("unknown command token: ${name}")),
        };
        result.push_str(&shell_quote(&value));
        i = end;
    }
    Ok(result)
}

fn shell_quote(value: &str) -> String { format!("'{}'", value.replace('\'', "'\\''")) }

fn run_shell(command: &str, timeout: Duration, lightr_home: Option<&Path>) -> CommandResult {
    let mut shell = Command::new("sh");
    shell.args(["-c", command]);
    if let Some(home) = lightr_home { shell.env("LIGHTR_HOME", home); }
    run_command(&mut shell, timeout)
}

fn run_command(command: &mut Command, timeout: Duration) -> CommandResult {
    let started = unix_ms();
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)] unsafe { use std::os::unix::process::CommandExt; command.pre_exec(|| if libc::setpgid(0, 0) == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }); }
    let mut child = match command.spawn() { Ok(child) => child, Err(e) => return failed_result(&format!("spawn command: {e}"), started) };
    let stdout = child.stdout.take().unwrap(); let stderr = child.stderr.take().unwrap();
    let out_thread = thread::spawn(move || { let mut b = Vec::new(); let _ = stdout.take(u64::MAX).read_to_end(&mut b); b });
    let err_thread = thread::spawn(move || { let mut b = Vec::new(); let _ = stderr.take(u64::MAX).read_to_end(&mut b); b });
    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() >= deadline => { #[cfg(unix)] unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); } #[cfg(not(unix))] { let _ = child.kill(); } let status = child.wait().ok(); break (status, true); }
            Err(_error) => break (None, false),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
        }
    };
    CommandResult { status, stdout: out_thread.join().unwrap_or_default(), stderr: err_thread.join().unwrap_or_default(), timed_out, error: None, started, ended: unix_ms() }
}

fn evaluate_assertion(assertion: &Assertion, result: &CommandResult, docker: &Path, lightr: &Path, fixture: &Path, output: &Path) -> AssertionResult {
    let expected = &assertion.expected;
    let verdict: Result<bool, String> = match assertion.kind.as_str() {
        "exit_code" => Ok(result.status.as_ref().and_then(|s| s.code()) == expected.as_i64().map(|v| v as i32)),
        "stdout_exact" => Ok(String::from_utf8_lossy(&result.stdout) == expected.as_str().unwrap_or("")),
        "stdout_regex" => Regex::new(expected.as_str().unwrap_or("")).map(|r| r.is_match(&String::from_utf8_lossy(&result.stdout))).map_err(|e| e.to_string()),
        "stderr_regex" => Regex::new(expected.as_str().unwrap_or("")).map(|r| r.is_match(&String::from_utf8_lossy(&result.stderr))).map_err(|e| e.to_string()),
        "file_sha256" => assertion.path.as_deref().ok_or_else(|| "file_sha256 missing path".to_string()).and_then(|path| expand(path, docker, lightr, fixture, output)).and_then(|path| fs::read(path.trim_matches('\'')).map_err(|e| e.to_string())).map(|b| sha256(&b) == expected.as_str().unwrap_or("")),
        "http_status" => assertion.url.as_deref().ok_or_else(|| "http_status missing url".to_string()).and_then(http_status).map(|status| status == expected.as_u64().unwrap_or(0) as u16),
        "command" => assertion.command.as_deref().ok_or_else(|| "command assertion missing command".to_string()).and_then(|command| expand(command, docker, lightr, fixture, output)).map(|command| { let probe = run_shell(&command, Duration::from_secs(TIMEOUT_SECS), Some(&output.join("lightr-home"))); !probe.timed_out && probe.error.is_none() && probe.status.as_ref().and_then(|s| s.code()) == expected.as_i64().map(|v| v as i32) }),
        other => Err(format!("unknown assertion kind: {other}")),
    };
    match verdict { Ok(true) => AssertionResult { kind: assertion.kind.clone(), passed: true, detail: "matched".into() }, Ok(false) => AssertionResult { kind: assertion.kind.clone(), passed: false, detail: "mismatch".into() }, Err(e) => AssertionResult { kind: assertion.kind.clone(), passed: false, detail: e } }
}

fn http_status(url: &str) -> Result<u16, String> {
    let rest = url.strip_prefix("http://").ok_or_else(|| "http_status supports only http URLs".to_string())?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut stream = std::net::TcpStream::connect(authority).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS))).map_err(|e| e.to_string())?;
    stream.write_all(format!("GET /{} HTTP/1.0\r\nHost: {}\r\n\r\n", path, authority).as_bytes()).map_err(|e| e.to_string())?;
    let mut response = String::new(); stream.read_to_string(&mut response).map_err(|e| e.to_string())?;
    response.split_whitespace().nth(1).ok_or_else(|| "invalid HTTP response".to_string())?.parse::<u16>().map_err(|e| e.to_string())
}

fn applies(scope: &str, tool: &str) -> bool { scope == tool || scope == "docker_and_lightr" }
fn round_indices(rounds: usize) -> std::ops::Range<usize> { 0..rounds }
fn select_scenarios<'a>(scenarios: &'a [Scenario], chunk: usize, chunks: usize) -> Vec<&'a Scenario> { scenarios.iter().enumerate().filter(|(index, _)| index % chunks == chunk).map(|(_, scenario)| scenario).collect() }

fn skip_record(s: &Scenario, spec_hash: &str) -> RawRecord {
    let now = unix_ms();
    record_base(s, "skip", 0, "skipped", &failed_result("skipped", now), "", spec_hash, None, None, &Versions { docker_client: None, docker_server: None, docker_api: None, lightr: None, lightr_hash: None, error: None }, vec![AssertionResult { kind: "skip_reason".into(), passed: true, detail: s.reason.clone().unwrap_or_default() }])
}

fn record_base(s: &Scenario, tool: &str, round: usize, outcome: &str, result: &CommandResult, command: &str, spec_hash: &str, fixture_hash: Option<String>, source_commit: Option<String>, versions: &Versions, assertions: Vec<AssertionResult>) -> RawRecord {
    RawRecord { schema_version: SCHEMA_VERSION, scenario_id: s.id.clone(), availability: s.availability, tool: tool.into(), round, outcome: outcome.into(), started_at_unix_ms: result.started, ended_at_unix_ms: result.ended, elapsed_ms: result.ended.saturating_sub(result.started), timeout_secs: TIMEOUT_SECS, exit_code: result.status.as_ref().and_then(|s| s.code()), command_sha256: sha256(command.as_bytes()), stdout_sha256: sha256(&result.stdout), stderr_sha256: sha256(&result.stderr), spec_sha256: spec_hash.into(), fixture_tree_sha256: fixture_hash, source_commit, docker_client_version: versions.docker_client.clone(), docker_server_version: versions.docker_server.clone(), docker_api_version: versions.docker_api.clone(), lightr_version: versions.lightr.clone(), lightr_sha256: versions.lightr_hash.clone(), host_os: std::env::consts::OS.into(), host_arch: std::env::consts::ARCH.into(), host_kernel: host_kernel(), assertions }
}

fn failed_result(error: &str, started: u128) -> CommandResult { CommandResult { status: None, stdout: Vec::new(), stderr: Vec::new(), timed_out: false, error: Some(error.into()), started, ended: unix_ms() } }
fn write_record(file: &mut File, record: &RawRecord) -> Result<(), String> { serde_json::to_writer(&mut *file, record).map_err(|e| e.to_string())?; file.write_all(b"\n").map_err(|e| e.to_string()) }
fn sha256(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn unix_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn host_kernel() -> String { #[cfg(unix)] { unsafe { let mut value: libc::utsname = std::mem::zeroed(); if libc::uname(&mut value) == 0 { return std::ffi::CStr::from_ptr(value.release.as_ptr()).to_string_lossy().into(); } } } "unknown".into() }

fn tree_sha256(root: &Path) -> Result<String, String> {
    fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> { for entry in fs::read_dir(path).map_err(|e| e.to_string())? { let entry = entry.map_err(|e| e.to_string())?; let p = entry.path(); if entry.file_type().map_err(|e| e.to_string())?.is_dir() { collect(root, &p, files)?; } else if entry.file_type().map_err(|e| e.to_string())?.is_file() { files.push(p.strip_prefix(root).map_err(|e| e.to_string())?.to_path_buf()); } } Ok(()) }
    let mut files = Vec::new(); collect(root, root, &mut files)?; files.sort(); let mut hash = Sha256::new(); for relative in files { hash.update(relative.to_string_lossy().as_bytes()); hash.update([0]); hash.update(fs::read(root.join(relative)).map_err(|e| e.to_string())?); hash.update([0]); } Ok(format!("{:x}", hash.finalize()))
}

fn merge(input: &Path, out: &Path) -> Result<(), String> {
    let mut paths = Vec::new(); collect_jsonl(input, &mut paths)?; paths.sort(); let mut rows = Vec::new(); for path in paths { let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?; for (line, raw) in contents.lines().enumerate() { if raw.trim().is_empty() { continue; } rows.push(decode_raw(raw).map_err(|e| format!("malformed JSONL {}:{}: {e}", path.display(), line + 1))?); } } merge_rows(rows, out)
}
fn collect_jsonl(path: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> { for entry in fs::read_dir(path).map_err(|e| e.to_string())? { let entry = entry.map_err(|e| e.to_string())?; let p = entry.path(); if p.is_dir() { collect_jsonl(&p, paths)?; } else if p.extension().is_some_and(|extension| extension == "jsonl") { paths.push(p); } } Ok(()) }
fn decode_raw(raw: &str) -> Result<RawRecord, String> {
    const FIELDS: &[&str] = &["schema_version", "scenario_id", "availability", "tool", "round", "outcome", "started_at_unix_ms", "ended_at_unix_ms", "elapsed_ms", "timeout_secs", "exit_code", "command_sha256", "stdout_sha256", "stderr_sha256", "spec_sha256", "fixture_tree_sha256", "source_commit", "docker_client_version", "docker_server_version", "docker_api_version", "lightr_version", "lightr_sha256", "host_os", "host_arch", "host_kernel", "assertions"];
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or_else(|| "row is not a JSON object".to_string())?;
    for field in FIELDS { if !object.contains_key(*field) { return Err(format!("missing raw field: {field}")); } }
    let row: RawRecord = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if row.schema_version != SCHEMA_VERSION { return Err(format!("unsupported schema version: {}", row.schema_version)); }
    Ok(row)
}
fn merge_rows(rows: Vec<RawRecord>, out: &Path) -> Result<(), String> { let mut seen = BTreeSet::new(); for row in &rows { if !seen.insert((row.scenario_id.clone(), row.tool.clone(), row.round)) { return Err(format!("duplicate raw tuple: ({}, {}, {})", row.scenario_id, row.tool, row.round)); } } fs::create_dir_all(out).map_err(|e| e.to_string())?; let mut merged = File::create(out.join("merged.jsonl")).map_err(|e| e.to_string())?; let mut counts = BTreeMap::new(); for row in &rows { *counts.entry(row.outcome.clone()).or_insert(0usize) += 1; write_record(&mut merged, row)?; } serde_json::to_writer_pretty(File::create(out.join("summary.json")).map_err(|e| e.to_string())?, &json!({"schema_version": SCHEMA_VERSION, "counts": counts})).map_err(|e| e.to_string()) }

#[cfg(test)]
mod tests {
    use super::*;
    fn temp(name: &str) -> PathBuf { let path = std::env::temp_dir().join(format!("bench-runner-{name}-{}", unix_ms())); fs::create_dir_all(&path).unwrap(); path }
    fn scenario(id: &str, availability: Availability) -> Scenario { Scenario { id: id.into(), category: "build".into(), availability, reason: (availability != Availability::Supported).then(|| "reason".into()), fixture: Some(Fixture { project: "local".into(), path: Some("fixtures/x".into()), context: Some("fixtures/x".into()) }), docker: Some(ToolCommand { command: (availability == Availability::Supported).then(|| "$DOCKER x".into()) }), lightr: Some(ToolCommand { command: (availability == Availability::Supported).then(|| "$LIGHTR x".into()) }), metrics: vec!["a".into(), "b".into(), "c".into(), "d".into()], tags: vec!["build".into(), "evidence".into()], assertions: if availability == Availability::Supported { vec![Assertion { kind: "exit_code".into(), expected: json!(0), scope: "docker_and_lightr".into(), command: None, path: None, url: None }] } else { vec![] }, lightr_evidence: (availability == Availability::Supported).then(|| json!({"source": "x"})) } }
    fn corpus() -> Spec { Spec { taxonomy: Taxonomy { category: vec!["build".into()], evidence: vec!["evidence".into()] }, source_evidence: SourceEvidence { projects: vec![Project { id: "local".into(), repo: "local".into(), commit: Some("deadbeef".into()) }] }, scenarios: (0..250).map(|n| scenario(&format!("s-{n}"), Availability::Unsupported)).collect() } }
    #[test] fn duplicate_id_rejected() { let mut spec = corpus(); spec.scenarios[1].id = spec.scenarios[0].id.clone(); assert!(validate_spec(&spec).unwrap_err().contains("duplicate scenario id")); }
    #[test] fn source_order_chunks_partition_once() { let scenarios: Vec<_> = (0..9).map(|n| scenario(&format!("s-{n}"), Availability::Unsupported)).collect(); let left = select_scenarios(&scenarios, 0, 2); let right = select_scenarios(&scenarios, 1, 2); assert_eq!(left.iter().chain(right.iter()).map(|s| &s.id).collect::<BTreeSet<_>>().len(), scenarios.len()); assert_eq!(left.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["s-0", "s-2", "s-4", "s-6", "s-8"]); }
    #[test] fn supported_rounds_are_zero_based() { assert_eq!(round_indices(3).collect::<Vec<_>>(), vec![0, 1, 2]); }
    #[test] fn supported_fixture_requires_context() { let mut spec = corpus(); let mut supported = scenario("supported", Availability::Supported); supported.fixture.as_mut().unwrap().context = None; spec.scenarios[0] = supported; assert!(validate_spec(&spec).unwrap_err().contains("missing supported fixture path/context")); }
    #[test] fn invalid_chunk_args_rejected() { assert!(run(Path::new("missing"), 0, 0, 1, Path::new("/tmp/x"), Path::new("x"), Path::new("x")).is_err()); assert!(run(Path::new("missing"), 1, 1, 1, Path::new("/tmp/x"), Path::new("x"), Path::new("x")).is_err()); assert!(run(Path::new("missing"), 0, 1, 0, Path::new("/tmp/x"), Path::new("x"), Path::new("x")).is_err()); }
    #[test] fn missing_local_fixture_commit_fails() { let root = temp("missing-commit"); let spec = root.join("benchmarks/spec.yaml"); fs::create_dir_all(spec.parent().unwrap()).unwrap(); fs::write(&spec, "x").unwrap(); let scenario = scenario("x", Availability::Supported); let project = Project { id: "local".into(), repo: "local".into(), commit: Some("0000000000000000000000000000000000000000".into()) }; assert!(materialize_fixture(&spec, &scenario, Some(&project), &root.join("out")).unwrap_err().contains("missing local fixture commit")); }
    #[test] fn failed_command_is_failed() { let result = run_shell("exit 7", Duration::from_secs(1), None); assert_eq!(result.status.unwrap().code(), Some(7)); }
    #[test] fn timeout_is_typed() { let result = run_shell("sleep 1", Duration::from_millis(20), None); assert!(result.timed_out); }
    #[test] fn scenario_shell_gets_private_lightr_home() { let home = temp("lightr-home"); let command = format!("test \"$LIGHTR_HOME\" = {}", shell_quote(&home.display().to_string())); let result = run_shell(&command, Duration::from_secs(1), Some(&home)); assert!(result.status.unwrap().success()); }
    #[test] fn typed_skip_record() { let record = skip_record(&scenario("skip", Availability::Unsupported), "spec"); assert_eq!(record.tool, "skip"); assert_eq!(record.outcome, "skipped"); assert_eq!(record.round, 0); }
    #[test] fn duplicate_raw_tuple_rejected() { let root = temp("duplicate"); let record = skip_record(&scenario("skip", Availability::Unsupported), "spec"); assert!(merge_rows(vec![record.clone(), record], &root).unwrap_err().contains("duplicate raw tuple")); }
}
