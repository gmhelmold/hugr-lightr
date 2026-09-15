use clap::Args;
use crate::spec::{Spec, Scenario, Availability};
use crate::evidence::{RawRecord, Phase, Outcome};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};

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

    for scenario in scenarios {
        if matches!(scenario.availability, Availability::Supported) {
            run_scenario(
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
) -> anyhow::Result<()> {
    for round in 1..=rounds {
        // Warmup round (round 1)
        let phase = if round == 1 { Phase::Warmup } else { Phase::Timed };
        
        // Docker command
        let mut record = RawRecord::new(
            scenario.id.clone(),
            "docker".to_string(),
            round,
            phase,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        run_command(&mut record, &scenario.docker.command, docker_bin, timeout)?;
        writeln!(writer, "{}", record.to_jsonl())?;

        // Lightr command
        let mut record = RawRecord::new(
            scenario.id.clone(),
            "lightr".to_string(),
            round,
            phase,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        run_command(&mut record, &scenario.lightr.command, lightr_bin, timeout)?;
        writeln!(writer, "{}", record.to_jsonl())?;

        // Check assertions for timed rounds only
        if phase == Phase::Timed {
            // Note: assertions evaluated by merge/validation step
        }
    }
    Ok(())
}

fn record_skip(
    writer: &mut std::fs::File,
    scenario: &Scenario,
    rounds: u32,
    spec_digest: &str,
    fixture_tree_digest: &str,
    source_commit: &str,
    docker_client: &str,
    docker_server: &str,
    docker_api: &str,
    lightr_version: &str,
    lightr_digest: &str,
) -> anyhow::Result<()> {
    let reason = scenario.reason.as_deref().unwrap_or("unsupported");
    for round in 1..=rounds {
        let phase = if round == 1 { Phase::Warmup } else { Phase::Timed };
        let mut record = RawRecord::new(
            scenario.id.clone(),
            "docker".to_string(),
            round,
            phase,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        record.outcome = Outcome::Skipped;
        record.exit_code = None;
        writeln!(writer, "{}", record.to_jsonl())?;

        let mut record = RawRecord::new(
            scenario.id.clone(),
            "lightr".to_string(),
            round,
            phase,
            spec_digest.to_string(),
            fixture_tree_digest.to_string(),
            source_commit.to_string(),
            docker_client.to_string(),
            docker_server.to_string(),
            docker_api.to_string(),
            lightr_version.to_string(),
            lightr_digest.to_string(),
        );
        record.outcome = Outcome::Skipped;
        record.exit_code = None;
        writeln!(writer, "{}", record.to_jsonl())?;
    }
    Ok(())
}

fn run_command(
    record: &mut RawRecord,
    cmd: &str,
    _bin: &std::path::Path,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    use std::io::Read;

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    record.start_ts = start;

    // Execute command with timeout
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let timed_out = match wait_timeout(&mut child, std::time::Duration::from_secs(timeout_secs)) {
        Ok(Some(exit)) => {
            record.exit_code = Some(exit.code().unwrap_or(-1));
            false
        }
        Ok(None) => {
            let _ = child.kill();
            true
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

    record.finish(record.exit_code, &stdout, &stderr, timed_out);
    Ok(())
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
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("unknown".to_string())
    }
}

fn docker_versions(docker_bin: &std::path::Path) -> anyhow::Result<(String, String, String)> {
    let client = get_version(docker_bin, &["version", "--format", "{{.Client.Version}}"])?;
    let server = get_version(docker_bin, &["version", "--format", "{{.Server.Version}}"])?;
    let api = get_version(docker_bin, &["version", "--format", "{{.Server.APIVersion}}"])?;
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