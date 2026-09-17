//! Preserve original I/O errors and visibility phase through legacy wrappers.
use lightr_core::LightrError;
use std::{error::Error, fmt, io, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationOutcome {
    NotPublished,
    CommitUncertain,
    CommittedMaintenanceFailure,
    Published,
    RecoveryRequired,
    RecoveryBlockedResources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Validate,
    Lock,
    Stage,
    Verify,
    PayloadSync,
    PayloadInstall,
    PayloadDirectory,
    ReceiptSync,
    ReceiptInstall,
    ReceiptDirectory,
}

#[derive(Debug)]
pub struct PublicationFailure {
    pub operation_id: [u8; 16],
    pub outcome: PublicationOutcome,
    pub phase: Phase,
    pub relative_path: Option<PathBuf>,
    cause: io::Error,
}

impl PublicationFailure {
    pub fn new(
        operation_id: [u8; 16],
        outcome: PublicationOutcome,
        phase: Phase,
        relative_path: Option<PathBuf>,
        cause: io::Error,
    ) -> Self {
        Self {
            operation_id,
            outcome,
            phase,
            relative_path,
            cause,
        }
    }

    pub(crate) fn into_cause(self) -> io::Error {
        self.cause
    }

    pub fn original_io(&self) -> &io::Error {
        &self.cause
    }

    pub fn into_legacy(self) -> LightrError {
        LightrError::Io(io::Error::new(self.cause.kind(), self))
    }

    pub fn into_shared_legacy(self: std::sync::Arc<Self>) -> LightrError {
        LightrError::Io(io::Error::new(self.cause.kind(), self))
    }

    pub fn from_legacy(error: &LightrError) -> Option<&Self> {
        match error {
            LightrError::Io(io) => {
                let payload = io.get_ref()?;
                payload.downcast_ref::<Self>().or_else(|| {
                    payload
                        .downcast_ref::<std::sync::Arc<Self>>()
                        .map(AsRef::as_ref)
                })
            }
            _ => None,
        }
    }
}

impl fmt::Display for PublicationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.outcome, self.phase)?;
        if let Some(path) = &self.relative_path {
            write!(f, " ({})", path.display())?;
        }
        write!(f, ": {}", self.cause)
    }
}

impl Error for PublicationFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}
