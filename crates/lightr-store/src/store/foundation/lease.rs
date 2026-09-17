//! Additive single-lease composition. Existing Store routing is unchanged.
//!
//! Each acquisition opens one private native lock handle. Borrowing a lease
//! never opens another. No upgrades, raw-handle escape, or recursive wrappers.
//! Directory IDs come from opened native handles; paths alone are not identity.
use super::lease_io::{Directory, NativeLock};
use lightr_core::Digest;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct Cancellation(AtomicBool);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// A nonblocking attempt or a finite, cooperatively cancellable wait.
/// Cancellation does not asynchronously terminate a running filesystem call.
#[derive(Clone, Copy)]
pub enum Wait<'a> {
    Try,
    Until {
        deadline: Instant,
        cancellation: &'a Cancellation,
    },
}
impl Wait<'_> {
    pub(super) fn check(self) -> io::Result<()> {
        if let Self::Until {
            deadline,
            cancellation,
        } = self
        {
            if cancellation.is_cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "lease acquisition cancelled",
                ));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "lease acquisition deadline elapsed",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn remaining(self) -> io::Result<Option<Duration>> {
        self.check()?;
        Ok(match self {
            Self::Try => None,
            Self::Until { deadline, .. } => {
                Some(deadline.saturating_duration_since(Instant::now()))
            }
        })
    }
}

/// Opens an existing, caller-managed local Store root without creating a Store
/// or installing a storage protocol. Public topology validation belongs upstream.
#[derive(Debug, Clone)]
pub struct StoreLocks(Arc<Directory>);
impl StoreLocks {
    pub fn open_existing(root: &Path) -> io::Result<Self> {
        Ok(Self(Arc::new(Directory::open(root)?)))
    }
    pub fn same_store(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
    pub fn shared(&self, wait: Wait<'_>) -> io::Result<StoreLease> {
        let guard = NativeLock::acquire(&self.0, ".gc.lock", true, wait)?;
        Ok(StoreLease {
            _guard: guard,
            domain: self.clone(),
        })
    }
    pub fn exclusive(&self, wait: Wait<'_>) -> io::Result<ExclusiveStoreLease> {
        let guard = NativeLock::acquire(&self.0, ".gc.lock", false, wait)?;
        Ok(ExclusiveStoreLease {
            _guard: guard,
            domain: self.clone(),
        })
    }
}

/// One shared Store protection, borrowed by staged work and digest/cache guards.
/// Not Clone. It cannot be unlocked explicitly while borrowed work is alive.
#[derive(Debug)]
pub struct StoreLease {
    _guard: NativeLock,
    pub(super) domain: StoreLocks,
}
impl StoreLease {
    pub fn belongs_to(&self, domain: &StoreLocks) -> bool {
        self.domain.same_store(domain)
    }
    pub(super) fn staging(&self) -> io::Result<Directory> {
        self.domain.0.child(".si01-staging")
    }

    /// Independent worker scopes borrow the same native lock; no reacquisition.
    pub fn worker(&self) -> LeaseWorker<'_> {
        LeaseWorker { lease: self }
    }
}

/// Exclusive Store capability; deliberately not convertible to a shared lease.
#[derive(Debug)]
pub struct ExclusiveStoreLease {
    _guard: NativeLock,
    domain: StoreLocks,
}
impl ExclusiveStoreLease {
    pub fn belongs_to(&self, domain: &StoreLocks) -> bool {
        self.domain.same_store(domain)
    }
}

/// A worker obtains key locks in one sorted set, or one cache leaf section.
/// Mutable borrowing excludes overlapping cache/digest sections in this worker.
/// Cross-worker nesting is forbidden by C02, not claimed globally type-proven.
#[derive(Debug)]
pub struct LeaseWorker<'a> {
    lease: &'a StoreLease,
}
impl LeaseWorker<'_> {
    pub fn digests<'a>(
        &'a mut self,
        keys: &[Digest],
        wait: Wait<'_>,
    ) -> io::Result<DigestLocks<'a>> {
        wait.check()?;
        let dir = self.lease.domain.0.child(".si01-digest-locks")?;
        let mut keys = keys.to_vec();
        keys.sort_unstable_by_key(|key| key.0);
        keys.dedup();
        let mut guards = Vec::with_capacity(keys.len());
        for key in &keys {
            guards.push(NativeLock::acquire(&dir, &key.to_hex(), false, wait)?);
        }
        Ok(DigestLocks {
            _guards: guards,
            keys,
            _borrow: std::marker::PhantomData,
        })
    }

    pub fn cache<'a>(
        &'a mut self,
        cache: &'a CacheLocks,
        wait: Wait<'_>,
    ) -> io::Result<CacheLease<'a>> {
        cache.exclusive(wait)
    }
}

#[derive(Debug)]
pub struct DigestLocks<'a> {
    _guards: Vec<NativeLock>,
    keys: Vec<Digest>,
    _borrow: std::marker::PhantomData<&'a mut ()>,
}

impl DigestLocks<'_> {
    /// The exact sorted, deduplicated order in which these locks were acquired.
    pub fn keys(&self) -> &[Digest] {
        &self.keys
    }
}

/// Cache locks belong to the shared cache, NOT to any initiating Store. Both
/// standalone cache clients and Store workers must use this identical domain.
#[derive(Debug, Clone)]
pub struct CacheLocks(Arc<Directory>);
impl CacheLocks {
    pub fn open_existing(root: &Path) -> io::Result<Self> {
        Ok(Self(Arc::new(Directory::open(root)?)))
    }
    pub fn same_cache(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
    pub fn exclusive(&self, wait: Wait<'_>) -> io::Result<CacheLease<'_>> {
        let guard = NativeLock::acquire(&self.0, ".si01-cache.lock", false, wait)?;
        Ok(CacheLease {
            _guard: guard,
            _cache: self,
        })
    }
}

#[derive(Debug)]
pub struct CacheLease<'a> {
    _guard: NativeLock,
    _cache: &'a CacheLocks,
}
