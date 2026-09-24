use super::*;
use crate::store::foundation::{tree_plan::TreeLimits, Cancellation};
use lightr_core::{Digest, Manifest};
use std::time::{Duration, Instant};

fn file(path: &str) -> Entry {
    Entry::File {
        path: path.into(),
        mode: 0o644,
        size: 0,
        digest: Digest::of_bytes(b""),
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
fn manifest(entries: Vec<Entry>) -> Manifest {
    Manifest {
        version: 1,
        total_size: 0,
        entries,
    }
}
fn plan(m: &Manifest) -> TreePlan<'_> {
    TreePlan::validate(
        m,
        TreeLimits {
            max_entries: 100,
            max_components: 1000,
            max_text_bytes: 200000,
        },
        Wait::Try,
    )
    .unwrap()
}
fn rejected(entries: Vec<Entry>, reason: RepresentationReason, index: usize) {
    let m = manifest(entries);
    let tree = plan(&m);
    let e = WindowsLinkPlan::validate(&tree, Wait::Try)
        .expect_err("unsupported representation accepted");
    assert_eq!(e.kind(), io::ErrorKind::Unsupported);
    let detail = e
        .get_ref()
        .unwrap()
        .downcast_ref::<UnsupportedRepresentation>()
        .unwrap();
    assert_eq!(detail.reason, reason);
    assert_eq!(detail.entry_index, index);
}

#[test]
fn windows_link_plan_derives_file_explicit_implied_and_root_directory_kinds() {
    let m = manifest(vec![
        link("lf", "data"),
        link("ld", "empty"),
        link("li", "sub"),
        link("lr", "."),
        file("sub/item"),
        file("data"),
        dir("empty"),
    ]);
    let tree = plan(&m);
    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
    assert_eq!(
        output.links().iter().map(|x| x.kind).collect::<Vec<_>>(),
        [
            LinkKind::File,
            LinkKind::Directory,
            LinkKind::Directory,
            LinkKind::Directory
        ]
    );
    assert!(std::ptr::eq(output.tree(), &tree));
}

#[test]
fn windows_link_plan_relative_parent_segments_preserve_original_target_text() {
    let m = manifest(vec![
        file("data"),
        link("sub/ref", "./../sub/../data"),
        link("sub/root", ".."),
    ]);
    let original = m.clone();
    let tree = plan(&m);
    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
    assert_eq!(output.links()[0].kind, LinkKind::File);
    assert_eq!(output.links()[1].kind, LinkKind::Directory);
    assert_eq!(output.links()[0].target, "./../sub/../data");
    if let Entry::Symlink { path, target } = &m.entries[1] {
        assert!(std::ptr::eq(output.links()[0].path.as_ptr(), path.as_ptr()));
        assert!(std::ptr::eq(
            output.links()[0].target.as_ptr(),
            target.as_ptr()
        ));
    } else {
        panic!("fixture kind");
    }
    assert_eq!(m, original);
}

#[test]
fn windows_link_plan_missing_intermediate_or_final_targets_are_not_guessed() {
    for target in ["absent", "absent/../data", "sub/absent", "sub/absent/.."] {
        rejected(
            vec![file("data"), dir("sub"), link("ref", target)],
            RepresentationReason::MissingTarget,
            2,
        );
    }
}

#[test]
fn windows_link_plan_chains_self_cycles_and_link_before_parent_are_rejected() {
    for target in ["alias", "alias/../data", "alias/.", "alias/child"] {
        rejected(
            vec![file("data"), link("alias", "data"), link("ref", target)],
            RepresentationReason::LinkTraversal,
            2,
        );
    }
    rejected(
        vec![link("self", "self")],
        RepresentationReason::LinkTraversal,
        0,
    );
    rejected(
        vec![link("a", "b"), link("b", "a")],
        RepresentationReason::LinkTraversal,
        0,
    );
}

#[test]
fn windows_link_plan_file_components_cannot_be_walked_through() {
    for target in ["data/.", "data/..", "data/../sub", "data/child"] {
        rejected(
            vec![file("data"), dir("sub"), link("ref", target)],
            RepresentationReason::NonDirectoryTraversal,
            2,
        );
    }
}

#[test]
fn windows_link_plan_parent_resolution_never_leaves_captured_root() {
    rejected(
        vec![file("data"), link("ref", "../data")],
        RepresentationReason::OutsideTree,
        1,
    );
    rejected(
        vec![file("data"), link("sub/ref", "../../data")],
        RepresentationReason::OutsideTree,
        1,
    );
    rejected(
        vec![file("data"), link("sub/ref", "../sub/../../data")],
        RepresentationReason::OutsideTree,
        1,
    );
}

#[test]
fn windows_link_plan_rejects_nonportable_target_syntax_without_rewriting() {
    for target in [
        "/data",
        "//host/share",
        "C:data",
        "C:/data",
        r"sub\data",
        r"\??\data",
        "sub//data",
        "sub/",
    ] {
        rejected(
            vec![file("sub/data"), link("ref", target)],
            RepresentationReason::TargetSyntax,
            1,
        );
    }
}

#[test]
fn windows_link_plan_checks_names_of_every_entry_even_without_links() {
    for name in [
        r"a\b", "a:b", "a?b", "a*b", "a<b", "a>b", "a|b", "a\"b", "space ", "dot.", "a\u{1f}b",
    ] {
        rejected(
            vec![file("ordinary"), file(name)],
            RepresentationReason::ComponentSyntax,
            1,
        );
    }
    rejected(
        vec![dir("bad./child")],
        RepresentationReason::ComponentSyntax,
        0,
    );
}

#[test]
fn windows_link_plan_reserved_devices_and_extensions_are_rejected() {
    for name in [
        "CON",
        "prn.txt",
        "Aux.tar.gz",
        "NUL",
        "com1",
        "LPT9.log",
        "COM¹",
        "lpt².txt",
        "COM³",
        "NUL .txt",
        "CONIN$",
        "CONOUT$",
        "COM0",
        "LPT0",
    ] {
        rejected(vec![file(name)], RepresentationReason::ReservedName, 0);
    }
    let m = manifest(vec![
        file("com10"),
        file("console"),
        file(".hidden"),
        file("COM四"),
        file("COM¹more"),
    ]);
    assert!(WindowsLinkPlan::validate(&plan(&m), Wait::Try).is_ok());
}

#[test]
fn windows_link_plan_exact_case_and_unicode_do_not_claim_native_alias_qualification() {
    rejected(
        vec![file("Data"), link("ref", "data")],
        RepresentationReason::MissingTarget,
        1,
    );
    rejected(
        vec![file("é"), link("ref", "e\u{301}")],
        RepresentationReason::MissingTarget,
        1,
    );
    let m = manifest(vec![
        file("Data"),
        file("data"),
        file("é"),
        file("e\u{301}"),
        link("ref", "é"),
    ]);
    let tree = plan(&m);
    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
    assert_eq!(output.links()[0].kind, LinkKind::File);
    assert_eq!(output.links()[0].target, "é");
    // This pass preserves logical distinctions; a native collision probe MUST
    // reject this tree on a volume where any pair actually aliases.
    assert_eq!(output.tree().entries(), m.entries);
}

#[test]
fn windows_link_plan_empty_tree_and_long_names_remain_static_only() {
    let empty = manifest(vec![]);
    assert!(WindowsLinkPlan::validate(&plan(&empty), Wait::Try)
        .unwrap()
        .links()
        .is_empty());
    let long = "é".repeat(1000);
    let m = manifest(vec![file(&long), link("ref", &long)]);
    let tree = plan(&m);
    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
    assert_eq!(output.links()[0].target, long);
    // Wire validity and a derived kind are NOT a native filename-length pass.
}

#[test]
fn windows_link_plan_all_entry_orders_preserve_link_order_and_kinds() {
    let original = [
        file("data"),
        file("sub/child"),
        link("lf", "data"),
        link("ld", "sub"),
    ];
    let mut count = 0;
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let order = [a, b, c, d];
                    if (0..4).any(|i| order[i + 1..].contains(&order[i])) {
                        continue;
                    }
                    let m = manifest(order.map(|i| original[i].clone()).to_vec());
                    let tree = plan(&m);
                    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
                    let expected: Vec<_> = m
                        .entries
                        .iter()
                        .filter_map(|e| match e {
                            Entry::Symlink { path, target } => Some(PlannedLink {
                                path,
                                target,
                                kind: if path == "lf" {
                                    LinkKind::File
                                } else {
                                    LinkKind::Directory
                                },
                            }),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(output.links(), expected);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24);
}

#[test]
fn windows_link_plan_every_checkpoint_is_terminal_and_preserves_input() {
    let m = manifest(vec![file("data"), link("sub/ref", "../data")]);
    let original = m.clone();
    let tree = plan(&m);
    let mut count = 0;
    WindowsLinkPlan::validate_checked(&tree, || {
        count += 1;
        Ok(())
    })
    .unwrap();
    for stop in 1..=count {
        let mut called = 0;
        let error = WindowsLinkPlan::validate_checked(&tree, || {
            called += 1;
            if called == stop {
                Err(io::Error::other("checkpoint witness"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "checkpoint witness");
        assert_eq!(called, stop);
        assert_eq!(m, original);
    }
    let cancellation = Cancellation::default();
    let expired = Wait::Until {
        deadline: Instant::now() - Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        WindowsLinkPlan::validate(&tree, expired)
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    cancellation.cancel();
    let stopped = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(1),
        cancellation: &cancellation,
    };
    assert_eq!(
        WindowsLinkPlan::validate(&tree, stopped)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
}

#[cfg(windows)]
#[path = "windows_link_native_tests.rs"]
mod native;
