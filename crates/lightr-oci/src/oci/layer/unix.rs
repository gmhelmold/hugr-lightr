//! Descriptor-confined Unix implementation of [`super::apply::LayerFs`].

use super::apply::{LayerFs, LayerOp};
use lightr_core::{LightrError, Result};
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags};
use std::{
    ffi::OsString,
    fs::File,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn invalid(error: rustix::io::Errno) -> LightrError {
    LightrError::InvalidManifest(format!("confined layer apply failed: {error}"))
}
fn open_dir(parent: &File, component: &std::ffi::OsStr) -> Result<File> {
    let fd = fs::openat(
        parent,
        component,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(invalid)?;
    Ok(File::from(fd))
}
fn mkdir_open(parent: &File, component: &std::ffi::OsStr) -> Result<File> {
    match fs::mkdirat(parent, component, Mode::from_raw_mode(0o755)) {
        Ok(()) => (),
        Err(rustix::io::Errno::EXIST) => (),
        Err(error) => return Err(invalid(error)),
    }
    open_dir(parent, component)
}
fn ensure(root: &File, path: &[OsString]) -> Result<File> {
    let mut dir = root.try_clone().map_err(LightrError::Io)?;
    for part in path {
        dir = mkdir_open(&dir, part)?;
    }
    Ok(dir)
}
fn existing(root: &File, path: &[OsString]) -> Result<File> {
    let mut dir = root.try_clone().map_err(LightrError::Io)?;
    for part in path {
        dir = open_dir(&dir, part)?;
    }
    Ok(dir)
}
fn unlink_one(parent: &File, component: &std::ffi::OsStr) -> Result<()> {
    let stat = match fs::statat(parent, component, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(rustix::io::Errno::NOENT) => return Ok(()),
        Err(error) => return Err(invalid(error)),
    };
    let directory = FileType::from_raw_mode(stat.st_mode) == FileType::Directory;
    if directory {
        let child = open_dir(parent, component)?;
        clear(&child)?;
    }
    fs::unlinkat(
        parent,
        component,
        if directory {
            AtFlags::REMOVEDIR
        } else {
            AtFlags::empty()
        },
    )
    .map_err(invalid)
}
fn clear(dir: &File) -> Result<()> {
    let mut entries = Dir::read_from(dir).map_err(invalid)?;
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(invalid)?;
        let name = entry.file_name();
        if name.to_bytes() != b"." && name.to_bytes() != b".." {
            unlink_one(dir, std::ffi::OsStr::from_bytes(name.to_bytes()))?;
        }
    }
    Ok(())
}

pub(super) struct UnixLayerFs {
    root: File,
    path: PathBuf,
    parent: File,
    stage_name: OsString,
    identity: (i32, u64),
}
impl UnixLayerFs {
    pub(super) fn new() -> Result<Self> {
        let base = std::env::temp_dir();
        let parent = File::open(&base).map_err(LightrError::Io)?;
        for _ in 0..32 {
            let stage = OsString::from(format!(
                "lightr-oci-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::mkdirat(&parent, &stage, Mode::from_raw_mode(0o700)) {
                Ok(()) => {
                    let root = open_dir(&parent, &stage)?;
                    let stat = fs::fstat(&root).map_err(invalid)?;
                    return Ok(Self {
                        root,
                        path: base.join(&stage),
                        parent,
                        stage_name: stage,
                        identity: (stat.st_dev, stat.st_ino),
                    });
                }
                Err(rustix::io::Errno::EXIST) => (),
                Err(error) => return Err(invalid(error)),
            }
        }
        Err(LightrError::InvalidManifest(
            "private OCI staging reservation exhausted 32 candidates".into(),
        ))
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for UnixLayerFs {
    fn drop(&mut self) {
        let _ = clear(&self.root);
        if let Ok(stat) = fs::statat(&self.parent, &self.stage_name, AtFlags::SYMLINK_NOFOLLOW) {
            if (stat.st_dev, stat.st_ino) == self.identity {
                let _ = fs::unlinkat(&self.parent, &self.stage_name, AtFlags::REMOVEDIR);
            }
        }
    }
}
impl LayerFs for UnixLayerFs {
    fn apply(&mut self, op: &LayerOp) -> Result<()> {
        match op {
            LayerOp::Directory(path) => {
                ensure(&self.root, path)?;
            }
            LayerOp::Whiteout {
                parent,
                name: Some(name),
            } => {
                let parent = ensure(&self.root, parent)?;
                unlink_one(&parent, name)?;
            }
            LayerOp::Whiteout { parent, name: None } => {
                let dir = ensure(&self.root, parent)?;
                clear(&dir)?;
            }
            LayerOp::Regular { dest, data, mode } => {
                let (parents, leaf) = dest.split_at(dest.len() - 1);
                let parent = ensure(&self.root, parents)?;
                unlink_one(&parent, &leaf[0])?;
                let fd = fs::openat(
                    &parent,
                    &leaf[0],
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::from_raw_mode(*mode as u16),
                )
                .map_err(invalid)?;
                let mut file = File::from(fd);
                file.write_all(data).map_err(LightrError::Io)?;
                fs::fchmod(&file, Mode::from_raw_mode(*mode as u16)).map_err(invalid)?;
            }
            LayerOp::Symlink { dest, target } => {
                let (parents, leaf) = dest.split_at(dest.len() - 1);
                let parent = ensure(&self.root, parents)?;
                unlink_one(&parent, &leaf[0])?;
                fs::symlinkat(target, &parent, &leaf[0]).map_err(invalid)?;
            }
            LayerOp::Hardlink { dest, target } => {
                let (source_parents, source_leaf) = target.split_at(target.len() - 1);
                let source_parent = existing(&self.root, source_parents)?;
                let stat = fs::statat(&source_parent, &source_leaf[0], AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|_| {
                    LightrError::InvalidManifest("hardlink target not found".into())
                })?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                    return Err(LightrError::InvalidManifest(
                        "hardlink target not regular".into(),
                    ));
                }
                let (parents, leaf) = dest.split_at(dest.len() - 1);
                let parent = ensure(&self.root, parents)?;
                unlink_one(&parent, &leaf[0])?;
                fs::linkat(
                    &source_parent,
                    &source_leaf[0],
                    &parent,
                    &leaf[0],
                    AtFlags::empty(),
                )
                .map_err(invalid)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn hardlink_preserves_inode_identity() {
        let mut fs = UnixLayerFs::new().unwrap();
        fs.apply(&LayerOp::Regular {
            dest: path(&["source"]),
            data: b"data".to_vec(),
            mode: 0o644,
        })
        .unwrap();
        fs.apply(&LayerOp::Hardlink {
            dest: path(&["copy"]),
            target: path(&["source"]),
        })
        .unwrap();
        let source = std::fs::metadata(fs.path().join("source")).unwrap();
        let copy = std::fs::metadata(fs.path().join("copy")).unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_eq!((source.dev(), source.ino()), (copy.dev(), copy.ino()));
    }

    #[test]
    fn whiteout_of_symlink_does_not_touch_target() {
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"retained").unwrap();
        let mut fs = UnixLayerFs::new().unwrap();
        fs.apply(&LayerOp::Symlink {
            dest: path(&["link"]),
            target: outside.path().as_os_str().to_os_string(),
        })
        .unwrap();
        fs.apply(&LayerOp::Whiteout {
            parent: vec![],
            name: Some(OsString::from("link")),
        })
        .unwrap();
        assert_eq!(std::fs::read(outside.path()).unwrap(), b"retained");
    }
}
