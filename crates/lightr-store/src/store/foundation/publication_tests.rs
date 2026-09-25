use super::{
    Assurance, LeasedStagedFile, Phase, PreparationWork, PublicationOutcome, Readiness, StoreLocks,
    Wait,
};
use lightr_core::Digest;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub(super) fn path(root: &Path, family: &str, digest: Digest) -> PathBuf {
    let hex = digest.to_hex();
    root.join(family).join(&hex[..2]).join(&hex[2..])
}
pub(super) fn identity(path: &Path) -> super::lease_io::Identity {
    super::lease_io::identity(&File::open(path).unwrap()).unwrap()
}
pub(super) fn legacy(root: &Path, bytes: &[u8]) -> Digest {
    let digest = Digest::of_bytes(bytes);
    let dest = path(root, "objects", digest);
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(dest, bytes).unwrap();
    digest
}

#[test]
fn publication_new_payload_has_verified_receipt_and_borrowed_proof() {
    for n in [0, 1, 65536, 131073] {
        let root = TempDir::new().unwrap();
        let domain = StoreLocks::open_existing(root.path()).unwrap();
        let lease = domain.shared(Wait::Try).unwrap();
        let bytes = vec![61; n];
        let proof = lease
            .prepare_reader(&mut Cursor::new(&bytes), None, Wait::Try, [1; 16])
            .unwrap();
        assert_eq!(proof.digest(), Digest::of_bytes(&bytes));
        assert_eq!(proof.len(), n as u64);
        assert_eq!(proof.work(), PreparationWork::Created);
        assert_eq!(proof.assurance(), Assurance::native());
        let payload = path(root.path(), "objects", proof.digest());
        assert_eq!(fs::read(&payload).unwrap(), bytes);
        let mut receipt = File::open(path(root.path(), "objects-ready", proof.digest())).unwrap();
        let ready = Readiness::read(&mut receipt).unwrap();
        assert_eq!(ready.length(), proof.len());
        assert_eq!(ready.digest(), proof.digest());
        assert_eq!(
            domain.exclusive(Wait::Try).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            fs::read_dir(root.path().join(".si01-staging"))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn publication_legacy_object_is_rewritten_not_resynced() {
    let root = TempDir::new().unwrap();
    let digest = legacy(root.path(), b"legacy");
    let dest = path(root.path(), "objects", digest);
    let old = File::open(&dest).unwrap();
    let old_id = super::lease_io::identity(&old).unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease.prepare_existing(digest, Wait::Try, [2; 16]).unwrap();
    assert_ne!(
        old_id,
        identity(&dest),
        "requalification must install a freshly written file"
    );
    assert_eq!(proof.work(), PreparationWork::Requalified);
    assert_eq!(fs::read(&dest).unwrap(), b"legacy");
}

#[test]
fn publication_cross_lease_reuse_keeps_payload_inode_and_rebuilds_receipt() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let digest = Digest::of_bytes(b"immutable");
    {
        let lease = locks.shared(Wait::Try).unwrap();
        lease
            .prepare_reader(&mut Cursor::new(b"immutable"), None, Wait::Try, [3; 16])
            .unwrap();
    }
    let payload = path(root.path(), "objects", digest);
    let receipt = path(root.path(), "objects-ready", digest);
    let old_payload = File::open(&payload).unwrap();
    let old_receipt = File::open(&receipt).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease
        .prepare_reader(&mut Cursor::new(b"immutable"), None, Wait::Try, [4; 16])
        .unwrap();
    assert_eq!(
        super::lease_io::identity(&old_payload).unwrap(),
        identity(&payload),
        "confirmed payload must never be replaced"
    );
    assert_eq!(proof.work(), PreparationWork::Reused);
    assert_ne!(
        super::lease_io::identity(&old_receipt).unwrap(),
        identity(&receipt),
        "cross-operation receipt is reconstructed"
    );
}

#[test]
fn publication_missing_receipt_requires_fresh_payload_even_when_bytes_match() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let digest = Digest::of_bytes(b"bytes");
    {
        let lease = locks.shared(Wait::Try).unwrap();
        lease
            .prepare_reader(&mut Cursor::new(b"bytes"), None, Wait::Try, [5; 16])
            .unwrap();
    }
    let payload = path(root.path(), "objects", digest);
    let old = File::open(&payload).unwrap();
    fs::remove_file(path(root.path(), "objects-ready", digest)).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease.prepare_existing(digest, Wait::Try, [6; 16]).unwrap();
    assert_eq!(proof.work(), PreparationWork::Requalified);
    assert_ne!(super::lease_io::identity(&old).unwrap(), identity(&payload));
}

#[test]
fn publication_corrupt_existing_payload_is_not_overwritten_with_good_input() {
    let root = TempDir::new().unwrap();
    let digest = legacy(root.path(), b"good");
    let payload = path(root.path(), "objects", digest);
    fs::write(&payload, b"bad!").unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let err = lease
        .prepare_reader(&mut Cursor::new(b"good"), None, Wait::Try, [7; 16])
        .unwrap_err();
    assert_eq!(err.outcome, PublicationOutcome::RecoveryRequired);
    assert_eq!(err.original_io().kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read(&payload).unwrap(), b"bad!");
    assert!(!path(root.path(), "objects-ready", digest).exists());
}

#[test]
fn publication_bad_receipt_cannot_be_repaired_by_overwriting() {
    let root = TempDir::new().unwrap();
    let digest = legacy(root.path(), b"content");
    let receipt = path(root.path(), "objects-ready", digest);
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    let invalids = vec![
        b"not a receipt".to_vec(),
        Readiness::new(digest, 99, Assurance::native())
            .encode()
            .unwrap(),
        Readiness::new(Digest::of_bytes(b"other"), 7, Assurance::native())
            .encode()
            .unwrap(),
    ];
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    for bytes in invalids {
        fs::write(&receipt, &bytes).unwrap();
        let lease = locks.shared(Wait::Try).unwrap();
        assert!(lease.prepare_existing(digest, Wait::Try, [8; 16]).is_err());
        assert_eq!(fs::read(&receipt).unwrap(), bytes);
        assert_eq!(
            fs::read(path(root.path(), "objects", digest)).unwrap(),
            b"content"
        );
    }
}

#[test]
fn publication_receipt_without_payload_and_absent_digest_fail_closed() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let digest = Digest::of_bytes(b"missing");
    {
        let lease = locks.shared(Wait::Try).unwrap();
        assert_eq!(
            lease
                .prepare_existing(digest, Wait::Try, [9; 16])
                .unwrap_err()
                .original_io()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
    let receipt = path(root.path(), "objects-ready", digest);
    fs::write(
        &receipt,
        Readiness::new(digest, 7, Assurance::native())
            .encode()
            .unwrap(),
    )
    .unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    assert_eq!(
        lease
            .prepare_reader(&mut Cursor::new(b"missing"), None, Wait::Try, [10; 16])
            .unwrap_err()
            .outcome,
        PublicationOutcome::RecoveryRequired
    );
    assert!(!path(root.path(), "objects", digest).exists());
}

#[test]
fn publication_rejects_staged_mutation_before_payload_install() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let stage =
        LeasedStagedFile::copy(&lease, &mut Cursor::new(b"original"), None, [11; 16]).unwrap();
    let digest = stage.digest();
    let err = lease
        .prepare_with(digest, Some(stage), Wait::Try, [11; 16], &|phase, p| {
            if phase == Phase::Verify && p.file_name().is_some_and(|n| n == "payload") {
                fs::write(p, b"tampered")?;
            }
            Ok(())
        })
        .unwrap_err();
    assert_eq!(err.phase, Phase::Verify);
    assert_eq!(err.original_io().kind(), io::ErrorKind::InvalidData);
    assert!(!path(root.path(), "objects", digest).exists());
}

#[test]
fn publication_same_lease_cache_does_not_bypass_new_source_validation() {
    let root = TempDir::new().unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let digest = Digest::of_bytes(b"first");
    lease
        .prepare_reader(&mut Cursor::new(b"first"), None, Wait::Try, [12; 16])
        .unwrap();
    let err = lease
        .prepare_reader(
            &mut Cursor::new(b"other"),
            Some((digest, 5)),
            Wait::Try,
            [13; 16],
        )
        .unwrap_err();
    assert_eq!(err.phase, Phase::Verify);
    assert_eq!(
        fs::read(path(root.path(), "objects", digest)).unwrap(),
        b"first"
    );
    assert_eq!(
        lease
            .prepare_existing(digest, Wait::Try, [14; 16])
            .unwrap()
            .work(),
        PreparationWork::LeaseCached
    );
}

#[cfg(unix)]
#[test]
fn publication_rejects_symlink_payload_and_receipt_without_touching_target() {
    use std::os::unix::fs::symlink;
    for family in ["objects", "objects-ready"] {
        let root = TempDir::new().unwrap();
        let digest = Digest::of_bytes(b"protected");
        let outside = root.path().join("sentinel");
        fs::write(&outside, b"protected").unwrap();
        let link = path(root.path(), family, digest);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(&outside, &link).unwrap();
        let locks = StoreLocks::open_existing(root.path()).unwrap();
        let lease = locks.shared(Wait::Try).unwrap();
        assert!(lease
            .prepare_reader(&mut Cursor::new(b"protected"), None, Wait::Try, [15; 16])
            .is_err());
        assert_eq!(fs::read(outside).unwrap(), b"protected");
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }
}

#[test]
fn publication_preserves_readonly_source_attributes() {
    let root = TempDir::new().unwrap();
    let input = root.path().join("source");
    fs::write(&input, b"read only").unwrap();
    let original_permissions = fs::metadata(&input).unwrap().permissions();
    let mut perms = original_permissions.clone();
    perms.set_readonly(true);
    fs::set_permissions(&input, perms).unwrap();
    let locks = StoreLocks::open_existing(root.path()).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease
        .prepare_reader(&mut File::open(&input).unwrap(), None, Wait::Try, [16; 16])
        .unwrap();
    assert_eq!(proof.len(), 9);
    assert_eq!(fs::read(&input).unwrap(), b"read only");
    assert!(fs::metadata(&input).unwrap().permissions().readonly());
    #[cfg(windows)]
    {
        fs::set_permissions(input, original_permissions).unwrap();
    }
}
