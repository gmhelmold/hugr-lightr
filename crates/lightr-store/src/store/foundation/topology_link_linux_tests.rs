//! Raw-byte namespace cases require Linux; no lossy decoding is accepted.
use super::*;

#[test]
fn source_link_linux_raw_name_is_not_lossily_rewritten() {
    let f = Fixture::new();
    let leaf = OsStr::from_bytes(b"raw-\xff");
    symlink("literal", f.source.join(leaf)).unwrap();
    assert_eq!(
        f.inspect()
            .read_source_link(Path::new(leaf), 100, Wait::Try)
            .unwrap(),
        "literal"
    );
}

#[test]
fn source_link_linux_non_utf8_target_is_rejected_not_recoded() {
    let f = Fixture::new();
    symlink(OsStr::from_bytes(b"bad-\xff"), f.source.join("leaf")).unwrap();
    assert_eq!(
        read(&f.inspect(), "leaf").unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        fs::read_link(f.source.join("leaf"))
            .unwrap()
            .as_os_str()
            .as_bytes(),
        b"bad-\xff"
    );
}

#[test]
fn source_link_linux_pin_preserves_hostile_replacement_as_link() {
    let f = Fixture::new();
    f.link("leaf", "original-target");
    let hostile_target = f.store.join("sentinel");
    let inspection = f.inspect();
    let mut replaced = false;
    let result = read_checked(&inspection, Path::new("leaf"), 4096, Wait::Try, |stage| {
        if stage == Stage::Checked && !replaced {
            replaced = true;
            fs::rename(f.source.join("leaf"), f.source.join("retained"))?;
            symlink(&hostile_target, f.source.join("leaf"))?;
        }
        Ok(())
    });
    assert!(replaced, "pin/open replacement checkpoint was not reached");
    assert_eq!(result.unwrap(), hostile_target.to_str().unwrap());
    assert_eq!(
        fs::read_link(f.source.join("retained")).unwrap(),
        Path::new("original-target")
    );
    assert_eq!(fs::read(&hostile_target).unwrap(), b"protected");
    fs::remove_file(f.source.join("leaf")).unwrap();
    assert!(fs::symlink_metadata(f.source.join("leaf")).is_err());
    assert_eq!(fs::read(&hostile_target).unwrap(), b"protected");
}
