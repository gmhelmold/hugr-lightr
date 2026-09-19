//! Native, read-only C12 observations on a conservative supported subset.
use super::super::Wait;
use super::topology_io::{definitely_absent, key, open_at, unsupported, Key};
use super::{ProtectedRoot, SourcePath};
use std::fs::File;
use std::io;
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Directory,
    File,
    ProposedDirectory,
}
struct Request {
    path: PathBuf,
    kind: Kind,
}
struct Resolved {
    object: File,
    object_key: Key,
    _ancestors: Vec<File>,
    ancestry: Vec<Key>,
    missing: Vec<Vec<u8>>,
    directory: bool,
}
impl Resolved {
    fn unchanged(&self, other: &Self) -> bool {
        self.object_key == other.object_key
            && self.ancestry == other.ancestry
            && self.missing == other.missing
            && self.directory == other.directory
    }
    fn check_unresolved_overlap(&self, other: &Self) -> io::Result<()> {
        if !self.missing.is_empty()
            && !other.missing.is_empty()
            && self.object_key == other.object_key
        {
            if self.missing.starts_with(&other.missing) || other.missing.starts_with(&self.missing)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "planned protected root overlaps missing public destination",
                ));
            }
            // No actual entries exist to compare. Different byte spellings do
            // not establish disjoint native names on every supported volume.
            return Err(unsupported(
                "unresolved paths share an ancestor; native disjointness is unknown",
            ));
        }
        Ok(())
    }
    fn contains(&self, other: &Self) -> bool {
        self.missing.is_empty()
            && (self.object_key == other.object_key
                || (self.directory && other.ancestry.contains(&self.object_key)))
    }
}

pub(super) struct Inspection {
    cwd: File,
    requests: Vec<Request>,
    roots: usize,
    destination: Option<usize>,
    observed: Vec<Resolved>,
}
impl Inspection {
    pub(super) fn inspect(
        protected: &[&Path],
        source: SourcePath<'_>,
        destination: Option<&Path>,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        Self::inspect_paths(
            protected
                .iter()
                .copied()
                .map(ProtectedRoot::ExistingDirectory),
            Some(source),
            destination,
            wait,
        )
    }

    pub(super) fn inspect_destination(
        protected: &[&Path],
        destination: &Path,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        Self::inspect_paths(
            protected
                .iter()
                .copied()
                .map(ProtectedRoot::ExistingDirectory),
            None,
            Some(destination),
            wait,
        )
    }

    pub(super) fn inspect_configured(
        protected: &[ProtectedRoot<'_>],
        source: Option<SourcePath<'_>>,
        destination: Option<&Path>,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        Self::inspect_paths(protected.iter().copied(), source, destination, wait)
    }

    fn inspect_paths<'a>(
        protected: impl ExactSizeIterator<Item = ProtectedRoot<'a>>,
        source: Option<SourcePath<'_>>,
        destination: Option<&Path>,
        wait: Wait<'_>,
    ) -> io::Result<Self> {
        let roots = protected.len();
        if roots == 0 || roots > 32 {
            return Err(unsupported(
                "supply between 1 and 32 configured protected roots",
            ));
        }
        wait.check()?;
        let cwd = File::open(".")?;
        let mut requests: Vec<_> = protected
            .map(|root| {
                let (path, kind) = match root {
                    ProtectedRoot::ExistingDirectory(path) => (path, Kind::Directory),
                    ProtectedRoot::PlannedDirectory(path) => (path, Kind::ProposedDirectory),
                };
                Request {
                    path: path.to_path_buf(),
                    kind,
                }
            })
            .collect();
        if let Some(source) = source {
            let (path, kind) = match source {
                SourcePath::Directory(path) => (path, Kind::Directory),
                SourcePath::File(path) => (path, Kind::File),
            };
            requests.push(Request {
                path: path.to_path_buf(),
                kind,
            });
        }
        let destination = destination.map(|path| {
            let index = requests.len();
            requests.push(Request {
                path: path.to_path_buf(),
                kind: Kind::ProposedDirectory,
            });
            index
        });
        let observed = snapshot(&cwd, &requests, roots, wait)?;
        let result = Self {
            cwd,
            requests,
            roots,
            destination,
            observed,
        };
        // Detect changes during acquisition rather than returning an already
        // stale observation. This is bounded validation, never a retry loop.
        result.revalidate(wait)?;
        Ok(result)
    }

    pub(super) fn revalidate(&self, wait: Wait<'_>) -> io::Result<()> {
        let current = snapshot(&self.cwd, &self.requests, self.roots, wait)?;
        if !self
            .observed
            .iter()
            .zip(&current)
            .all(|(a, b)| a.unchanged(b))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "observed topology changed",
            ));
        }
        wait.check()
    }

    pub(super) fn source_handle(&self) -> &File {
        &self.observed[self.roots].object
    }
    pub(super) fn destination_is_missing(&self) -> bool {
        self.destination
            .is_some_and(|index| !self.observed[index].missing.is_empty())
    }

    pub(super) fn destination_handle(&self) -> Option<&File> {
        self.destination.and_then(|index| {
            let target = &self.observed[index];
            target.missing.is_empty().then_some(&target.object)
        })
    }
}

fn snapshot(
    cwd: &File,
    requests: &[Request],
    roots: usize,
    wait: Wait<'_>,
) -> io::Result<Vec<Resolved>> {
    let mut budget = 512usize;
    let observed: Vec<_> = requests
        .iter()
        .map(|request| resolve(cwd, request, wait, &mut budget))
        .collect::<io::Result<_>>()?;
    for subject in &observed[roots..] {
        for protected in &observed[..roots] {
            protected.check_unresolved_overlap(subject)?;
            if protected.contains(subject) || subject.contains(protected) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "public path overlaps a protected namespace",
                ));
            }
            if !subject.object_key.same_volume(protected.object_key) {
                return Err(unsupported(
                    "cross-mount topology requires separate qualification",
                ));
            }
        }
    }
    if let Some(destination) = observed.get(roots + 1) {
        let source = &observed[roots];
        if source.contains(destination) || destination.contains(source) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source and destination overlap",
            ));
        }
    }
    wait.check()?;
    Ok(observed)
}

fn resolve(
    cwd: &File,
    request: &Request,
    wait: Wait<'_>,
    budget: &mut usize,
) -> io::Result<Resolved> {
    let bytes = request.path.as_os_str().as_bytes();
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty or NUL-containing path",
        ));
    }
    if bytes.len() > 4096 || (bytes.starts_with(b"//") && !bytes.starts_with(b"///")) {
        return Err(unsupported(
            "path length or double-root semantics are unsupported",
        ));
    }
    let parts: Vec<_> = bytes
        .split(|b| *b == b'/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() > 128 {
        return Err(unsupported("topology path depth exceeded"));
    }
    wait.check()?;
    let mut parent = if bytes[0] == b'/' {
        open_at(cwd, b"/", true)?
    } else {
        cwd.try_clone()?
    };
    if parts.is_empty() {
        if request.kind == Kind::File {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        return retain(parent, None, Vec::new(), wait, budget);
    }
    for (index, part) in parts.iter().enumerate() {
        wait.check()?;
        let last = index + 1 == parts.len();
        let directory = !last || request.kind != Kind::File || bytes.ends_with(b"/");
        match open_at(&parent, part, directory) {
            Ok(file) => {
                wait.check()?;
                if last {
                    if request.kind == Kind::File {
                        let metadata = file.metadata()?;
                        if !metadata.is_file() {
                            return Err(io::ErrorKind::InvalidInput.into());
                        }
                        if metadata.nlink() != 1 {
                            return Err(unsupported("multiply linked or unlinked regular source"));
                        }
                        return retain(file, Some(parent), Vec::new(), wait, budget);
                    }
                    return retain(file, None, Vec::new(), wait, budget);
                }
                parent = file;
            }
            Err(error)
                if error.kind() == io::ErrorKind::NotFound
                    && request.kind == Kind::ProposedDirectory =>
            {
                if !definitely_absent(&parent, part)? {
                    return Err(unsupported(
                        "unresolved present entry is not an absent destination",
                    ));
                }
                let tail = &parts[index..];
                #[cfg(target_os = "macos")]
                if tail.iter().any(|name| std::str::from_utf8(name).is_err()) {
                    return Err(unsupported("unrepresentable APFS destination name"));
                }

                if tail.iter().any(|p| *p == b"." || *p == b"..") {
                    return Err(unsupported(
                        "dot components after a missing ancestor are unsupported",
                    ));
                }
                return retain(
                    parent,
                    None,
                    tail.iter().map(|p| p.to_vec()).collect(),
                    wait,
                    budget,
                );
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("nonempty path always returns at final component")
}

fn retain(
    object: File,
    parent: Option<File>,
    missing: Vec<Vec<u8>>,
    wait: Wait<'_>,
    budget: &mut usize,
) -> io::Result<Resolved> {
    let directory = parent.is_none();
    let object_key = key(&object)?;
    let mut current = match parent {
        Some(parent) => parent,
        None => object.try_clone()?,
    };
    let mut handles = Vec::new();
    let mut ancestry = Vec::new();
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| unsupported("topology handle budget exhausted"))?;
    for _ in 0..128 {
        wait.check()?;
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| unsupported("topology handle budget exhausted"))?;
        let here = key(&current)?;
        let next = open_at(&current, b"..", true)?;
        let above = key(&next)?;
        ancestry.push(here);
        handles.push(current);
        if here == above {
            return Ok(Resolved {
                object,
                object_key,
                _ancestors: handles,
                ancestry,
                missing,
                directory,
            });
        }
        current = next;
    }
    Err(unsupported("native ancestry depth exceeded"))
}
