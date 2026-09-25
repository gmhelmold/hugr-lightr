use super::*;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn reserved(root: &Path) -> PathBuf {
    let items = fs::read_dir(root.join(".si01-staging"))
        .unwrap()
        .map(|item| item.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 1);
    items[0].clone()
}
fn empty(root: &Path) {
    assert_eq!(fs::read_dir(root.join(".si01-staging")).unwrap().count(), 0);
}

#[test]
fn name_probe_whole_tree_preserves_exact_names_and_implicit_parents() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("sentinel"), b"original").unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![
        file("nested/deeper/data"),
        directory("empty"),
        file(".hidden"),
        Entry::Symlink {
            path: "dangling".into(),
            target: "../external/unchanged".into(),
        },
    ]);
    let original = m.clone();
    let mut seen = BTreeSet::new();
    let result = probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, path| {
        if step == Step::Created {
            assert!(seen.insert(path.to_string()));
            let entry = reserved(root.path()).join(path);
            if ["nested", "nested/deeper", "empty"].contains(&path) {
                assert!(entry.is_dir());
            } else {
                assert!(fs::symlink_metadata(entry).unwrap().is_file());
            }
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        result,
        ScratchNameObservation {
            directories: 3,
            leaf_names: 3
        }
    );
    assert_eq!(seen.len(), 6);
    assert_eq!(m, original);
    assert_eq!(fs::read(root.path().join("sentinel")).unwrap(), b"original");
    empty(root.path());
}

#[test]
fn name_probe_sibling_markers_coexist_until_all_names_are_tested() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a"), file("b"), directory("c")]);
    probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, path| {
        if step == Step::Created && path == "c" {
            let p = reserved(root.path());
            assert!(
                p.join("a").is_file() && p.join("b").is_file() && p.join("c").is_dir(),
                "sibling names were cleaned before collision checks"
            );
        }
        Ok(())
    })
    .unwrap();
    empty(root.path());
}

#[test]
fn name_probe_case_and_unicode_results_follow_native_creation_not_folding() {
    for (first, second) in [("A", "a"), ("é", "e\u{301}"), ("a", "ab")] {
        let root = TempDir::new().unwrap();
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let lease = locks.shared(Wait::Try).unwrap();
        // Independent native oracle in another newly created directory under
        // the same controlled fixture. This is not a cross-directory guarantee.
        let oracle = root.path().join("oracle");
        fs::create_dir(&oracle).unwrap();
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(oracle.join(first))
            .unwrap();
        let second_result = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(oracle.join(second));
        let m = manifest(vec![file(first), file(second)]);
        let actual = probe_scratch_names(&plan(&m), &lease, limits(), Wait::Try);
        match second_result {
            Ok(_) => assert_eq!(actual.unwrap().leaf_names, 2),
            Err(error) => {
                let actual = actual.unwrap_err();
                assert_eq!(actual.primary.unwrap().raw_os_error(), error.raw_os_error());
                assert!(actual.cleanup.is_empty());
            }
        }
        assert!(oracle.join(first).is_file());
        empty(root.path());
    }
}

#[test]
fn name_probe_same_leaf_name_in_different_parents_is_not_a_collision() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("one/data"), file("two/data"), directory("three")]);
    let result = probe_scratch_names(&plan(&m), &lease, limits(), Wait::Try).unwrap();
    assert_eq!(
        result,
        ScratchNameObservation {
            directories: 3,
            leaf_names: 2
        }
    );
    empty(root.path());
}

#[test]
fn name_probe_late_native_creation_failure_cleans_prior_allocations() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a"), file(&"z".repeat(512))]);
    let mut seen = 0;
    let failure = probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, _| {
        if step == Step::Created {
            seen += 1;
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(seen, 1);
    assert_eq!(
        failure.primary.unwrap().raw_os_error(),
        Some(libc::ENAMETOOLONG)
    );
    assert!(failure.cleanup.is_empty());
    empty(root.path());
}

#[test]
fn name_probe_post_creation_cancellation_is_cleaned_and_not_success() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a"), file("b")]);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(30),
        cancellation: &cancellation,
    };
    let mut seen = 0;
    let failure = probe_checked(&plan(&m), &lease, limits(), wait, |step, _| {
        if step == Step::Created {
            seen += 1;
            cancellation.cancel();
        }
        Ok(())
    })
    .expect_err("cancelled name probe accepted");
    assert_eq!(seen, 1);
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(failure.cleanup.is_empty());
    empty(root.path());
}

#[test]
fn name_probe_terminal_cancellation_after_cleanup_is_not_success() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a")]);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(30),
        cancellation: &cancellation,
    };
    let failure = probe_checked(&plan(&m), &lease, limits(), wait, |step, _| {
        if step == Step::AfterCleanup {
            empty(root.path());
            cancellation.cancel();
        }
        Ok(())
    })
    .expect_err("terminal cancellation accepted");
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(failure.cleanup.is_empty());
    empty(root.path());
}

#[test]
fn name_probe_primary_and_cleanup_failures_are_both_preserved() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a")]);
    let failure = probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, path| {
        if step == Step::BeforeCleanup && path.is_empty() {
            fs::write(reserved(root.path()).join("unknown"), b"retain").unwrap();
            return Err(io::Error::from_raw_os_error(libc::EIO));
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(failure.primary.unwrap().raw_os_error(), Some(libc::EIO));
    assert_eq!(failure.cleanup.len(), 1);
    assert_eq!(failure.cleanup[0].raw_os_error(), Some(libc::ENOTEMPTY));
    let p = reserved(root.path());
    assert!(!p.join("a").exists());
    assert_eq!(fs::read(p.join("unknown")).unwrap(), b"retain");
}

#[test]
fn name_probe_cleanup_failure_alone_prevents_an_observation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a")]);
    let failure = probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, path| {
        if step == Step::BeforeCleanup && path.is_empty() {
            fs::write(reserved(root.path()).join("unknown"), b"retain").unwrap();
        }
        Ok(())
    })
    .expect_err("failed cleanup accepted");
    assert!(failure.primary.is_none());
    assert_eq!(failure.cleanup.len(), 1);
    assert_eq!(
        fs::read(reserved(root.path()).join("unknown")).unwrap(),
        b"retain"
    );
}

#[test]
fn name_probe_replaced_marker_and_unknown_bytes_are_not_deleted() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let m = manifest(vec![file("a")]);
    let failure = probe_checked(&plan(&m), &lease, limits(), Wait::Try, |step, path| {
        if step == Step::Created && path == "a" {
            let p = reserved(root.path());
            fs::rename(p.join("a"), p.join("retained-owned")).unwrap();
            fs::write(p.join("a"), b"replacement").unwrap();
        }
        Ok(())
    })
    .unwrap_err();
    assert!(failure.primary.is_none());
    assert_eq!(failure.cleanup.len(), 2);
    assert_eq!(failure.cleanup[0].kind(), io::ErrorKind::InvalidData);
    let p = reserved(root.path());
    assert_eq!(fs::read(p.join("a")).unwrap(), b"replacement");
    assert!(p.join("retained-owned").is_file());
}
