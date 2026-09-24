use super::*;
use crate::store::foundation::{tree_plan::TreeLimits, Cancellation, StoreLocks};
use lightr_core::{Digest, Manifest};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn manifest(entries: Vec<Entry>) -> Manifest {
    Manifest {
        version: 1,
        total_size: 0,
        entries,
    }
}
fn file(path: &str) -> Entry {
    Entry::File {
        path: path.into(),
        mode: 0o644,
        size: 0,
        digest: Digest([0; 32]),
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn directory(path: &str) -> Entry {
    Entry::Dir { path: path.into() }
}
fn plan(m: &Manifest) -> TreePlan<'_> {
    TreePlan::validate(
        m,
        TreeLimits {
            max_entries: 8192,
            max_components: 100_000,
            max_text_bytes: 1_000_000,
        },
        Wait::Try,
    )
    .unwrap()
}
fn limits() -> NameProbeLimits {
    NameProbeLimits {
        max_nodes: 256,
        max_depth: 32,
    }
}

#[test]
fn name_probe_precancel_precedes_scratch_allocation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a")]);
    let cancelled = Cancellation::default();
    cancelled.cancel();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(5),
        cancellation: &cancelled,
    };
    let error = probe_scratch_names(&plan(&m), &lease, limits(), wait).unwrap_err();
    assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(!root.path().join(".si01-staging").exists());
}

#[test]
fn name_probe_expired_deadline_precedes_scratch_allocation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![]);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        probe_scratch_names(&plan(&m), &lease, limits(), wait)
            .unwrap_err()
            .primary
            .unwrap()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(!root.path().join(".si01-staging").exists());
}

#[test]
fn name_probe_whole_plan_budgets_precede_allocation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let cases = [
        (
            vec![file("a/b")],
            NameProbeLimits {
                max_nodes: 1,
                max_depth: 32,
            },
        ),
        (
            vec![file("a/b")],
            NameProbeLimits {
                max_nodes: 2,
                max_depth: 1,
            },
        ),
        (
            vec![file(&vec!["a"; 65].join("/"))],
            NameProbeLimits {
                max_nodes: 100,
                max_depth: 100,
            },
        ),
        (
            (0..4097).map(|i| file(&format!("n{i}"))).collect(),
            NameProbeLimits {
                max_nodes: 8192,
                max_depth: 32,
            },
        ),
    ];
    for (entries, bounds) in cases {
        let m = manifest(entries);
        let error = probe_scratch_names(&plan(&m), &lease, bounds, Wait::Try)
            .expect_err("name budget accepted");
        assert_eq!(error.primary.unwrap().kind(), io::ErrorKind::InvalidInput);
        assert!(error.cleanup.is_empty());
        assert!(!root.path().join(".si01-staging").exists());
    }
}

#[test]
fn name_probe_empty_tree_platform_contract_is_explicit() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![]);
    let result = probe_scratch_names(
        &plan(&m),
        &lease,
        NameProbeLimits {
            max_nodes: 0,
            max_depth: 0,
        },
        Wait::Try,
    );
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        assert_eq!(
            result.unwrap(),
            ScratchNameObservation {
                directories: 0,
                leaf_names: 0
            }
        );
        assert_eq!(
            std::fs::read_dir(root.path().join(".si01-staging"))
                .unwrap()
                .count(),
            0
        );
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        assert_eq!(
            result.unwrap_err().primary.unwrap().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(!root.path().join(".si01-staging").exists());
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "scratch_name_probe_unix_tests.rs"]
mod unix;
