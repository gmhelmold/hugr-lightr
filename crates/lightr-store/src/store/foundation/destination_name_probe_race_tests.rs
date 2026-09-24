//! Concurrent-name and cleanup adversarial cases for actual-destination probing.
use super::*;

#[test]
fn destination_name_probe_never_adopts_foreign_name_after_empty_check() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("foreign")]);

    let failure = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == ProbeStep::AfterPreflight {
                fs::write(output.join("foreign"), b"keep")?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(
        failure.primary.unwrap().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read(output.join("foreign")).unwrap(), b"keep");
}

#[test]
fn destination_name_probe_cancellation_cleans_created_names() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a"), file("b")]);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(30),
        cancellation: &cancellation,
    };

    let mut seen = 0;
    let failure = probe_checked(&anchor, &plan(&manifest), limits(), wait, |step, _, _| {
        if step == ProbeStep::Created {
            seen += 1;
            cancellation.cancel();
        }
        Ok(())
    })
    .unwrap_err();

    assert_eq!(seen, 1);
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn destination_name_probe_replacement_is_preserved_and_reports_incomplete_cleanup() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a")]);

    let failure = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == ProbeStep::Created && path == "a" {
                fs::rename(output.join("a"), output.join("retained-owned"))?;
                fs::write(output.join("a"), b"replacement")?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert!(failure.primary.is_none());
    assert_eq!(failure.cleanup.len(), 1);
    assert!(!failure.cleanup_complete);
    assert_eq!(fs::read(output.join("a")).unwrap(), b"replacement");
    assert!(output.join("retained-owned").is_file());
}

#[test]
fn destination_name_probe_unknown_child_blocks_recursive_cleanup() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![directory("dir")]);

    let failure = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == ProbeStep::BeforeCleanup && path == "dir" {
                fs::write(output.join("dir/foreign"), b"preserve")?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert!(failure.primary.is_none());
    assert_eq!(failure.cleanup.len(), 1);
    assert_eq!(failure.cleanup[0].raw_os_error(), Some(libc::ENOTEMPTY));
    assert!(!failure.cleanup_complete);
    assert_eq!(fs::read(output.join("dir/foreign")).unwrap(), b"preserve");
}

#[test]
fn destination_name_probe_final_empty_check_detects_foreign_root_entry() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("temporary")]);

    let failure = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == ProbeStep::AfterCleanup {
                fs::write(output.join("foreign"), b"keep")?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(
        failure.primary.unwrap().raw_os_error(),
        Some(libc::ENOTEMPTY)
    );
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read(output.join("foreign")).unwrap(), b"keep");
}

#[test]
fn destination_name_probe_final_binding_rejects_root_replacement() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let retained = f.parent.join("retained");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("temporary")]);

    let failure = probe_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == ProbeStep::AfterCleanup {
                fs::rename(&output, &retained)?;
                fs::create_dir(&output)?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert!(retained.is_dir());
    assert!(output.is_dir());
    assert_eq!(fs::read_dir(&retained).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}
