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
