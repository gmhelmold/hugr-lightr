use super::*;
use lightr_core::Digest;
use std::io::{self, Read};

fn resigned(body: &[u8]) -> Vec<u8> {
    let mut data = b"\0\0LSIR01".to_vec();
    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body);
    let mut digest = b"lightr/snapshot-integrity/metadata/v1/".to_vec();
    digest.extend_from_slice(&data);
    data.extend_from_slice(&Digest::of_bytes(&digest).0);
    data
}
fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes().chunks(2).map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(),16).unwrap()).collect()
}

#[test]
fn ready_matches_frozen_si00_golden_bytes() {
    let values: Vec<serde_json::Value> = serde_json::from_str(include_str!("ready-golden.json")).unwrap();
    assert_eq!(values.len(), 2);
    for value in values {
        let raw = unhex(value["wire_hex"].as_str().unwrap());
        let receipt = Readiness::read(&mut raw.as_slice()).unwrap();
        assert_eq!(receipt.encode().unwrap(), raw);
        assert_eq!(receipt.digest().to_hex(), value["body"]["digest"].as_str().unwrap());
        assert_eq!(receipt.length(), value["body"]["length"].as_u64().unwrap());
        for end in 0..raw.len() {
            assert!(Readiness::read(&mut &raw[..end]).is_err(), "accepted truncated prefix {end}");
        }
        let mut trailing = raw.clone(); trailing.push(0);
        assert!(Readiness::read(&mut trailing.as_slice()).is_err());
        let mut damaged = raw; let end = damaged.len()-1; damaged[end]^=1;
        assert!(Readiness::read(&mut damaged.as_slice()).is_err());
    }
}

#[test]
fn readiness_rejects_resigned_semantic_corruption() {
    let normal = format!(r#"{{"version":1,"digest":"{}","length":1,"assurance":"unix-file-directory-v1"}}"#, "11".repeat(32));
    for body in [normal.replace("\"version\":1", "\"version\":true"),
                 normal.replace("\"version\":1", "\"version\":2"),
                 normal.replace("\"length\":1", "\"length\":1.0"),
                 normal.replace("\"length\":1", "\"length\":-1"),
                 normal.replace("\"length\":1", "\"length\":18446744073709551616"),
                 normal.replace("\"length\":1", "\"length\":true"),
                 normal.replace("\"length\":1", "\"length\":1,\"length\":1"),
                 normal.replace("\"length\":1", "\"length\":1,\"extra\":0"),
                 normal.replace("11", "FF"), normal.replace("unix-file-directory-v1", "powerloss") ] {
        assert!(Readiness::read(&mut resigned(body.as_bytes()).as_slice()).is_err(), "accepted {body}");
    }
    assert!(Readiness::read(&mut resigned(b"\xff").as_slice()).is_err());
    let mut full = normal.into_bytes(); full.resize(4096,b' ');
    assert!(Readiness::read(&mut resigned(&full).as_slice()).is_ok());
}

#[test]
fn oversized_header_is_rejected_before_reading_body() {
    struct HeaderOnly { header: io::Cursor<Vec<u8>>, reads: usize }
    impl Read for HeaderOnly {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            assert!(self.header.position()<12,"attempt to read overlength body");
            self.header.read(bytes)
        }
    }
    let mut header = b"\0\0LSIR01".to_vec(); header.extend_from_slice(&4097u32.to_le_bytes());
    let mut reader = HeaderOnly { header:io::Cursor::new(header), reads:0 };
    assert!(Readiness::read(&mut reader).is_err()); assert_eq!(reader.reads,1);
}

#[test]
fn streaming_hash_matches_bytes_and_preserves_reader_failure() {
    for length in [0,1,63,64,65535,65536,65537,200000] {
        let data:Vec<u8>=(0..length).map(|n| (n%251) as u8).collect();
        assert_eq!(Digest::of_reader(&mut data.as_slice()).unwrap(), (Digest::of_bytes(&data),length as u64));
    }
    struct Broken;
    impl Read for Broken { fn read(&mut self,_:&mut[u8])->io::Result<usize>{Err(io::Error::from_raw_os_error(5))} }
    assert_eq!(Digest::of_reader(&mut Broken).unwrap_err().raw_os_error(),Some(5));
    struct Interrupted(bool);
    impl Read for Interrupted {
        fn read(&mut self,_:&mut[u8])->io::Result<usize>{
            if self.0 { self.0=false;Err(io::ErrorKind::Interrupted.into()) } else {Ok(0)}
        }
    }
    assert_eq!(Digest::of_reader(&mut Interrupted(true)).unwrap().0,Digest::of_bytes(b""));
}

#[test]
fn structured_failure_survives_legacy_wrapper_and_source_chain() {
    use std::error::Error;
    let original = PublicationFailure::new([7;16],PublicationOutcome::NotPublished,Phase::PayloadSync,None,io::Error::from_raw_os_error(5));
    let legacy = original.into_legacy();
    let recovered = PublicationFailure::from_legacy(&legacy).unwrap();
    assert_eq!(recovered.original_io().raw_os_error(),Some(5));
    assert_eq!(recovered.operation_id,[7;16]);
    assert_eq!(recovered.phase,Phase::PayloadSync);
    assert_eq!(recovered.source().unwrap().downcast_ref::<io::Error>().unwrap().raw_os_error(),Some(5));
}
