//! Public-path regressions for literal path/target fidelity, with raw OS oracles.
use super::{manifest_relative_path, scan, Index};
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
fn non_utf8_path_conversion_is_rejected_before_any_filesystem_operation() {
    let bad = OsString::from_vec(b"bad\xff".to_vec());
    let error = manifest_relative_path(Path::new(&bad)).err().unwrap();
    assert!(matches!(error, LightrError::InvalidManifest(_)));
}

#[test]
fn native_filesystem_or_scan_explicitly_rejects_selected_non_utf8_filename() {
    let root = TempDir::new().unwrap();
    let path = root.path().join(OsString::from_vec(b"bad\xff".to_vec()));
    match fs::write(&path, b"data") {
        Ok(()) => {
            let error = scan(root.path(), &mut Index::empty()).err().unwrap();
            assert!(matches!(error, LightrError::InvalidManifest(_)));
            assert_eq!(fs::read(path).unwrap(), b"data");
        }
        Err(error) => {
            // APFS rejects the fixture at creation (Darwin EILSEQ = 92).
            // This is an asserted native capability, not a skipped scan success.
            // The separate conversion test still exercises our rejection logic.
            assert!(cfg!(target_os = "macos"), "unexpected creation error: {error}");
            assert_eq!(error.raw_os_error(), Some(92));
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        }
    }
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
