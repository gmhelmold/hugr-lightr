//! Additive SI-01 primitives (ADR-0020); not routed from any Store/CLI method.
//!
//! A parsed readiness record is data, **not** a `PreparedObject` or proof that
//! bytes are safe to reuse. Lease-bound preparation, requalification and caller
//! conversion must precede activation. The metadata writer does not replace
//! the legacy writer and requires external resource/key exclusion.

mod failure;
mod publish;
mod readiness;

pub use crate::store::foundation::{Assurance, Readiness};
pub use failure::{InstallFailure, InstallStage, InstallVisibility};
pub use publish::{install_metadata, MetadataInstall, MAX_METADATA_BYTES};
pub use readiness::{ReadinessFrame, MAX_READY_BODY};

#[cfg(test)]
mod failure_tests;
#[cfg(test)]
mod publish_tests;
#[cfg(test)]
mod readiness_tests;
