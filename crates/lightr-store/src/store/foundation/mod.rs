//! SI-01 pure wire/error primitives. No storage protocol is activated here.
//!
//! Readiness is persistent metadata, not proof that an object is safe to publish.
//! Issuing a lease-bound PreparedObject and writing receipts are separate,
//! not-yet-published SI-01 work. Existing Store methods are unchanged.

pub mod metadata;
mod error;
mod ready;

pub use error::{Phase, PublicationFailure, PublicationOutcome};
pub use ready::{Assurance, Readiness};

#[cfg(test)]
mod ready_tests;
