use crate::evidence::{AssertionRecord, RawRecord};
use crate::spec::{Availability, Scenario, Spec};
use clap::Args;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to benchmark spec YAML
    #[arg(long)]
    pub spec: PathBuf,
    /// Chunk index (0-based)
    #[arg(long)]
    pub chunk: usize,
    /// Total number of chunks
    #[arg(long)]
    pub chunks: usize,
    /// Number of rounds per scenario
    #[arg(long, default_value = "1")]
    pub rounds: u32,
    /// Output directory for raw JSONL
    #[arg(long)]
    pub out: PathBuf,
    /// Path to docker binary
    #[arg(long)]
    pub docker: PathBuf,
    /// Path to lightr binary
    #[arg(long)]
    pub lightr: PathBuf,
    /// Timeout per command in seconds
    #[arg(long, default_value = "300")]
    pub timeout: u64,
}

pub fn execute(args: RunArgs) -> anyhow::Result<()> {
    let spec = Spec::load(&args.spec)?;
    let scenarios = spec.chunk(args.chunk, args.chunks);

    if scenarios.is_empty() {
        println!("Chunk {} of {} has no scenarios", args.chunk, args.chunks);
        return Ok(());
    }

    std::fs::create_dir_all(&args.out)?;
    let chunk_file = args.out.join(format!("chunk-{:02}.jsonl", args.chunk));
    let mut writer = std::fs::File::create(&chunk_file)?;

    let spec_digest = spec_digest(&args.spec)?;
    let fixture_tree_digest = fixture_tree_digest(&scenarios)?;
    let source_commit = git_commit()?;
    let (docker_client, docker_server, docker_api) = docker_versions(&args.docker)?;
    let lightr_version = lightr_version(&args.lightr)?;
    let lightr_digest = binary_digest(&args.lightr)?;

    let mut any_supported_failed = false;
    let root = args
        .spec
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| anyhow::anyhow!("spec must live under benchmarks/"))?;

    for scenario in scenarios {
        if matches!(scenario.availability, Availability::Supported) {
            any_supported_failed |= run_scenario(
                &mut writer,
                scenario,
                args.rounds,
                args.timeout,
                &args.docker,
                &args.lightr,
                &spec_digest,
                &fixture_tree_digest,
                &source_commit,
                &docker_client,
                &docker_server,
                &docker_api,
                &lightr_version,
                &lightr_digest,
                root,
                &args.out,
            )?;
        } else {
            // Record typed skip for non-supported
            record_skip(
                &mut writer,
                scenario,
                args.rounds,
                &spec_digest,
                &fixture_tree_digest,
                &source_commit,
                &docker_client,
                &docker_server,
                &docker_api,
                &lightr_version,
                &lightr_digest,
            )?;
        }
    }

    if any_supported_failed {
        anyhow::bail!("one or more supported scenarios failed");
    }

    Ok(())
}

fn run_scenario(
    writer: &mut std::fs::File,
    scenario: &Scenario,
    rounds: u32,
    timeout: u64,
    docker_bin: &std::path::Path,
    lightr_bin: &std::path::Path,
    spec_digest: &str,
    fixture_tree_digest: &str,
    source_commit: &str,
    docker_client: &str,
    docker_server: &str,
    docker_api: &str,
    lightr_version: &str,
    lightr_digest: &str,
    root: &std::path::Path,
    out_dir: &std::path::Path,
) -> anyhow::Result<bool> {
    let availability = match scenario.availability {
        Availability::Supported => "supported",
        Availability::Unsupported => "unsupported",
        Availability::HardwareGated => "hardware_gated",
        Availability::OutOfScope => "out_of_scope",
    };
    // Use 0-indexed rounds to match validation expectations (0..rounds-1)
    let mut failed = false;
    for round in 0..rounds {
        let fixture_dir = root.join(&scenario.fixture.path);
        if !fixture_dir.is_dir() {
            anyhow::bail!("supported fixture is absent: {}", fixture_dir.display());
        }
        let output_dir = out_dir
            .join("outputs")
            .join(&scenario.id)
            .join(round.to_string());
        std::fs::create_dir_all(&output_dir)?;
        let context = CommandContext {
            docker: docker_bin,
            lightr: lightr_bin,
            fixture_dir: &fixture_dir,
            output_dir: &output_dir,
        };

        let mut record = RawRecord::new(
            scenario.id.clone(),
            availability.to_string(),
            "docker".to_string(),
            round,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        record.timeout_secs = timeout;
        record.start_timed();
        run_command(&mut record, &scenario.docker.command, &context, timeout)?;

        let mut lightr_record = RawRecord::new(
            scenario.id.clone(),
            availability.to_string(),
            "lightr".to_string(),
            round,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        lightr_record.timeout_secs = timeout;
        lightr_record.start_timed();
        run_command(
            &mut lightr_record,
            &scenario.lightr.command,
            &context,
            timeout,
        )?;

        apply_assertions(
            &mut record,
            &scenario.assertions,
            "docker",
            &context,
            timeout,
        )?;
        apply_assertions(
            &mut lightr_record,
            &scenario.assertions,
            "lightr",
            &context,
            timeout,
        )?;

        failed |= record.outcome != "passed" || lightr_record.outcome != "passed";
        writeln!(writer, "{}", record.to_jsonl())?;
        writeln!(writer, "{}", lightr_record.to_jsonl())?;
    }
    Ok(failed)
}

struct CommandContext<'a> {
    docker: &'a std::path::Path,
    lightr: &'a std::path::Path,
    fixture_dir: &'a std::path::Path,
    output_dir: &'a std::path::Path,
}

fn apply_assertions(
    record: &mut RawRecord,
    assertions: &[crate::spec::Assertion],
    tool: &str,
    context: &CommandContext<'_>,
    timeout: u64,
) -> anyhow::Result<()> {
    for assertion in assertions {
        let (kind, passed) = match assertion {
            crate::spec::Assertion::ExitCode { expected } => {
                ("exit_code".to_string(), record.exit_code == Some(*expected))
            }
            crate::spec::Assertion::Command {
                command,
                expected,
                scope,
            } => {
                if scope
                    .as_deref()
                    .is_some_and(|value| value != tool && value != "docker_and_lightr")
                {
                    continue;
                }
                let result = execute_command(command, context, timeout)?;
                (
                    "command".to_string(),
                    !result.timed_out && result.exit_code == Some(*expected),
                )
            }
            _ => continue,
        };
        record.assertions.push(AssertionRecord { kind, passed });
        if !passed && record.outcome == "passed" {
            record.outcome = "failed".to_string();
        }
    }
    Ok(())
}

fn record_skip(
    writer: &mut std::fs::File,
    scenario: &Scenario,
    _rounds: u32,
    spec_digest: &str,
    fixture_tree_digest: &str,
    source_commit: &str,
    docker_client: &str,
    docker_server: &str,
    docker_api: &str,
    lightr_version: &str,
    lightr_digest: &str,
) -> anyhow::Result<()> {
    // Only emit one skip record per non-supported scenario (round 0)
    let availability = match scenario.availability {
        Availability::Supported => "supported",
        Availability::Unsupported => "unsupported",
        Availability::HardwareGated => "hardware_gated",
        Availability::OutOfScope => "out_of_scope",
    };
    let mut record = RawRecord::new(
        scenario.id.clone(),
        availability.to_string(),
        "skip".to_string(),
        0,
        spec_digest.to_string(),
        fixture_tree_digest.to_string(),
        source_commit.to_string(),
        docker_client.to_string(),
        docker_server.to_string(),
        docker_api.to_string(),
        lightr_version.to_string(),
        lightr_digest.to_string(),
    );
    record.record_skip();
    // For skip records, provide valid SHA256 hashes (hash of "skip")
    let skip_hash = RawRecord::sha256_hex("skip".as_bytes());
    record.command_sha256 = skip_hash.clone();
    record.stdout_sha256 = skip_hash.clone();
    record.stderr_sha256 = skip_hash;
    writeln!(writer, "{}", record.to_jsonl())?;
    Ok(())
}

fn run_command(
    record: &mut RawRecord,
    cmd: &str,
    context: &CommandContext<'_>,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let result = execute_command(cmd, context, timeout_secs)?;
    record.finish(
        result.exit_code,
        &result.stdout,
        &result.stderr,
        result.timed_out,
        &result.command,
    );
    Ok(())
}

struct CommandResult {
    command: String,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

fn execute_command(
    cmd: &str,
    context: &CommandContext<'_>,
    timeout_secs: u64,
) -> anyhow::Result<CommandResult> {
    use std::io::Read;

    let command = expand_command(cmd, context)?;

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .env("LIGHTR_HOME", context.output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (exit_code, timed_out) =
        match wait_timeout(&mut child, std::time::Duration::from_secs(timeout_secs)) {
            Ok(Some(exit)) => (Some(exit.code().unwrap_or(-1)), false),
            Ok(None) => {
                let _ = child.kill();
                (None, true)
            }
            Err(e) => {
                let _ = child.kill();
                return Err(e.into());
            }
        };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr)?;
    }

    Ok(CommandResult {
        command,
        exit_code,
        stdout,
        stderr,
        timed_out,
    })
}

fn expand_command(command: &str, context: &CommandContext<'_>) -> anyhow::Result<String> {
    let values = [
        ("$DOCKER", context.docker),
        ("$LIGHTR", context.lightr),
        ("$FIXTURE_DIR", context.fixture_dir),
        ("$OUTPUT_DIR", context.output_dir),
    ];
    let mut expanded = command.to_string();
    for (token, path) in values {
        expanded = expanded.replace(token, &shell_quote(&path.display().to_string()));
    }
    if regex::Regex::new(r"\$[A-Za-z_][A-Za-z0-9_]*")?.is_match(&expanded) {
        anyhow::bail!("declared command contains unknown variable: {command}");
    }
    Ok(expanded)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn wait_timeout(
    child: &mut std::process::Child,
    dur: std::time::Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {
                if start.elapsed() >= dur {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(e),
        }
    }
}

fn spec_digest(path: &std::path::Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn fixture_tree_digest(scenarios: &[&Scenario]) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    for s in scenarios {
        hasher.update(s.fixture.path.as_bytes());
        hasher.update(s.fixture.context.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn git_commit() -> anyhow::Result<String> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("unknown".to_string())
    }
}

fn docker_versions(docker_bin: &std::path::Path) -> anyhow::Result<(String, String, String)> {
    let client = get_version(docker_bin, &["version", "--format", "{{.Client.Version}}"])?;
    let server = get_version(docker_bin, &["version", "--format", "{{.Server.Version}}"])?;
    let api = get_version(
        docker_bin,
        &["version", "--format", "{{.Server.APIVersion}}"],
    )?;
    Ok((client, server, api))
}

fn lightr_version(lightr_bin: &std::path::Path) -> anyhow::Result<String> {
    get_version(lightr_bin, &["--version"])
}

fn binary_digest(path: &std::path::Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn get_version(bin: &std::path::Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(bin).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("unknown".to_string())
    }
}
