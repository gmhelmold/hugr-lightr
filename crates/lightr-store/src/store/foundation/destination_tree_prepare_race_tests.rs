//! Races and cleanup failures for prepared anchored output.
use super::*;

#[test]
fn prepared_tree_never_adopts_foreign_file_after_representation() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("foreign", 0)]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == PrepareStep::BeforeCreate {
                fs::write(output.join("foreign"), b"keep")?;
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(
        failure.primary.unwrap().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read(output.join("foreign")).unwrap(), b"keep");
}

#[test]
fn prepared_tree_cancellation_after_first_entry_rolls_back_owned_names() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0), file("b", 0)]);
    let cancellation = Cancellation::default();
    let wait = Wait::Until {
        deadline: Instant::now() + Duration::from_secs(30),
        cancellation: &cancellation,
    };
    let mut seen = 0;

    let failure = prepare_checked(&anchor, &plan(&manifest), limits(), wait, |step, _, _| {
        if step == PrepareStep::Created {
            seen += 1;
            cancellation.cancel();
        }
        Ok(())
    })
    .err()
    .unwrap();

    assert_eq!(seen, 1);
    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::Interrupted);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn prepared_tree_replacement_is_preserved_and_cleanup_is_incomplete() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0)]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == PrepareStep::Created && path == "a" {
                fs::rename(output.join("a"), output.join("retained-owned"))?;
                fs::write(output.join("a"), b"replacement")?;
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(failure.primary.unwrap().raw_os_error(), Some(libc::EIO));
    assert_eq!(failure.cleanup.len(), 1);
    assert!(!failure.cleanup_complete);
    assert_eq!(fs::read(output.join("a")).unwrap(), b"replacement");
    assert!(output.join("retained-owned").is_file());
}

#[test]
fn prepared_tree_unknown_child_prevents_recursive_directory_cleanup() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![directory("dir")]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, path, _| {
            if step == PrepareStep::Created && path == "dir" {
                fs::write(output.join("dir/foreign"), b"preserve")?;
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(failure.primary.unwrap().raw_os_error(), Some(libc::EIO));
    assert_eq!(failure.cleanup.len(), 1);
    assert_eq!(failure.cleanup[0].raw_os_error(), Some(libc::ENOTEMPTY));
    assert!(!failure.cleanup_complete);
    assert_eq!(fs::read(output.join("dir/foreign")).unwrap(), b"preserve");
}

#[test]
fn prepared_tree_final_binding_rejects_root_replacement() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    let retained = f.parent.join("retained");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0)]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == PrepareStep::BeforeFinalValidation {
                fs::rename(&output, &retained)?;
                fs::create_dir(&output)?;
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read_dir(&retained).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn prepared_tree_final_entry_revalidation_rejects_leaf_replacement() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0)]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == PrepareStep::BeforeFinalValidation {
                fs::remove_file(output.join("a"))?;
                fs::write(output.join("a"), b"replacement")?;
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert_eq!(failure.cleanup.len(), 1);
    assert!(!failure.cleanup_complete);
    assert_eq!(fs::read(output.join("a")).unwrap(), b"replacement");
}

#[test]
fn prepared_tree_drop_best_effort_removes_uncommitted_entries() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0), directory("dir")]);

    {
        let _prepared = anchor
            .prepare_tree(&plan(&manifest), limits(), Wait::Try)
            .unwrap();
        assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    }

    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn prepared_tree_final_validation_rejects_unplanned_root_entry() {
    let f = Fixture::new();
    let output = f.parent.join("output");
    fs::create_dir(&output).unwrap();
    let observed = f.inspect(&output);
    let anchor = observed.anchor_empty(Wait::Try).unwrap();
    let manifest = manifest(vec![file("a", 0)]);

    let failure = prepare_checked(
        &anchor,
        &plan(&manifest),
        limits(),
        Wait::Try,
        |step, _, _| {
            if step == PrepareStep::BeforeFinalValidation {
                fs::write(output.join("foreign"), b"preserve")?;
            }
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert_eq!(failure.primary.unwrap().kind(), io::ErrorKind::InvalidData);
    assert!(failure.cleanup.is_empty());
    assert!(failure.cleanup_complete);
    assert_eq!(fs::read(output.join("foreign")).unwrap(), b"preserve");
    assert!(!output.join("a").exists());
}
