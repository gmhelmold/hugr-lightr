use super::*;
use std::{
    io::Read,
    os::unix::fs::{symlink, MetadataExt},
    path::{Path, PathBuf},
};

struct Fixture {
    root: TempDir,
    managed: PathBuf,
    public: PathBuf,
    existing: PathBuf,
    planned: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let managed = root.path().join("managed");
        let public = root.path().join("public");
        let existing = root.path().join("existing-store");
        for path in [&managed, &public, &existing] {
            fs::create_dir(path).unwrap();
        }
        fs::write(public.join("data"), b"owned stable source").unwrap();
        let planned = managed.join("new-store");
        Self {
            root,
            managed,
            public,
            existing,
            planned,
        }
    }
    fn inspect(&self, path: &Path) -> io::Result<TopologyInspection> {
        TopologyInspection::inspect_configured(
            &[ProtectedRoot::PlannedDirectory(&self.planned)],
            SourcePath::Directory(path),
            None,
            Wait::Try,
        )
    }
    fn destination(&self, path: &Path) -> io::Result<DestinationInspection> {
        DestinationInspection::inspect_configured(
            &[ProtectedRoot::PlannedDirectory(&self.planned)],
            path,
            Wait::Try,
        )
    }
}

#[test]
fn planned_roots_accept_absent_inventory_without_creating_names() {
    let f = Fixture::new();
    let before = fs::metadata(&f.public).unwrap();
    let result = f
        .inspect(&f.public)
        .expect("planned root inspection failed");
    result.revalidate(Wait::Try).unwrap();
    let opened = result.source_handle().metadata().unwrap();
    assert_eq!((opened.dev(), opened.ino()), (before.dev(), before.ino()));
    assert_eq!(fs::read_dir(&f.managed).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.public.join("data")).unwrap(),
        b"owned stable source"
    );
}

#[test]
fn planned_roots_keep_existing_directory_contract_strict() {
    let f = Fixture::new();
    rejected(
        TopologyInspection::inspect(
            &[&f.planned],
            SourcePath::Directory(&f.public),
            None,
            Wait::Try,
        ),
        io::ErrorKind::NotFound,
        "old root contract relaxed",
    );
    rejected(
        DestinationInspection::inspect(&[&f.planned], &f.public, Wait::Try),
        io::ErrorKind::NotFound,
        "old destination contract relaxed",
    );
    rejected(
        DestinationInspection::inspect_configured(
            &[ProtectedRoot::ExistingDirectory(&f.planned)],
            &f.public,
            Wait::Try,
        ),
        io::ErrorKind::NotFound,
        "explicit existing root accepted absence",
    );
    assert!(!f.planned.exists());
}

#[test]
fn planned_roots_reject_public_ancestors() {
    let f = Fixture::new();
    for public in [&f.managed, &f.root.path().to_path_buf()] {
        rejected(
            f.inspect(public),
            io::ErrorKind::InvalidInput,
            "planned root ancestor accepted",
        );
        rejected(
            f.destination(public),
            io::ErrorKind::InvalidInput,
            "planned destination ancestor accepted",
        );
    }
    assert!(!f.planned.exists());
}

#[test]
fn planned_roots_reject_equal_and_nested_missing_destinations() {
    let f = Fixture::new();
    for destination in [&f.planned, &f.planned.join("nested")] {
        rejected(
            f.destination(destination),
            io::ErrorKind::InvalidInput,
            "missing overlap accepted",
        );
        rejected(
            TopologyInspection::inspect_configured(
                &[ProtectedRoot::PlannedDirectory(&f.planned)],
                SourcePath::Directory(&f.public),
                Some(destination),
                Wait::Try,
            ),
            io::ErrorKind::InvalidInput,
            "paired missing overlap accepted",
        );
    }
    let deeper = f.planned.join("nested");
    rejected(
        DestinationInspection::inspect_configured(
            &[ProtectedRoot::PlannedDirectory(&deeper)],
            &f.planned,
            Wait::Try,
        ),
        io::ErrorKind::InvalidInput,
        "missing ancestor overlap accepted",
    );
    assert_eq!(fs::read_dir(&f.managed).unwrap().count(), 0);
}

#[test]
fn planned_roots_reject_unproven_sibling_names() {
    let f = Fixture::new();
    for name in ["NEW-STORE", "new-store-other", "é", "e\u{301}"] {
        rejected(
            f.destination(&f.managed.join(name)),
            io::ErrorKind::Unsupported,
            "unresolved sibling disjointness was guessed",
        );
    }
    assert_eq!(fs::read_dir(&f.managed).unwrap().count(), 0);
}

#[test]
fn planned_roots_allow_distinct_native_ancestors() {
    let f = Fixture::new();
    let target = f.public.join("missing/output");
    let observation = f.destination(&target).unwrap();
    assert!(observation.is_missing());
    assert!(observation.existing_directory().is_none());
    observation.revalidate(Wait::Try).unwrap();
    assert!(!target.exists());
    assert!(!f.public.join("missing").exists());
    assert!(!f.planned.exists());
}

#[test]
fn planned_roots_preserve_native_alias_parent_resolution() {
    let f = Fixture::new();
    fs::create_dir(f.managed.join("child")).unwrap();
    symlink(f.managed.join("child"), f.public.join("alias")).unwrap();
    let planned = f.public.join("alias/../new-store");
    let roots = [ProtectedRoot::PlannedDirectory(&planned)];
    rejected(
        DestinationInspection::inspect_configured(&roots, &f.managed, Wait::Try),
        io::ErrorKind::InvalidInput,
        "native alias parent was collapsed lexically",
    );
    let observation = TopologyInspection::inspect_configured(
        &roots,
        SourcePath::Directory(&f.existing),
        None,
        Wait::Try,
    )
    .unwrap();
    observation.revalidate(Wait::Try).unwrap();
    assert!(!f.planned.exists());
    assert!(!f.public.join("new-store").exists());
}

#[test]
fn planned_roots_reject_present_dangling_alias() {
    let f = Fixture::new();
    symlink(f.managed.join("absent-target"), &f.planned).unwrap();
    rejected(
        f.inspect(&f.public),
        io::ErrorKind::Unsupported,
        "dangling root treated as absence",
    );
    assert!(fs::symlink_metadata(&f.planned)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read_link(&f.planned).unwrap(),
        f.managed.join("absent-target")
    );
}

#[test]
fn planned_roots_preserve_type_and_unresolved_parent_errors() {
    let f = Fixture::new();
    fs::write(&f.planned, b"not a directory").unwrap();
    let direct = fs::read_dir(&f.planned).err().unwrap();
    let observed = f.inspect(&f.public).err().unwrap();
    assert_eq!(observed.raw_os_error(), direct.raw_os_error());
    assert_eq!(fs::read(&f.planned).unwrap(), b"not a directory");
    let unresolved = f.managed.join("missing/../other");
    rejected(
        DestinationInspection::inspect_configured(
            &[ProtectedRoot::PlannedDirectory(&unresolved)],
            &f.public,
            Wait::Try,
        ),
        io::ErrorKind::Unsupported,
        "missing-parent semantics were invented",
    );
}

#[test]
fn planned_roots_revalidation_detects_root_appearance() {
    let f = Fixture::new();
    let source = f.inspect(&f.public).unwrap();
    let destination = f.destination(&f.public).unwrap();
    fs::create_dir(&f.planned).unwrap();
    rejected(
        source.revalidate(Wait::Try),
        io::ErrorKind::InvalidData,
        "appeared root accepted as old observation",
    );
    rejected(
        destination.revalidate(Wait::Try),
        io::ErrorKind::InvalidData,
        "appeared root accepted by destination",
    );
    // A fresh observation may accept the new directory; revalidation never updates the old one.
    f.inspect(&f.public).unwrap().revalidate(Wait::Try).unwrap();
}

#[test]
fn planned_roots_revalidation_detects_parent_replacement() {
    let f = Fixture::new();
    let observation = f.destination(&f.public).unwrap();
    fs::rename(&f.managed, f.root.path().join("previous-managed")).unwrap();
    fs::create_dir(&f.managed).unwrap();
    rejected(
        observation.revalidate(Wait::Try),
        io::ErrorKind::InvalidData,
        "replaced planned ancestor accepted",
    );
    assert!(observation
        .existing_directory()
        .unwrap()
        .metadata()
        .unwrap()
        .is_dir());
}

#[test]
fn planned_roots_mixed_inventory_checks_each_root() {
    let f = Fixture::new();
    let roots = [
        ProtectedRoot::ExistingDirectory(&f.existing),
        ProtectedRoot::PlannedDirectory(&f.planned),
    ];
    for destination in [&f.existing, &f.managed] {
        rejected(
            DestinationInspection::inspect_configured(&roots, destination, Wait::Try),
            io::ErrorKind::InvalidInput,
            "one configured root was ignored",
        );
    }
    fs::create_dir(&f.planned).unwrap();
    f.destination(&f.public)
        .unwrap()
        .revalidate(Wait::Try)
        .unwrap();
    rejected(
        f.destination(&f.planned),
        io::ErrorKind::InvalidInput,
        "existing planned root lost protection",
    );
}

#[test]
fn planned_roots_preserve_inventory_bounds() {
    let f = Fixture::new();
    for roots in [
        vec![],
        vec![ProtectedRoot::PlannedDirectory(&f.planned); 33],
    ] {
        rejected(
            DestinationInspection::inspect_configured(&roots, &f.public, Wait::Try),
            io::ErrorKind::Unsupported,
            "root inventory bound bypassed",
        );
    }
    assert!(!f.planned.exists());
}

#[test]
fn planned_roots_retained_source_is_not_reopened() {
    let f = Fixture::new();
    let path = f.public.join("data");
    let observation = TopologyInspection::inspect_configured(
        &[ProtectedRoot::PlannedDirectory(&f.planned)],
        SourcePath::File(&path),
        None,
        Wait::Try,
    )
    .unwrap();
    fs::rename(&path, f.public.join("previous-data")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    let mut bytes = Vec::new();
    observation
        .source_handle()
        .try_clone()
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"owned stable source");
    rejected(
        observation.revalidate(Wait::Try),
        io::ErrorKind::InvalidData,
        "replacement source accepted",
    );
}
