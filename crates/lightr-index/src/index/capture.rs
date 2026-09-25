//! Inert, lease-bound source capture for SI-02.
#![allow(dead_code)] // SI-03 intentionally owns production activation.

use lightr_core::{Entry, LightrError, Manifest, Result};
use lightr_store::store::foundation::{CaptureMode, PreparedObject, StoreLease, Wait};
use std::fs::{self, File, Metadata};
use std::path::{Path, PathBuf};

struct Candidate {
    relative: String,
    absolute: PathBuf,
    kind: CandidateKind,
    metadata: Metadata,
}

enum CandidateKind {
    File,
    Symlink(String),
    Directory,
}

/// Manifest plus preparation proofs. Proofs cannot outlive the caller lease.
pub(super) struct CapturedManifest<'lease> {
    manifest: Manifest,
    proofs: Vec<PreparedObject<'lease>>,
}

/// Capture selected source bytes without consulting the stat index or Store.
/// This module is intentionally private until SI-03 wires a publisher to it.
pub(super) fn capture_verified<'lease>(
    source: &Path,
    lease: &'lease StoreLease,
    _policy: CaptureMode,
) -> Result<CapturedManifest<'lease>> {
    let root = source.canonicalize().map_err(LightrError::Io)?;
    let root_meta = fs::symlink_metadata(&root).map_err(LightrError::Io)?;
    if !root_meta.is_dir() {
        return Err(LightrError::InvalidManifest(
            "capture source root is not a directory".into(),
        ));
    }

    let mut candidates = Vec::new();
    let walker = ignore::WalkBuilder::new(&root)
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .add_custom_ignore_filename(".lightrignore")
        .filter_entry(|entry| {
            entry.file_name() != ".git"
                || !entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_dir())
        })
        .build();

    for result in walker {
        let entry = result.map_err(|error| LightrError::Io(std::io::Error::other(error)))?;
        let absolute = entry.path().to_path_buf();
        if absolute == root {
            continue;
        }

        let metadata = fs::symlink_metadata(&absolute).map_err(LightrError::Io)?;
        let relative = manifest_relative_path(absolute.strip_prefix(&root).map_err(|_| {
            LightrError::InvalidManifest("walked path escaped capture root".into())
        })?)?;

        let kind = if metadata.is_symlink() {
            let target = fs::read_link(&absolute).map_err(LightrError::Io)?;
            let target = target.to_str().ok_or_else(|| {
                LightrError::InvalidManifest("selected link target is not valid UTF-8".into())
            })?;
            CandidateKind::Symlink(target.to_owned())
        } else if metadata.is_dir() {
            let mut entries = fs::read_dir(&absolute).map_err(LightrError::Io)?;
            let mut nonempty = false;
            for entry in &mut entries {
                entry.map_err(LightrError::Io)?;
                nonempty = true;
            }
            if nonempty {
                continue;
            }
            CandidateKind::Directory
        } else if metadata.is_file() {
            CandidateKind::File
        } else {
            return Err(LightrError::InvalidManifest(format!(
                "unsupported selected file type: {relative}"
            )));
        };

        candidates.push(Candidate {
            relative,
            absolute,
            kind,
            metadata,
        });
    }

    candidates.sort_by(|left, right| left.relative.cmp(&right.relative));
    let mut entries = Vec::with_capacity(candidates.len());
    let mut proofs = Vec::new();
    let mut total_size = 0u64;

    for candidate in candidates {
        match candidate.kind {
            CandidateKind::File => {
                let mut file = File::open(&candidate.absolute).map_err(LightrError::Io)?;
                let before = file.metadata().map_err(LightrError::Io)?;
                ensure_unchanged(&candidate.metadata, &before, &candidate.relative)?;

                let operation_id = operation_id(&candidate.relative);
                let prepared = lease
                    .prepare_reader(&mut file, None, Wait::Try, operation_id)
                    .map_err(|error| error.into_shared_legacy())?;

                let after_file = file.metadata().map_err(LightrError::Io)?;
                let after_path =
                    fs::symlink_metadata(&candidate.absolute).map_err(LightrError::Io)?;
                ensure_unchanged(&before, &after_file, &candidate.relative)?;
                ensure_unchanged(&before, &after_path, &candidate.relative)?;

                total_size = total_size
                    .checked_add(prepared.len())
                    .ok_or_else(|| LightrError::InvalidManifest("capture size overflow".into()))?;
                entries.push(Entry::File {
                    path: candidate.relative,
                    mode: stat_fields(&before).3,
                    size: prepared.len(),
                    digest: prepared.digest(),
                });
                proofs.push(prepared);
            }
            CandidateKind::Symlink(target) => entries.push(Entry::Symlink {
                path: candidate.relative,
                target,
            }),
            CandidateKind::Directory => entries.push(Entry::Dir {
                path: candidate.relative,
            }),
        }
    }

    Ok(CapturedManifest {
        manifest: Manifest {
            version: 1,
            total_size,
            entries,
        },
        proofs,
    })
}

fn manifest_relative_path(relative: &Path) -> Result<String> {
    let raw = relative
        .to_str()
        .ok_or_else(|| LightrError::InvalidManifest("selected path is not valid UTF-8".into()))?;
    #[cfg(windows)]
    let path = raw.replace('\\', "/");
    #[cfg(not(windows))]
    let path = raw.to_owned();
    Ok(path)
}

fn stat_fields(metadata: &Metadata) -> (u64, u64, u64, u32) {
    super::scan::stat_fields(metadata)
}

fn ensure_unchanged(before: &Metadata, after: &Metadata, path: &str) -> Result<()> {
    if before.file_type() != after.file_type() || stat_fields(before) != stat_fields(after) {
        return Err(LightrError::InvalidManifest(format!(
            "source changed during capture: {path}"
        )));
    }
    Ok(())
}

fn operation_id(relative: &str) -> [u8; 16] {
    let digest = lightr_core::Digest::of_bytes(relative.as_bytes());
    digest.0[..16].try_into().expect("fixed digest prefix")
}

#[cfg(test)]
mod tests {
    use super::{capture_verified, CaptureMode};
    use crate::index::{codec::Index, scan::scan};
    use lightr_core::{Digest, Entry, LightrError};
    use lightr_store::store::foundation::{StoreLocks, Wait};
    use lightr_store::Store;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    use tempfile::TempDir;

    fn capture<'a>(
        source: &std::path::Path,
        lease: &'a lightr_store::store::foundation::StoreLease,
    ) -> super::CapturedManifest<'a> {
        capture_verified(source, lease, CaptureMode::CopyOnly).unwrap()
    }

    fn lease(store: &Store) -> (StoreLocks, lightr_store::store::foundation::StoreLease) {
        let locks = StoreLocks::open_existing(store.root()).unwrap();
        let lease = locks.shared(Wait::Try).unwrap();
        (locks, lease)
    }

    #[test]
    fn warmed_index_is_ignored_and_fresh_bytes_win() {
        let _env_guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        std::env::set_var("LIGHTR_HOME", &home);
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let file = source.join("data");
        fs::write(&file, b"AAAA").unwrap();
        let mut index = Index::empty();
        scan(&source, &mut index).unwrap();
        let original_mtime = fs::metadata(&file).unwrap().modified().unwrap();
        fs::write(&file, b"BBBB").unwrap();
        let handle = fs::OpenOptions::new().write(true).open(&file).unwrap();
        let _ = handle.set_modified(original_mtime);

        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        let captured = capture(&source, &lease);
        let Entry::File { digest, size, .. } = &captured.manifest.entries[0] else {
            panic!("expected file entry")
        };
        assert_eq!(*digest, Digest::of_bytes(b"BBBB"));
        assert_eq!(*size, 4);
        assert_eq!(captured.proofs[0].digest(), *digest);
        assert_eq!(captured.proofs[0].len(), *size);
    }

    #[test]
    fn missing_root_is_error_not_empty_manifest() {
        let temp = TempDir::new().unwrap();
        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        let result = capture_verified(&temp.path().join("missing"), &lease, CaptureMode::CopyOnly);
        assert!(matches!(result, Err(LightrError::Io(_))));
    }

    #[test]
    #[cfg(unix)]
    fn empty_directory_and_link_text_round_trip() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        fs::create_dir_all(source.join("empty")).unwrap();
        symlink("target\\text", source.join("link")).unwrap();
        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        let captured = capture(&source, &lease);
        assert!(captured.manifest.entries.contains(&Entry::Dir {
            path: "empty".into()
        }));
        assert!(captured.manifest.entries.contains(&Entry::Symlink {
            path: "link".into(),
            target: "target\\text".into(),
        }));
    }

    #[test]
    #[cfg(unix)]
    fn literal_backslash_name_and_link_target_are_preserved() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("literal\\name"), b"bytes").unwrap();
        symlink("literal\\name", source.join("link\\name")).unwrap();
        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        let captured = capture(&source, &lease);
        assert!(captured
            .manifest
            .entries
            .iter()
            .any(|entry| entry.path() == "literal\\name"));
        assert!(captured.manifest.entries.contains(&Entry::Symlink {
            path: "link\\name".into(),
            target: "literal\\name".into(),
        }));
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_name_is_rejected_or_explicitly_not_exercised() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let path = source.join(std::ffi::OsString::from_vec(b"bad\xff".to_vec()));
        if let Err(error) = fs::write(&path, b"bytes") {
            eprintln!("NOT_EXERCISED: native filesystem rejected non-UTF8 fixture: {error}");
            return;
        }
        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        assert!(matches!(
            capture_verified(&source, &lease, CaptureMode::CopyOnly),
            Err(LightrError::InvalidManifest(_))
        ));
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_link_target_is_rejected_or_explicitly_not_exercised() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let target = std::ffi::OsString::from_vec(b"target\xff".to_vec());
        if let Err(error) = symlink(&target, source.join("link")) {
            eprintln!("NOT_EXERCISED: native filesystem rejected non-UTF8 link target: {error}");
            return;
        }
        let store = Store::open(temp.path().join("store")).unwrap();
        let (_locks, lease) = lease(&store);
        assert!(matches!(
            capture_verified(&source, &lease, CaptureMode::CopyOnly),
            Err(LightrError::InvalidManifest(_))
        ));
    }
}
