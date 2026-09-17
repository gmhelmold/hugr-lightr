//! Public-path regressions for literal path/target fidelity, with raw OS oracles.
use super::{scan, Index};
use crate::{hydrate_verified, snapshot};
use lightr_core::LightrError;
use lightr_store::Store;
use std::ffi::OsString;
use std::fs;
use std::os::unix::{ffi::OsStringExt, fs::symlink};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn snapshot_preserves_literal_backslash_files_and_exact_link_targets() {
    let _env_guard = crate::TEST_ENV_LOCK.lock().unwrap();
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(source.join("literal")).unwrap();
    fs::write(source.join("literal\\file"), b"literal-backslash").unwrap();
    fs::write(source.join("literal/file"), b"slash-separated").unwrap();
    symlink("literal\\file", source.join("live-link")).unwrap();
    symlink("absent\\leaf", source.join("dangling-link")).unwrap();
    let store = Store::open(root.path().join("store")).unwrap();
    snapshot(&source, &store, "@si00/paths").unwrap();
    let dest = root.path().join("restored");
    hydrate_verified(&dest, &store, "@si00/paths").unwrap();
    for tree in [&source, &dest] {
        assert_eq!(
            fs::read(tree.join("literal\\file")).unwrap(),
            b"literal-backslash"
        );
        assert_eq!(
            fs::read(tree.join("literal/file")).unwrap(),
            b"slash-separated"
        );
        assert_eq!(
            fs::read_link(tree.join("live-link")).unwrap(),
            Path::new("literal\\file")
        );
        assert_eq!(
            fs::read_link(tree.join("dangling-link")).unwrap(),
            Path::new("absent\\leaf")
        );
        assert_eq!(
            fs::read(tree.join("live-link")).unwrap(),
            b"literal-backslash"
        );
        assert!(fs::symlink_metadata(tree.join("dangling-link"))
            .unwrap()
            .file_type()
            .is_symlink());
    }
}

#[test]
fn selected_non_utf8_filename_is_rejected_instead_of_lossily_rewritten() {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join(OsString::from_vec(b"bad\xff".to_vec())),
        b"data",
    )
    .unwrap();
    let error = scan(root.path(), &mut Index::empty()).err().unwrap();
    assert!(matches!(error, LightrError::InvalidManifest(_)));
}

#[test]
fn selected_non_utf8_link_target_is_rejected_instead_of_lossily_rewritten() {
    let root = TempDir::new().unwrap();
    let target = OsString::from_vec(b"absent\xff".to_vec());
    symlink(&target, root.path().join("link")).unwrap();
    let error = scan(root.path(), &mut Index::empty()).err().unwrap();
    assert!(matches!(error, LightrError::InvalidManifest(_)));
    assert_eq!(
        fs::read_link(root.path().join("link")).unwrap().as_os_str(),
        target.as_os_str()
    );
}
