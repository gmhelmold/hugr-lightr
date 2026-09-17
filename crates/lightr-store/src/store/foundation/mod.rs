//! Additive SI-01 foundations. Existing Store routing remains unchanged.
//!
//! Readiness is persistent metadata, not proof that an object is safe to publish.
//! Leases protect staged work, not final payload readiness. Receipt publication
//! and a lease-bound PreparedObject are still separate implementation work.

mod error;
mod ready;

pub use error::{Phase, PublicationFailure, PublicationOutcome};
pub use ready::{Assurance, Readiness};

#[cfg(test)]
mod ready_tests;

mod lease;
mod lease_io;
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
