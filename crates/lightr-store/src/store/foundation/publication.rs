//! Additive C03 publication on stable, caller-managed local Store directories.
//! Not wired into Store/CLI routing; callers must not mix legacy writers.
use super::publication_io::Layout;
use super::{
    Assurance, LeasedStagedFile, Phase, PublicationFailure, PublicationOutcome, Readiness,
    StoreLease, Wait,
};
use lightr_core::Digest;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;

/// Shared failure preserves the original OS cause and the leader's operation
/// identity for every waiter. Failed entries can only be retried in a NEW lease.
pub type PreparationResult<T> = Result<T, Arc<PublicationFailure>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationWork {
    Created,
    Requalified,
    Reused,
    LeaseCached,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Confirmed {
    digest: Digest,
    length: u64,
    assurance: Assurance,
    work: PreparationWork,
}

/// Proof minted only after payload/receipt barriers, tied to the ACTUAL lease.
/// Copying its digest does not extend that protection. No public constructor.
///
/// ```compile_fail,E0505
/// use lightr_store::store::foundation::{StoreLocks, Wait};
/// use std::{path::Path, io::Cursor};
/// fn cannot_release(root: &Path) {
///     let locks = StoreLocks::open_existing(root).unwrap();
///     let lease = locks.shared(Wait::Try).unwrap();
///     let proof = lease.prepare_reader(&mut Cursor::new(b"bytes"), None, Wait::Try, [0;16]).unwrap();
///     drop(lease);
///     assert_eq!(proof.len(), 5);
/// }
/// ```
#[derive(Debug)]
pub struct PreparedObject<'a> {
    confirmed: Confirmed,
    _lease: &'a StoreLease,
}
impl PreparedObject<'_> {
    pub fn digest(&self) -> Digest {
        self.confirmed.digest
    }
    pub fn len(&self) -> u64 {
        self.confirmed.length
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn assurance(&self) -> Assurance {
        self.confirmed.assurance
    }
    /// Only the single leader with Created adds a new CAS object. Waiters report
    /// LeaseCached rather than multiplying counts by the number of consumers.
    pub fn work(&self) -> PreparationWork {
        self.confirmed.work
    }
}

type Observer<'a> = &'a dyn Fn(Phase, &Path) -> io::Result<()>;
/// Scoped syscall seams are private; normal publication always uses sync_all.
#[derive(Clone, Copy)]
pub(super) struct Effects<'a> {
    pub(super) observer: Observer<'a>,
    pub(super) file_sync: &'a dyn Fn(&File) -> io::Result<()>,
    pub(super) dir_sync: &'a dyn Fn(&File) -> io::Result<()>,
}
impl<'a> Effects<'a> {
    pub(super) fn new(observer: Observer<'a>) -> Self {
        Self {
            observer,
            file_sync: &File::sync_all,
            dir_sync: &File::sync_all,
        }
    }
}

fn no_fault(_: Phase, _: &Path) -> io::Result<()> {
    Ok(())
}

fn failure(
    id: [u8; 16],
    phase: Phase,
    outcome: PublicationOutcome,
    path: &Path,
    cause: io::Error,
) -> Arc<PublicationFailure> {
    Arc::new(PublicationFailure::new(
        id,
        outcome,
        phase,
        Some(path.to_path_buf()),
        cause,
    ))
}

impl StoreLease {
    /// Captures this invocation's bytes before consulting the lease-local map.
    /// The reader is supplied/validated by the caller; this is not public path
    /// preflight or CoW. Synchronous filesystem calls cannot be preempted.
    pub fn prepare_reader<'a>(
        &'a self,
        reader: &mut impl Read,
        expected: Option<(Digest, u64)>,
        wait: Wait<'_>,
        id: [u8; 16],
    ) -> PreparationResult<PreparedObject<'a>> {
        wait.check().map_err(|e| {
            failure(
                id,
                Phase::Stage,
                PublicationOutcome::NotPublished,
                Path::new(".si01-staging"),
                e,
            )
        })?;
        LeasedStagedFile::copy(self, reader, expected, id)
            .map_err(Arc::new)?
            .publish(wait, id)
    }

    /// Resolve a CAS digest; without a valid receipt it must be read, checked,
    /// fully copied to NEW staging, sealed and installed before issuing proof.
    pub fn prepare_existing(
        &self,
        digest: Digest,
        wait: Wait<'_>,
        id: [u8; 16],
    ) -> PreparationResult<PreparedObject<'_>> {
        self.prepare_with(digest, None, wait, id, &no_fault)
    }

    pub(super) fn prepare_with<'a>(
        &'a self,
        digest: Digest,
        stage: Option<LeasedStagedFile<'a>>,
        wait: Wait<'_>,
        id: [u8; 16],
        observer: Observer<'_>,
    ) -> PreparationResult<PreparedObject<'a>> {
        self.prepare_with_effects(digest, stage, wait, id, Effects::new(observer))
    }

    pub(super) fn prepare_with_effects<'a>(
        &'a self,
        digest: Digest,
        stage: Option<LeasedStagedFile<'a>>,
        wait: Wait<'_>,
        id: [u8; 16],
        effects: Effects<'_>,
    ) -> PreparationResult<PreparedObject<'a>> {
        if stage
            .as_ref()
            .is_some_and(|s| !std::ptr::eq(s._lease, self) || s.digest() != digest)
        {
            return Err(failure(
                id,
                Phase::Validate,
                PublicationOutcome::NotPublished,
                Path::new(".si01-staging"),
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "stage belongs to another lease or digest",
                ),
            ));
        }
        let (mut confirmed, cached) = self.preparations.run(digest, wait, id, || {
            publish(self, digest, stage, wait, id, effects)
        })?;
        if cached {
            confirmed.work = PreparationWork::LeaseCached;
        }
        Ok(PreparedObject {
            confirmed,
            _lease: self,
        })
    }
}
impl<'a> LeasedStagedFile<'a> {
    pub fn publish(self, wait: Wait<'_>, id: [u8; 16]) -> PreparationResult<PreparedObject<'a>> {
        self._lease
            .prepare_with(self.digest(), Some(self), wait, id, &no_fault)
    }
}

struct Attempt<'a> {
    id: [u8; 16],
    wait: Wait<'a>,
    effects: Effects<'a>,
}
impl Attempt<'_> {
    fn step<T>(
        &self,
        phase: Phase,
        outcome: PublicationOutcome,
        path: &Path,
        action: impl FnOnce() -> io::Result<T>,
    ) -> PreparationResult<T> {
        let cancelled = if outcome == PublicationOutcome::CommitUncertain {
            outcome
        } else {
            PublicationOutcome::NotPublished
        };
        self.wait
            .check()
            .map_err(|e| failure(self.id, phase, cancelled, path, e))?;
        (self.effects.observer)(phase, path)
            .and_then(|()| action())
            .map_err(|e| failure(self.id, phase, outcome, path, e))
    }
}

fn publish<'a>(
    lease: &'a StoreLease,
    digest: Digest,
    mut stage: Option<LeasedStagedFile<'a>>,
    wait: Wait<'_>,
    id: [u8; 16],
    effects: Effects<'_>,
) -> PreparationResult<Confirmed> {
    use PublicationOutcome::{CommitUncertain, NotPublished, RecoveryRequired};
    let at = Attempt { id, wait, effects };
    let mut worker = lease.worker();
    let _lock = at.step(
        Phase::Lock,
        NotPublished,
        Path::new(".si01-digest-locks"),
        || worker.digests(&[digest], wait),
    )?;
    let layout = at.step(
        Phase::Validate,
        NotPublished,
        lease.directory().path(),
        || Layout::open(lease, digest),
    )?;
    let target = layout.payload_path();
    let (existing, ready, old_length) =
        at.step(Phase::Verify, RecoveryRequired, &target, || {
            layout.inspect(digest)
        })?;
    let work;
    let length;
    if ready {
        // Immutable confirmed object: NEVER rename another file over it.
        length = old_length;
        work = PreparationWork::Reused;
        if stage.as_ref().is_some_and(|s| s.len() != length) {
            return Err(failure(
                id,
                Phase::Verify,
                RecoveryRequired,
                &target,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "staged and confirmed lengths differ",
                ),
            ));
        }
    } else {
        work = if existing.is_some() {
            PreparationWork::Requalified
        } else {
            PreparationWork::Created
        };
        if stage.is_none() {
            let mut input = existing.ok_or_else(|| {
                failure(
                    id,
                    Phase::Verify,
                    RecoveryRequired,
                    &target,
                    io::Error::from(io::ErrorKind::NotFound),
                )
            })?;
            stage = Some(
                LeasedStagedFile::copy(lease, &mut input, Some((digest, old_length)), id)
                    .map_err(Arc::new)?,
            );
        }
        let staged = stage
            .as_mut()
            .expect("staging established or returned error");
        let source = staged.stage.staged_path();
        at.step(Phase::Verify, NotPublished, &source, || {
            staged._parent.verify()?;
            staged.stage.verify_again()
        })?;
        length = staged.len();
        at.step(Phase::PayloadSync, NotPublished, &source, || {
            staged.stage.sync_for_install(true, at.effects.file_sync)
        })?;
        at.step(Phase::PayloadInstall, CommitUncertain, &target, || {
            layout.payload.verify()?;
            staged.stage.install(&target)
        })?;
        at.step(Phase::PayloadDirectory, CommitUncertain, &target, || {
            layout.sync_payload(at.effects.dir_sync)
        })?;
    }
    // A prior receipt may be visible after its last barrier failed. Reconstruct
    // it from verified immutable payload into a fresh private file every time
    // a different lease adopts it. No payload fsync-only retry is performed.
    let receipt = Readiness::new(digest, length, Assurance::native());
    let ready_path = layout.receipt_path();
    let mut record = at.step(Phase::ReceiptSync, CommitUncertain, &ready_path, || {
        layout.new_receipt(&receipt, id, at.effects.file_sync)
    })?;
    at.step(Phase::ReceiptInstall, CommitUncertain, &ready_path, || {
        layout.receipt.verify()?;
        record.install(&ready_path)
    })?;
    at.step(
        Phase::ReceiptDirectory,
        CommitUncertain,
        &ready_path,
        || layout.sync_receipt(at.effects.dir_sync),
    )?;
    Ok(Confirmed {
        digest,
        length,
        assurance: Assurance::native(),
        work,
    })
}
