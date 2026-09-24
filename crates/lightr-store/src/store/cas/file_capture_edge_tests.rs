use super::*;
use std::io::Read;
use tempfile::TempDir;

#[test]
fn file_capture_opened_source_is_not_reinterpreted_by_path() {
    for mode in [CaptureMode::CloneOrCopy, CaptureMode::CopyOnly] {
        let root = TempDir::new().unwrap();
        let source = root.path().join("source");
        let moved = root.path().join("moved");
        let staging = root.path().join("staging");
        fs::create_dir(&staging).unwrap();
        fs::write(&source, b"opened source bytes").unwrap();
        let input = File::open(&source).unwrap();
        fs::rename(&source, &moved).unwrap();
        fs::write(&source, b"replacement pathname bytes").unwrap();
        let (mut stage, _) = StagedFile::capture_checked(
            &staging,
            input,
            mode,
            Some((Digest::of_bytes(b"opened source bytes"), 19)),
            [7; 16],
            &|| Ok(()),
        )
        .unwrap();
        let mut bytes = Vec::new();
        stage.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"opened source bytes");
        assert_eq!(fs::read(&source).unwrap(), b"replacement pathname bytes");
        assert_eq!(fs::read(&moved).unwrap(), b"opened source bytes");
    }
}

#[test]
fn file_capture_unknown_clone_scratch_blocks_cleanup_and_fallback() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let staging = root.path().join("staging");
    fs::create_dir(&staging).unwrap();
    fs::write(&source, b"input").unwrap();
    let effects = CaptureEffects {
        clone_file: |_, path, _| {
            fs::write(path, b"partial")?;
            fs::write(path.parent().unwrap().join("unknown"), b"keep")?;
            Err(io::ErrorKind::Unsupported.into())
        },
        sync: &File::sync_all,
        checkpoint: &|| Ok(()),
    };
    let result = StagedFile::capture_with(
        &staging,
        File::open(&source).unwrap(),
        CaptureMode::CloneOrCopy,
        None,
        [9; 16],
        effects,
    );
    assert!(
        result.is_err(),
        "uncertain clone cleanup must not proceed to fallback"
    );
    let allocations: Vec<_> = fs::read_dir(&staging)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(allocations.len(), 1);
    assert_eq!(fs::read(allocations[0].join("unknown")).unwrap(), b"keep");
    assert!(!allocations[0].join("payload").exists());
    assert_eq!(fs::read(source).unwrap(), b"input");
}
