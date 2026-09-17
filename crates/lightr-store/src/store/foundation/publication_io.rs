//! Checked fixed-namespace I/O. No public-path resolver or recursive deletion.
use super::lease_io::Directory;
use super::ready::invalid;
use super::{Assurance, Readiness, StoreLease};
use crate::store::cas::preparation::StagedFile;
use lightr_core::Digest;
use std::fs::File;
use std::io::{self, Cursor, Seek};
use std::path::PathBuf;

pub(super) struct Layout<'a> {
    root: &'a Directory,
    objects: Directory,
    pub(super) payload: Directory,
    receipts: Directory,
    pub(super) receipt: Directory,
    staging: Directory,
    name: String,
}

impl<'a> Layout<'a> {
    pub(super) fn open(lease: &'a StoreLease, digest: Digest) -> io::Result<Self> {
        let root = lease.directory();
        let hex = digest.to_hex();
        let objects = root.child("objects")?;
        let payload = objects.child(&hex[..2])?;
        let receipts = root.child("objects-ready")?;
        let receipt = receipts.child(&hex[..2])?;
        let staging = lease.staging()?;
        if [&objects, &payload, &receipts, &receipt, &staging]
            .iter()
            .any(|d| !root.same_filesystem(d))
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "publication namespaces cross filesystems",
            ));
        }
        Ok(Self {
            root,
            objects,
            payload,
            receipts,
            receipt,
            staging,
            name: hex[2..].to_owned(),
        })
    }
    pub(super) fn payload_path(&self) -> PathBuf {
        self.payload.path().join(&self.name)
    }
    pub(super) fn receipt_path(&self) -> PathBuf {
        self.receipt.path().join(&self.name)
    }

    /// A receipt never exempts its payload from validation across operations.
    /// A corrupt existing object is not silently repaired using caller bytes.
    pub(super) fn inspect(&self, digest: Digest) -> io::Result<(Option<File>, bool, u64)> {
        let ready = self
            .receipt
            .open_file(&self.name)?
            .map(|mut f| Readiness::read(&mut f))
            .transpose()?;
        let Some(mut file) = self.payload.open_file(&self.name)? else {
            if ready.is_some() {
                return Err(invalid("receipt exists without payload"));
            }
            return Ok((None, false, 0));
        };
        let (actual, length) = Digest::of_reader(&mut file)?;
        if actual != digest {
            return Err(invalid("existing payload digest mismatch"));
        }
        file.rewind()?;
        if let Some(ready) = &ready {
            if ready.digest() != digest
                || ready.length() != length
                || ready.assurance() != Assurance::native()
            {
                return Err(invalid("receipt identity or assurance mismatch"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if file.metadata()?.permissions().mode() & 0o777 != 0o444 {
                    return Err(invalid("confirmed payload permissions changed"));
                }
            }
        }
        Ok((Some(file), ready.is_some(), length))
    }
    pub(super) fn new_receipt(
        &self,
        ready: &Readiness,
        id: [u8; 16],
        sync: &dyn Fn(&File) -> io::Result<()>,
    ) -> io::Result<StagedFile> {
        self.staging.verify()?;
        let wire = ready.encode()?;
        let record =
            StagedFile::copy_from_reader(self.staging.path(), &mut Cursor::new(wire), None, id)
                .map_err(|e| e.into_cause())?;
        record.sync_for_install(false, sync)?;
        Ok(record)
    }
    pub(super) fn sync_payload(&self, sync: &dyn Fn(&File) -> io::Result<()>) -> io::Result<()> {
        // Repeat directory barriers even when the name survived an earlier
        // failed mkdir barrier. Payload requalification still uses NEW bytes.
        self.payload.sync_using(sync)?;
        self.objects.sync_using(sync)?;
        self.root.sync_using(sync)
    }
    pub(super) fn sync_receipt(&self, sync: &dyn Fn(&File) -> io::Result<()>) -> io::Result<()> {
        self.receipt.sync_using(sync)?;
        self.receipts.sync_using(sync)?;
        self.root.sync_using(sync)
    }
}
