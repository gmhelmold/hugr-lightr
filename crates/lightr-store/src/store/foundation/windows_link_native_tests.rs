//! Native Windows witness with disposable data only; no capability skip.
use super::*;
use std::fs;
use std::os::windows::fs::{symlink_dir, symlink_file, FileTypeExt};
use tempfile::TempDir;

#[test]
fn windows_link_plan_native_creation_keeps_manifest_derived_kind_after_target_removal() {
    let m = manifest(vec![
        file("data"),
        dir("folder"),
        link("file-link", "data"),
        link("dir-link", "folder"),
    ]);
    let tree = plan(&m);
    let output = WindowsLinkPlan::validate(&tree, Wait::Try).unwrap();
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("data"), b"captured fixture").unwrap();
    fs::create_dir(root.path().join("folder")).unwrap();
    for item in output.links() {
        let destination = root.path().join(item.path);
        match item.kind {
            LinkKind::File => symlink_file(item.target, destination),
            LinkKind::Directory => symlink_dir(item.target, destination),
        }
        .expect("native link capability is required for this Windows witness");
    }
    let file_link = root.path().join("file-link");
    let dir_link = root.path().join("dir-link");
    assert_eq!(fs::read(&file_link).unwrap(), b"captured fixture");
    assert!(fs::read_dir(&dir_link).unwrap().next().is_none());
    for dangling in [false, true] {
        if dangling {
            fs::remove_file(root.path().join("data")).unwrap();
            fs::remove_dir(root.path().join("folder")).unwrap();
        }
        assert!(fs::symlink_metadata(&file_link)
            .unwrap()
            .file_type()
            .is_symlink_file());
        assert!(fs::symlink_metadata(&dir_link)
            .unwrap()
            .file_type()
            .is_symlink_dir());
        assert_eq!(
            fs::read_link(&file_link).unwrap(),
            std::path::Path::new("data")
        );
        assert_eq!(
            fs::read_link(&dir_link).unwrap(),
            std::path::Path::new("folder")
        );
    }
    // Removal is explicit and does not follow either dangling target.
    fs::remove_file(file_link).unwrap();
    fs::remove_dir(dir_link).unwrap();
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
