//! Live-file capture only. Unconfirmed CAS requalification never calls this.
use super::OwnedTemp;
use super::StagedFile;
use crate::store::cow::CowRung;
use crate::store::foundation::{Phase, PublicationFailure, PublicationOutcome};
use lightr_core::Digest;
use std::fs::{self, File};
use std::io::{self, Seek};
use std::path::Path;

#[path = "file_clone.rs"]
mod native;

/// Explicitly force ordinary byte copy or attempt one native clone first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    CloneOrCopy,
    CopyOnly,
}

/// The operation actually used, not the Store's earlier capability probe.
#[derive(Debug)]
pub enum CaptureMethod {
    Native(CowRung),
    Copy { clone_error: Option<io::Error> },
}

type CloneFile = fn(&File, &Path, &dyn Fn() -> io::Result<()>) -> io::Result<(File, CowRung)>;
struct CaptureEffects<'a> {
    clone_file: CloneFile,
    sync: &'a dyn Fn(&File) -> io::Result<()>,
    checkpoint: &'a dyn Fn() -> io::Result<()>,
}

impl StagedFile {
    /// Source ownership is transferred; fallback reads from offset zero.
    /// Caller must supply a regular live file, not unconfirmed CAS data, and
    /// must validate source topology before calling this inactive primitive.
    pub(crate) fn capture_checked(
        parent: &Path,
        source: File,
        mode: CaptureMode,
        expected: Option<(Digest, u64)>,
        id: [u8; 16],
        checkpoint: &dyn Fn() -> io::Result<()>,
    ) -> Result<(Self, CaptureMethod), PublicationFailure> {
        Self::capture_with(
            parent,
            source,
            mode,
            expected,
            id,
            CaptureEffects {
                clone_file: native::clone_file,
                sync: &File::sync_all,
                checkpoint,
            },
        )
    }

    fn capture_with(
        parent: &Path,
        mut source: File,
        mode: CaptureMode,
        expected: Option<(Digest, u64)>,
        id: [u8; 16],
        effects: CaptureEffects<'_>,
    ) -> Result<(Self, CaptureMethod), PublicationFailure> {
        let fail = |phase, cause| {
            PublicationFailure::new(
                id,
                PublicationOutcome::NotPublished,
                phase,
                Some("payload".into()),
                cause,
            )
        };
        (effects.checkpoint)().map_err(|e| fail(Phase::Stage, e))?;
        let meta = source.metadata().map_err(|e| fail(Phase::Stage, e))?;
        if !meta.is_file() {
            return Err(fail(
                Phase::Stage,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "capture requires a regular live file",
                ),
            ));
        }
        let mut clone_error = None;
        if mode == CaptureMode::CloneOrCopy && meta.len() != 0 {
            let owned = OwnedTemp::new(parent, "clone").map_err(|e| fail(Phase::Stage, e))?;
            (effects.checkpoint)().map_err(|e| fail(Phase::Stage, e))?;
            match (effects.clone_file)(&source, &owned.payload(), effects.checkpoint) {
                Ok((file, rung)) => {
                    (effects.checkpoint)().map_err(|e| fail(Phase::Stage, e))?;
                    let stage = Self::finish_capture(
                        owned,
                        file,
                        meta.len(),
                        expected,
                        id,
                        effects.sync,
                        effects.checkpoint,
                    )?;
                    return Ok((stage, CaptureMethod::Native(rung)));
                }
                Err(error) if native::can_fallback(&error) => {
                    // Discard only this failed reservation. Fallback gets a NEW
                    // exclusive file; partial clone data is never reused.
                    match fs::remove_file(owned.payload()) {
                        Ok(()) => {}
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(fail(Phase::Stage, e)),
                    }
                    owned.finish().map_err(|e| fail(Phase::Stage, e))?;
                    clone_error = Some(error);
                }
                // I/O, space/quota, permission and interrupted clone failures
                // stop the attempt. They are not unsupported-capability probes.
                Err(error) => return Err(fail(Phase::Stage, error)),
            }
        }
        (effects.checkpoint)().map_err(|e| fail(Phase::Stage, e))?;
        source.rewind().map_err(|e| fail(Phase::Stage, e))?;
        let stage = Self::copy_with_checkpoints(
            parent,
            &mut source,
            expected,
            id,
            effects.sync,
            effects.checkpoint,
        )?;
        Ok((stage, CaptureMethod::Copy { clone_error }))
    }
}

#[cfg(test)]
#[path = "file_capture_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "file_capture_edge_tests.rs"]
mod edge_tests;
