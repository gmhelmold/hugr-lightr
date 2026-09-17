//! Lease-local single-flight. Only a completed successful result is a proof.
//! Failures are shared verbatim; retry requires a new operation/Store lease.
use super::publication::{Confirmed, PreparationResult};
use super::{Phase, PublicationFailure, PublicationOutcome, Wait};
use lightr_core::Digest;
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

type Value = PreparationResult<Confirmed>;
#[derive(Debug, Default)]
struct Entry {
    value: Mutex<Option<Value>>,
    changed: Condvar,
}
#[derive(Debug, Default)]
pub(super) struct PreparationCache {
    entries: Mutex<HashMap<Digest, Arc<Entry>>>,
}

fn failure(id: [u8; 16], cause: io::Error) -> Arc<PublicationFailure> {
    Arc::new(PublicationFailure::new(
        id,
        PublicationOutcome::NotPublished,
        Phase::Lock,
        None,
        cause,
    ))
}

impl PreparationCache {
    pub(super) fn run(
        &self,
        key: Digest,
        wait: Wait<'_>,
        id: [u8; 16],
        prepare: impl FnOnce() -> Value,
    ) -> PreparationResult<(Confirmed, bool)> {
        self.run_observed(key, wait, id, prepare, || {})
    }

    pub(super) fn run_observed(
        &self,
        key: Digest,
        wait: Wait<'_>,
        id: [u8; 16],
        prepare: impl FnOnce() -> Value,
        mut on_wait: impl FnMut(),
    ) -> PreparationResult<(Confirmed, bool)> {
        wait.check().map_err(|e| failure(id, e))?;
        let (entry, leader) = {
            let mut entries = self
                .entries
                .lock()
                .map_err(|_| failure(id, io::Error::other("preparation map poisoned")))?;
            match entries.get(&key) {
                Some(entry) => (entry.clone(), false),
                None => {
                    let entry = Arc::new(Entry::default());
                    entries.insert(key, entry.clone());
                    (entry, true)
                }
            }
        };
        if leader {
            let mut completion = Completion {
                entry: &entry,
                id,
                finished: false,
            };
            let value = prepare();
            // No user code runs with the state mutex held. Poison is handled
            // conservatively by Completion::drop so waiters are always woken.
            let mut state = entry
                .value
                .lock()
                .map_err(|_| failure(id, io::Error::other("preparation state poisoned")))?;
            *state = Some(value.clone());
            completion.finished = true;
            drop(state);
            entry.changed.notify_all();
            return value.map(|value| (value, false));
        }
        let mut state = entry
            .value
            .lock()
            .map_err(|_| failure(id, io::Error::other("preparation state poisoned")))?;
        loop {
            wait.check().map_err(|e| failure(id, e))?;
            if let Some(value) = &*state {
                return value.clone().map(|value| (value, true));
            }
            let Some(left) = wait.remaining().map_err(|e| failure(id, e))? else {
                return Err(failure(id, io::Error::from(io::ErrorKind::WouldBlock)));
            };
            on_wait();
            (state, _) = entry
                .changed
                .wait_timeout(state, left.min(Duration::from_millis(5)))
                .map_err(|_| failure(id, io::Error::other("preparation wait poisoned")))?;
        }
    }
}

struct Completion<'a> {
    entry: &'a Entry,
    id: [u8; 16],
    finished: bool,
}
impl Drop for Completion<'_> {
    fn drop(&mut self) {
        if !self.finished {
            // Waking on unwind is essential: a worker panic must not leave an
            // immortal in-flight entry. No success or rollback is invented.
            let mut value = self.entry.value.lock().unwrap_or_else(|e| e.into_inner());
            *value = Some(Err(Arc::new(PublicationFailure::new(
                self.id,
                PublicationOutcome::RecoveryRequired,
                Phase::Stage,
                None,
                io::Error::other("preparation aborted; inspect and start a new operation"),
            ))));
            drop(value);
            self.entry.changed.notify_all();
        }
    }
}
