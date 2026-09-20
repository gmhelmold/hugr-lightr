//! Additive SI-01 foundations. Existing Store routing remains unchanged.
//!
//! Readiness is persistent metadata, not proof that an object is safe to publish.
//! Explicit publication returns a lease-bound proof after checked payload and
//! receipt barriers. Existing Store/CLI routes do not use these additive APIs.

mod error;
pub mod metadata;
mod ready;

pub use error::{Phase, PublicationFailure, PublicationOutcome};
pub use ready::{Assurance, Readiness};

#[cfg(test)]
mod ready_tests;

mod lease;
mod lease_io;
pub(crate) use lease_io::verify_file_path;
mod leased_stage;
pub use lease::{
    CacheLease, CacheLocks, Cancellation, DigestLocks, ExclusiveStoreLease, LeaseWorker,
    StoreLease, StoreLocks, Wait,
};
pub use leased_stage::LeasedStagedFile;

#[cfg(test)]
mod lease_process_tests;
#[cfg(test)]
mod lease_tests;

mod preparation_cache;
mod publication;
mod publication_io;
pub use publication::{PreparationResult, PreparationWork, PreparedObject};

#[cfg(test)]
mod publication_concurrency_tests;
#[cfg(test)]
mod publication_failure_tests;
#[cfg(test)]
mod publication_tests;

#[cfg(test)]
mod publication_process_tests;

#[cfg(test)]
mod capture_cancellation_tests;

#[cfg(test)]
mod inspection_cancellation_tests;

#[cfg(test)]
mod inspection_waiter_tests;

pub use crate::store::cas::preparation::{CaptureMethod, CaptureMode};

/// Read-only namespace inspection; public mutation remains unactivated.
pub mod topology;

#[cfg(test)]
mod destination_tests;

/// Logical tree checks only; native representation and writing remain separate.
pub mod tree_plan;

/// Static Windows link-kind derivation; native capability and output stay separate.
pub mod windows_link_plan;

#[cfg(test)]
mod planned_roots_tests;

/// Owned, create-only staging handles; no public destination writer.
pub mod anchored_scratch;

/// Whole logical-name probing in private scratch, not actual-destination approval.
pub mod scratch_name_probe;

/// Anchored existing-or-created destination root; no descendant writer yet.
pub mod destination_anchor;
