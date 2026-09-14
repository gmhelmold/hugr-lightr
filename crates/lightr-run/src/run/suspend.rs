//! Retained snapshot owner. Compose must hold this object, never an engine session.

use lightr_core::{LightrError, Result};
use lightr_engine::{Engine, ExecSpec, ResumedInstance, SuspendResume, SuspendedArtifact};
use std::path::{Path, PathBuf};

/// Owns engine and its snapshot until explicit resume or cleanup.
pub struct SuspensionOwner {
    engine: Box<dyn Engine>,
    artifact: SuspendedArtifact,
    artifact_dir: PathBuf,
}

impl SuspensionOwner {
    /// Suspend before listener bind. Unsupported remains typed data for caller.
    pub fn suspend(
        engine: Box<dyn Engine>,
        spec: &ExecSpec,
        stack_dir: &Path,
        service: &str,
    ) -> Result<std::result::Result<Self, SuspendResume>> {
        let artifact_dir = stack_dir.join("services").join(service).join("snapshot");
        std::fs::create_dir_all(&artifact_dir).map_err(LightrError::Io)?;
        match engine.suspend(spec, &artifact_dir)? {
            SuspendResume::Suspended(artifact) => Ok(Ok(Self {
                engine,
                artifact,
                artifact_dir,
            })),
            unsupported @ SuspendResume::Unsupported { .. } => {
                let _ = std::fs::remove_dir_all(&artifact_dir);
                Ok(Err(unsupported))
            }
        }
    }

    pub fn artifact(&self) -> &SuspendedArtifact {
        &self.artifact
    }

    /// Release token is authority owned by engine's gate protocol. Empty tokens
    /// fail before restore; VZ verifies exact token before workload spawn.
    pub fn resume(&self, release_token: &str) -> Result<ResumedInstance> {
        if release_token.is_empty() {
            return Err(LightrError::InvalidRef(
                "snapshot release token is empty".to_string(),
            ));
        }
        self.engine.resume(&self.artifact)
    }

    /// Removes snapshot/gate artifacts after retained engine is dropped.
    pub fn cleanup(self) -> Result<()> {
        std::fs::remove_dir_all(&self.artifact_dir).map_err(LightrError::Io)
    }
}

impl Drop for SuspensionOwner {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.artifact_dir);
    }
}
