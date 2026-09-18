use super::*;
use lightr_core::Digest;

fn limits() -> TreeLimits {
    TreeLimits {
        max_entries: 1024,
        max_components: 8192,
        max_text_bytes: 1024 * 1024,
    }
}
fn file(path: &str, size: u64) -> Entry {
    Entry::File {
        path: path.into(),
        mode: 0o644,
        size,
        digest: Digest::of_bytes(b"fixture"),
    }
}
fn dir(path: &str) -> Entry {
    Entry::Dir { path: path.into() }
}
fn link(path: &str, target: &str) -> Entry {
    Entry::Symlink {
        path: path.into(),
        target: target.into(),
    }
}
fn manifest(entries: Vec<Entry>, total_size: u64) -> Manifest {
    Manifest {
        version: 1,
        total_size,
        entries,
    }
}
fn check(m: &Manifest) -> io::Result<TreePlan<'_>> {
    TreePlan::validate(m, limits(), Wait::Try)
}

#[test]
fn tree_plan_empty_tree_fits_zero_budgets() {
    let m = manifest(vec![], 0);
    let plan = TreePlan::validate(
        &m,
        TreeLimits {
            max_entries: 0,
            max_components: 0,
            max_text_bytes: 0,
        },
        Wait::Try,
    )
    .unwrap();
    assert!(plan.entries().is_empty());
    assert!(plan.directories().is_empty());
    assert_eq!((plan.file_count(), plan.total_size()), (0, 0));
}

#[test]
fn tree_plan_unsorted_tree_preserves_entries_and_includes_implied_directories() {
    let m = manifest(
        vec![
            file("z/deep/data", 7),
            link("a/link", "../missing"),
            dir("z"),
            dir("empty"),
            file("a/item", 3),
        ],
        10,
    );
    let before = m.clone();
    let plan = check(&m).unwrap();
    assert_eq!(plan.directories(), &["a", "empty", "z", "z/deep"]);
    assert_eq!((plan.file_count(), plan.total_size()), (2, 10));
    assert!(std::ptr::eq(plan.entries(), m.entries.as_slice()));
    assert_eq!(plan.entries(), before.entries);
    assert_eq!(m, before);
}

#[test]
fn tree_plan_duplicate_entries_of_every_kind_are_rejected() {
    let kinds = [dir("same"), file("same", 0), link("same", "missing")];
    for a in &kinds {
        for b in &kinds {
            let m = manifest(vec![a.clone(), b.clone()], 0);
            let error = check(&m).expect_err("duplicate explicit entry accepted");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("duplicate"));
        }
    }
}

#[test]
fn tree_plan_non_directory_ancestors_are_rejected_in_all_entry_orders() {
    for ancestor in [file("a", 0), link("a", "elsewhere")] {
        let entries = [ancestor, file("a/b/leaf", 0), dir("a/b")];
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let m = manifest(order.map(|i| entries[i].clone()).to_vec(), 0);
            let error = check(&m).expect_err("non-directory ancestor accepted");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("required directory"));
        }
    }
}

#[test]
fn tree_plan_shared_implied_parents_and_sibling_prefixes_are_distinct() {
    let m = manifest(
        vec![file("a", 0), file("ab/b", 0), file("ab/c", 0), dir("ab")],
        0,
    );
    let plan = check(&m).unwrap();
    assert_eq!(plan.directories(), &["ab"]);
    assert_eq!(plan.file_count(), 3);
}

#[test]
fn tree_plan_names_are_exact_relative_components_not_normalized_paths() {
    for name in [
        "",
        "/absolute",
        "a/",
        "a//b",
        ".",
        "..",
        "a/./b",
        "a/../b",
        "a\0b",
    ] {
        let m = manifest(vec![dir(name)], 0);
        assert_eq!(
            check(&m)
                .expect_err("invalid relative components accepted")
                .kind(),
            io::ErrorKind::InvalidData,
            "{name:?}"
        );
    }
}

#[test]
fn tree_plan_preserves_unicode_case_and_unix_literal_characters() {
    // Logical names only. These distinct strings need later native collision checks.
    let names = [
        "café",
        "cafe\u{301}",
        "A",
        "a",
        r"literal\backslash",
        "C:literal",
    ];
    let m = manifest(names.iter().map(|p| file(p, 0)).collect(), 0);
    let plan = check(&m).unwrap();
    assert_eq!(
        plan.entries().iter().map(Entry::path).collect::<Vec<_>>(),
        names
    );
    assert!(plan.directories().is_empty());
}

#[test]
fn tree_plan_link_text_is_retained_without_lookup_or_normalization() {
    for target in [
        "missing",
        "../outside",
        "/absolute",
        "a/./../b",
        r"literal\target",
    ] {
        let m = manifest(vec![link("link", target)], 0);
        let plan = check(&m).unwrap();
        assert_eq!(plan.entries(), m.entries);
        assert_eq!(plan.file_count(), 0);
    }
    for target in ["", "bad\0target"] {
        assert_eq!(
            check(&manifest(vec![link("link", target)], 0))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn tree_plan_wire_lengths_count_utf8_bytes_without_claiming_native_support() {
    let maximum = "x".repeat(u16::MAX as usize);
    assert!(check(&manifest(vec![dir(&maximum)], 0)).is_ok());
    assert!(check(&manifest(vec![link("link", &maximum)], 0)).is_ok());
    for text in [format!("{maximum}x"), "é".repeat(32768)] {
        assert_eq!(
            check(&manifest(vec![dir(&text)], 0)).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            check(&manifest(vec![link("link", &text)], 0))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn tree_plan_declared_totals_and_sum_overflow_are_rejected() {
    let m = manifest(vec![file("one", 3), file("two", 4)], 6);
    assert!(check(&m)
        .expect_err("mismatched total accepted")
        .to_string()
        .contains("total differs"));
    let overflow = manifest(vec![file("one", u64::MAX), file("two", 1)], 0);
    assert!(check(&overflow)
        .unwrap_err()
        .to_string()
        .contains("sum overflow"));
    assert_eq!(
        check(&manifest(vec![file("one", u64::MAX)], u64::MAX))
            .unwrap()
            .total_size(),
        u64::MAX
    );
    assert!(check(&manifest(vec![], 1)).is_err());
}

#[test]
fn tree_plan_version_and_permission_bit_contracts_are_explicit() {
    let mut m = manifest(vec![], 0);
    m.version = 2;
    assert_eq!(check(&m).unwrap_err().kind(), io::ErrorKind::InvalidData);
    m = manifest(vec![file("one", 0)], 0);
    for mode in [0, 0o644, 0o755, 0o7777] {
        if let Entry::File { mode: actual, .. } = &mut m.entries[0] {
            *actual = mode;
        }
        assert!(check(&m).is_ok());
    }
    if let Entry::File { mode, .. } = &mut m.entries[0] {
        *mode = 0o100644;
    }
    assert!(check(&m)
        .unwrap_err()
        .to_string()
        .contains("non-permission"));
}

#[test]
fn tree_plan_caller_budgets_include_repeated_components_and_link_text() {
    let m = manifest(vec![file("a/b", 0), link("a/l", "target")], 0);
    let exact = TreeLimits {
        max_entries: 2,
        max_components: 4,
        max_text_bytes: 12,
    };
    assert!(TreePlan::validate(&m, exact, Wait::Try).is_ok());
    for bound in [
        TreeLimits {
            max_entries: 1,
            ..exact
        },
        TreeLimits {
            max_components: 3,
            ..exact
        },
        TreeLimits {
            max_text_bytes: 11,
            ..exact
        },
    ] {
        assert_eq!(
            TreePlan::validate(&m, bound, Wait::Try)
                .expect_err("work budget exceeded without failure")
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert!(consume(usize::MAX, 1, usize::MAX, "overflow").is_err());
}

#[test]
fn tree_plan_entry_cancellation_and_deadline_precede_validation() {
    use crate::store::foundation::Cancellation;
    use std::time::{Duration, Instant};
    let cancellation = Cancellation::default();
    let m = Manifest {
        version: 2,
        total_size: 1,
        entries: vec![],
    };
    let deadline = Instant::now() - Duration::from_secs(1);
    assert_eq!(
        TreePlan::validate(
            &m,
            limits(),
            Wait::Until {
                deadline,
                cancellation: &cancellation
            }
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::TimedOut
    );
    cancellation.cancel();
    assert_eq!(
        TreePlan::validate(
            &m,
            limits(),
            Wait::Until {
                deadline,
                cancellation: &cancellation
            }
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::Interrupted
    );
}

#[test]
fn tree_plan_every_checkpoint_failure_is_terminal_without_input_mutation() {
    let m = manifest(
        vec![file("b/c/file", 4), dir("empty"), link("a/link", "missing")],
        4,
    );
    let before = m.clone();
    let mut count = 0;
    TreePlan::validate_checked(&m, limits(), || {
        count += 1;
        Ok(())
    })
    .unwrap();
    assert!(count > m.entries.len());
    for stop in 1..=count {
        let mut seen = 0;
        let error = TreePlan::validate_checked(&m, limits(), || {
            seen += 1;
            if seen == stop {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "stop at checkpoint",
                ))
            } else {
                Ok(())
            }
        })
        .expect_err("checkpoint failure was ignored");
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(error.to_string(), "stop at checkpoint");
        assert_eq!(seen, stop);
        assert_eq!(m, before);
    }
}

#[test]
fn tree_plan_small_tree_results_match_independent_pairwise_oracle() {
    // A quadratic component-vector oracle, independent of the sorted-prefix index.
    let universe: Vec<_> = ["a", "a/b", "b", "ab/c"]
        .iter()
        .flat_map(|p| [dir(p), file(p, 0), link(p, "missing")])
        .collect();
    for a in &universe {
        for b in &universe {
            let pair = [a, b];
            let mut expected = a.path() != b.path();
            for (i, x) in pair.iter().enumerate() {
                let xp: Vec<_> = x.path().split('/').collect();
                let yp: Vec<_> = pair[1 - i].path().split('/').collect();
                if xp.len() < yp.len()
                    && xp.iter().zip(&yp).all(|(x, y)| x == y)
                    && !matches!(x, Entry::Dir { .. })
                {
                    expected = false;
                }
            }
            let m = manifest(vec![a.clone(), b.clone()], 0);
            assert_eq!(check(&m).is_ok(), expected, "{:?}", m.entries);
        }
    }
}
