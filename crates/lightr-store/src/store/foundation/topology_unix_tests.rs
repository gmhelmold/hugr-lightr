//! Disposable, test-owned namespace observations. No live Store or user paths.
use super::*;
use std::io::Read;
use std::os::unix::{
    ffi::OsStringExt,
    fs::{symlink, MetadataExt},
};
use std::path::{Path, PathBuf};

fn fixture() -> (TempDir, PathBuf, PathBuf) {
    let root = TempDir::new().unwrap();
    let store = root.path().join("store");
    let source = root.path().join("source");
    fs::create_dir(&store).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(store.join("sentinel"), b"protected bytes").unwrap();
    (root, store, source)
}
fn inspect(store: &Path, source: &Path, dest: Option<&Path>) -> io::Result<TopologyInspection> {
    TopologyInspection::inspect(&[store], SourcePath::Directory(source), dest, Wait::Try)
}
fn id(file: &std::fs::File) -> (u64, u64) {
    let m = file.metadata().unwrap();
    (m.dev(), m.ino())
}

#[test]
fn topology_rejects_equal_containing_and_contained_protected_roots() {
    let (root, store, _source) = fixture();
    let inner = store.join("inner");
    fs::create_dir(&inner).unwrap();
    for source in [&store, &inner, &root.path().to_path_buf()] {
        let error = inspect(&store, source, None)
            .err()
            .expect("protected overlap was accepted");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("protected namespace"));
    }
    assert_eq!(
        fs::read(store.join("sentinel")).unwrap(),
        b"protected bytes"
    );
}

#[test]
fn topology_sibling_prefix_is_not_ancestry_and_all_roots_are_checked() {
    let (root, store, source) = fixture();
    let sibling = root.path().join("store-archive");
    fs::create_dir(&sibling).unwrap();
    inspect(&store, &sibling, None).unwrap();
    let error = TopologyInspection::inspect(
        &[&store, &source],
        SourcePath::Directory(&source),
        None,
        Wait::Try,
    )
    .err()
    .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn topology_rejects_source_destination_overlap_in_both_directions() {
    let (_root, store, source) = fixture();
    let inner = source.join("inner");
    fs::create_dir(&inner).unwrap();
    for (a, b) in [(&source, &source), (&source, &inner), (&inner, &source)] {
        let error = inspect(&store, a, Some(b))
            .err()
            .expect("paired overlap was accepted");
        assert!(error.to_string().contains("source and destination overlap"));
    }
    assert!(inspect(&store, &source, Some(&source.join("not-created"))).is_err());
    assert!(!source.join("not-created").exists());
}

#[test]
fn topology_directory_alias_uses_native_identity_not_its_spelling() {
    let (root, store, source) = fixture();
    let alias = root.path().join("alias");
    symlink(&source, &alias).unwrap();
    let resolved = inspect(&store, &alias, None).unwrap();
    assert_eq!(
        id(resolved.source_handle()),
        id(&std::fs::File::open(&source).unwrap())
    );
    let store_alias = root.path().join("store-alias");
    symlink(&store, &store_alias).unwrap();
    assert!(inspect(&store, &store_alias, None).is_err());
}

#[test]
fn topology_parent_segment_is_resolved_after_directory_alias() {
    let (_root, store, source) = fixture();
    fs::create_dir(store.join("branch")).unwrap();
    fs::create_dir(store.join("selected")).unwrap();
    fs::create_dir(source.join("selected")).unwrap();
    symlink(store.join("branch"), source.join("alias")).unwrap();
    let requested = source.join("alias/../selected");
    let native = std::fs::File::open(&requested).unwrap();
    assert_eq!(
        id(&native),
        id(&std::fs::File::open(store.join("selected")).unwrap())
    );
    assert_ne!(
        id(&native),
        id(&std::fs::File::open(source.join("selected")).unwrap())
    );
    assert!(inspect(&store, &requested, None).is_err());
}

#[test]
fn topology_dot_segments_without_alias_keep_native_object() {
    let (_root, store, source) = fixture();
    fs::create_dir(source.join("child")).unwrap();
    let observation = inspect(&store, &source.join("./child/../."), None).unwrap();
    assert_eq!(
        id(observation.source_handle()),
        id(&std::fs::File::open(&source).unwrap())
    );
}

#[test]
fn topology_regular_file_trailing_slash_and_dot_are_not_erased() {
    let (_root, store, source) = fixture();
    let file = source.join("data");
    fs::write(&file, b"source bytes").unwrap();
    for suffix in ["/", "/.", "/.."] {
        let path = PathBuf::from(format!("{}{suffix}", file.display()));
        let native = std::fs::File::open(&path).err().unwrap();
        let error =
            TopologyInspection::inspect(&[&store], SourcePath::File(&path), None, Wait::Try)
                .err()
                .unwrap();
        assert_eq!(error.raw_os_error(), native.raw_os_error());
    }
    assert_eq!(fs::read(&file).unwrap(), b"source bytes");
}

#[test]
fn topology_missing_destination_uses_existing_alias_anchor_without_creating() {
    let (root, store, source) = fixture();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let alias = root.path().join("out-alias");
    symlink(&output, &alias).unwrap();
    let destination = alias.join("new/child");
    let observation = inspect(&store, &source, Some(&destination)).unwrap();
    assert!(observation.destination_is_missing());
    observation.revalidate(Wait::Try).unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    fs::create_dir_all(&destination).unwrap();
    assert!(
        observation.revalidate(Wait::Try).is_err(),
        "changed topology was accepted"
    );
}

#[test]
fn topology_missing_suffix_dot_and_dangling_link_are_explicitly_rejected() {
    let (root, store, source) = fixture();
    let absent = root.path().join("absent");
    for path in [absent.join("../output"), absent.join("./child")] {
        let error = inspect(&store, &source, Some(&path))
            .err()
            .expect("unresolved dot components were accepted");
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }
    let dangling = root.path().join("dangling");
    symlink(root.path().join("uncreated-target"), &dangling).unwrap();
    assert_eq!(
        inspect(&store, &source, Some(&dangling))
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Unsupported
    );
    assert!(!absent.exists());
    assert!(fs::symlink_metadata(&dangling)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn topology_retained_source_and_revalidation_detect_replacement() {
    let (root, store, source) = fixture();
    let file = source.join("data");
    fs::write(&file, b"original").unwrap();
    let observation =
        TopologyInspection::inspect(&[&store], SourcePath::File(&file), None, Wait::Try).unwrap();
    fs::rename(&file, root.path().join("old-data")).unwrap();
    fs::write(&file, b"replacement").unwrap();
    let mut bytes = Vec::new();
    observation.source_handle().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"original");
    assert!(
        observation.revalidate(Wait::Try).is_err(),
        "changed topology was accepted"
    );
    assert_eq!(fs::read(&file).unwrap(), b"replacement");
}

#[test]
fn topology_regular_aliases_are_rejected_instead_of_guessed() {
    let (root, store, source) = fixture();
    let file = source.join("data");
    fs::write(&file, b"original").unwrap();
    let link = root.path().join("file-link");
    symlink(&file, &link).unwrap();
    assert!(
        TopologyInspection::inspect(&[&store], SourcePath::File(&link), None, Wait::Try).is_err()
    );
    fs::hard_link(&file, root.path().join("second-name")).unwrap();
    assert_eq!(
        TopologyInspection::inspect(&[&store], SourcePath::File(&file), None, Wait::Try)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Unsupported
    );
    assert_eq!(fs::read(&file).unwrap(), b"original");
}

#[test]
fn topology_non_utf8_name_matches_native_representation() {
    let (_root, store, source) = fixture();
    let file = source.join(std::ffi::OsString::from_vec(vec![b'f', 0xff]));
    match fs::write(&file, b"raw name") {
        Ok(()) => {
            let observation =
                TopologyInspection::inspect(&[&store], SourcePath::File(&file), None, Wait::Try)
                    .unwrap();
            assert_eq!(
                id(observation.source_handle()),
                id(&std::fs::File::open(&file).unwrap())
            );
            eprintln!("topology native name: byte-exact supported");
        }
        Err(error) => {
            // APFS rejects this byte sequence. Exercise actual rejection, not
            // a skip or an arbitrary successful substitution with another name.
            assert_eq!(error.raw_os_error(), Some(libc::EILSEQ));
            let native = std::fs::File::open(&file).err().unwrap();
            let observed =
                TopologyInspection::inspect(&[&store], SourcePath::File(&file), None, Wait::Try)
                    .err()
                    .unwrap();
            assert_eq!(observed.raw_os_error(), native.raw_os_error());
            assert_eq!(fs::read_dir(&source).unwrap().count(), 0);
            eprintln!(
                "topology native name: create EILSEQ, lookup error {:?} preserved",
                native.raw_os_error()
            );
        }
    }
}

#[test]
fn topology_unresolved_source_keeps_native_error_and_no_destination() {
    let (root, store, source) = fixture();
    let missing = source.join("missing");
    let output = root.path().join("output");
    let native = std::fs::File::open(&missing).err().unwrap();
    let error = TopologyInspection::inspect(
        &[&store],
        SourcePath::File(&missing),
        Some(&output),
        Wait::Try,
    )
    .err()
    .unwrap();
    assert_eq!(error.raw_os_error(), native.raw_os_error());
    assert!(!output.exists());
}

#[test]
fn topology_unqualified_shapes_and_empty_inventory_fail_closed() {
    let (_root, store, source) = fixture();
    assert!(
        TopologyInspection::inspect(&[], SourcePath::Directory(&source), None, Wait::Try).is_err()
    );
    assert!(TopologyInspection::inspect(
        &vec![store.as_path(); 33],
        SourcePath::Directory(&source),
        None,
        Wait::Try
    )
    .is_err());
    for path in [
        PathBuf::new(),
        PathBuf::from("//ambiguous"),
        PathBuf::from(std::ffi::OsString::from_vec(b"nul\0part".to_vec())),
    ] {
        assert!(inspect(&store, &path, None).is_err());
    }
    assert_eq!(
        fs::read(store.join("sentinel")).unwrap(),
        b"protected bytes"
    );
}

#[test]
fn topology_missing_name_representation_is_not_guessed() {
    let (root, store, source) = fixture();
    let destination = root
        .path()
        .join(std::ffi::OsString::from_vec(vec![b'n', 0xff]));
    let result = inspect(&store, &source, Some(&destination));
    #[cfg(target_os = "macos")]
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::Unsupported);
    #[cfg(target_os = "linux")]
    assert!(result.unwrap().destination_is_missing());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn topology_observed_source_is_read_only() {
    use std::io::Write;
    let (_root, store, source) = fixture();
    let file = source.join("data");
    fs::write(&file, b"untouched").unwrap();
    let observation =
        TopologyInspection::inspect(&[&store], SourcePath::File(&file), None, Wait::Try).unwrap();
    let error = observation
        .source_handle()
        .write_all(b"changed")
        .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EBADF));
    assert_eq!(fs::read(&file).unwrap(), b"untouched");
}

#[test]
fn topology_relative_paths_match_native_lookup() {
    let (_root, store, source) = fixture();
    // Construct the oracle-relative spelling from native canonical directories;
    // never change process CWD, which other tests may be using concurrently.
    fn relative(target: &Path) -> PathBuf {
        let cwd = fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        let target = fs::canonicalize(target).unwrap();
        let a: Vec<_> = cwd.components().collect();
        let b: Vec<_> = target.components().collect();
        let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let mut result = PathBuf::new();
        for _ in common..a.len() {
            result.push("..");
        }
        for part in &b[common..] {
            result.push(part.as_os_str());
        }
        result
    }
    let observed = inspect(&relative(&store), &relative(&source), None).unwrap();
    assert_eq!(
        id(observed.source_handle()),
        id(&std::fs::File::open(&source).unwrap())
    );
    observed.revalidate(Wait::Try).unwrap();
}
