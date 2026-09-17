use super::{Assurance, ReadinessFrame, Readiness, MAX_READY_BODY};
use lightr_core::Digest;
use std::io::{self, Read};

fn hex(value: &str) -> Vec<u8> {
    value
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

fn specimens() -> [Vec<u8>; 2] {
    [
        hex(include_str!("vectors/ready-unix.hex")),
        hex(include_str!("vectors/ready-windows-u64-max.hex")),
    ]
}

// Independent test framing, including newly checksummed semantic defects.
fn framed(body: &[u8]) -> Vec<u8> {
    let mut wire = b"\0\0LSIR01".to_vec();
    wire.extend_from_slice(&(body.len() as u32).to_le_bytes());
    wire.extend_from_slice(body);
    let input = [
        b"lightr/snapshot-integrity/metadata/v1/".as_slice(),
        wire.as_slice(),
    ]
    .concat();
    wire.extend_from_slice(&Digest::of_bytes(&input).0);
    wire
}

fn good_json() -> String {
    format!(
        r#"{{"version":1,"digest":"{}","length":1024,"assurance":"unix-file-directory-v1"}}"#,
        "11".repeat(32)
    )
}

#[test]
fn frozen_si00_readiness_vectors_are_exact() {
    let vectors = specimens();
    let expected = [
        Readiness::new(Digest([0x11; 32]), 1024, Assurance::UnixFileDirectory),
        Readiness::new(Digest([0x22; 32]), u64::MAX, Assurance::WindowsFile),
    ];
    for (wire, expected) in vectors.iter().zip(expected) {
        let frame = ReadinessFrame::decode(wire).unwrap();
        assert_eq!(frame.record(), &expected);
        assert_eq!(frame.as_bytes(), wire);
        assert_eq!(expected.encode().unwrap().as_slice(), wire);
        assert_eq!(ReadinessFrame::read_from(wire.as_slice()).unwrap(), frame);
        assert_eq!(frame.record().digest(), expected.digest());
        assert_eq!(frame.record().length(), expected.length());
        assert_eq!(frame.record().assurance(), expected.assurance());
    }
}

#[test]
fn every_frozen_prefix_and_trailing_byte_is_rejected() {
    for wire in specimens() {
        for end in 0..wire.len() {
            assert!(
                ReadinessFrame::decode(&wire[..end]).is_err(),
                "prefix {end}"
            );
            assert!(
                ReadinessFrame::read_from(&wire[..end]).is_err(),
                "stream prefix {end}"
            );
        }
        let mut extra = wire;
        extra.push(0);
        assert!(ReadinessFrame::decode(&extra).is_err());
        assert!(ReadinessFrame::read_from(extra.as_slice()).is_err());
    }
}

#[test]
fn every_wire_byte_participates_in_validation() {
    let wire = specimens()[0].clone();
    for at in 0..wire.len() {
        let mut bad = wire.clone();
        bad[at] ^= 1;
        assert!(ReadinessFrame::decode(&bad).is_err(), "byte {at}");
    }
}

#[test]
fn resigned_invalid_schema_is_rejected_not_just_bad_checksums() {
    let good = good_json();
    let bad = [
        good.replace("\"version\":1", "\"version\":true"),
        good.replace("\"version\":1", "\"version\":1.0"),
        good.replace("\"version\":1", "\"version\":2"),
        good.replace("\"length\":1024", "\"length\":true"),
        good.replace("\"length\":1024", "\"length\":1.0"),
        good.replace("\"length\":1024", "\"length\":1e3"),
        good.replace("\"length\":1024", "\"length\":-1"),
        good.replace("\"length\":1024", "\"length\":18446744073709551616"),
        good.replace("\"length\":1024", "\"length\":null"),
        good.replace("\"length\":1024", "\"length\":NaN"),
        good.replace("11", "FF"),
        good.replace(&"11".repeat(32), "../not-a-digest"),
        good.replace("unix-file-directory-v1", "powerloss-certified"),
        good.replace("\"length\":1024,", ""),
        good.replace("\"length\":1024", "\"length\":1024,\"extra\":0"),
        good.replace("\"length\":1024", "\"length\":1024,\"length\":1024"),
        good.replace("\"version\":1", "\"version\":1,\"ver\\u0073ion\":1"),
        "[]".to_owned(),
        "null".to_owned(),
    ];
    for body in bad {
        assert!(
            ReadinessFrame::decode(&framed(body.as_bytes())).is_err(),
            "accepted {body}"
        );
    }
    assert!(ReadinessFrame::decode(&framed(b"\xff")).is_err());
}

#[test]
fn exact_bound_is_accepted_and_exact_read_bytes_preserved() {
    let mut body = good_json().into_bytes();
    body.resize(MAX_READY_BODY, b' ');
    let wire = framed(&body);
    let decoded = ReadinessFrame::read_from(wire.as_slice()).unwrap();
    assert_eq!(decoded.as_bytes(), wire);
    assert_ne!(decoded.record().encode().unwrap().as_slice(), wire);
    body.push(b' ');
    assert!(ReadinessFrame::decode(&framed(&body)).is_err());
}

struct HeaderOnly {
    bytes: io::Cursor<Vec<u8>>,
}
impl Read for HeaderOnly {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        assert!(
            self.bytes.position() < 12,
            "reader accessed body before bounding declared length"
        );
        self.bytes.read(output)
    }
}

#[test]
fn excessive_length_is_rejected_before_reading_body() {
    for size in [(MAX_READY_BODY + 1) as u32, u32::MAX] {
        let mut header = b"\0\0LSIR01".to_vec();
        header.extend_from_slice(&size.to_le_bytes());
        let source = HeaderOnly {
            bytes: io::Cursor::new(header),
        };
        assert_eq!(
            ReadinessFrame::read_from(source).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}

struct BrokenTail {
    bytes: io::Cursor<Vec<u8>>,
}
impl Read for BrokenTail {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.bytes.position() as usize == self.bytes.get_ref().len() {
            return Err(io::Error::from_raw_os_error(5));
        }
        self.bytes.read(output)
    }
}

#[test]
fn trailing_io_error_is_not_misread_as_eof() {
    let source = BrokenTail {
        bytes: io::Cursor::new(specimens()[0].clone()),
    };
    assert_eq!(
        ReadinessFrame::read_from(source)
            .unwrap_err()
            .raw_os_error(),
        Some(5)
    );
}

#[test]
fn zero_length_and_all_digest_bytes_roundtrip() {
    for byte in [0, 1, 127, 128, 255] {
        let record = Readiness::new(Digest([byte; 32]), 0, Assurance::native());
        assert_eq!(
            ReadinessFrame::decode(record.encode().unwrap().as_slice())
                .unwrap()
                .record(),
            &record
        );
    }
}
