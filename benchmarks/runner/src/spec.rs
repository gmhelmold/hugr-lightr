use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
    Supported,
    Unsupported,
    HardwareGated,
    OutOfScope,
}

impl Availability {
    pub fn requires_reason(&self) -> bool {
        !matches!(self, Availability::Supported)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    pub id: String,
    pub category: String,
    pub availability: Availability,
    #[serde(default)]
    pub reason: Option<String>,
    pub project: String,
    pub fixture_path: String,
    pub context: String,
    pub docker_cmd: String,
    pub lightr_cmd: String,
    pub metrics: Vec<String>,
    pub tags: Vec<String>,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Assertion {
    ExitCode { expected: i32 },
    StdoutRegex { pattern: String },
    StderrRegex { pattern: String },
    FileSha256 { path: String, expected: String },
    HttpStatus { expected: u16 },
    Command { command: String, expected: i32, #[serde(default)] scope: Option<String> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Spec {
    pub scenarios: Vec<Scenario>,
}

impl Spec {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let spec: Spec = serde_yaml::from_str(&content)?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let mut ids = HashSet::new();
        const VALID_CATEGORIES: &[&str] = &[
            "build", "buildkit", "run", "compose", "network", "volume",
            "security", "resources", "logging", "registry", "health", "plugin", "swarm"
        ];

        for s in &self.scenarios {
            if !ids.insert(&s.id) {
                anyhow::bail!("duplicate scenario id: {}", s.id);
            }
            if !VALID_CATEGORIES.contains(&s.category.as_str()) {
                anyhow::bail!("invalid category '{}' for scenario {}", s.category, s.id);
            }
            if s.availability.requires_reason() && s.reason.is_none() {
                anyhow::bail!("scenario {} requires reason for availability {:?}", s.id, s.availability);
            }
            if s.metrics.len() < 4 {
                anyhow::bail!("scenario {} requires at least 4 metrics, has {}", s.id, s.metrics.len());
            }
            if s.tags.len() < 2 {
                anyhow::bail!("scenario {} requires at least 2 taxonomy tags, has {}", s.id, s.tags.len());
            }
            if s.assertions.is_empty() {
                anyhow::bail!("scenario {} requires at least one assertion", s.id);
            }
            for a in &s.assertions {
                a.validate()?;
            }
        }
        Ok(())
    }

    pub fn chunk(&self, chunk: usize, total_chunks: usize) -> Vec<&Scenario> {
        self.scenarios
            .iter()
            .enumerate()
            .filter(|(i, _)| i % total_chunks == chunk)
            .map(|(_, s)| s)
            .collect()
    }

    pub fn supported_count(&self) -> usize {
        self.scenarios.iter().filter(|s| matches!(s.availability, Availability::Supported)).count()
    }
}

impl Assertion {
    fn validate(&self) -> anyhow::Result<()> {
        match self {
            Assertion::StdoutRegex { pattern } | Assertion::StderrRegex { pattern } => {
                regex::Regex::new(pattern)?;
            }
            Assertion::FileSha256 { path, .. } => {
                if path.is_empty() {
                    anyhow::bail!("file_sha256 assertion requires non-empty path");
                }
            }
            Assertion::HttpStatus { expected } => {
                if *expected < 100 || *expected > 599 {
                    anyhow::bail!("http_status expected must be 100-599");
                }
            }
            Assertion::Command { command, .. } => {
                if command.is_empty() {
                    anyhow::bail!("command assertion requires non-empty command");
                }
            }
            _ => {}
        }
        Ok(())
    }
}