//! Bounded Rust decoder for the accepted readiness wire contract.

use lightr_core::Digest;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

pub const MAX_READY_BODY: usize = 4096;
const HEADER: usize = 12;
const TRAILER: usize = 32;
const TAG: &[u8; 8] = b"\0\0LSIR01";
const DOMAIN: &[u8] = b"lightr/snapshot-integrity/metadata/v1/";

/// The profile stated in the record, not proof of destination qualification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Assurance {
    #[serde(rename = "unix-file-directory-v1")]
    UnixFileDirectory,
    #[serde(rename = "windows-file-v1")]
    WindowsFile,
}

impl Assurance {
    pub fn native() -> io::Result<Self> {
        if cfg!(unix) {
            Ok(Self::UnixFileDirectory)
        } else if cfg!(windows) {
            Ok(Self::WindowsFile)
        } else {
            Err(io::Error::new(io::ErrorKind::Unsupported, "no sync profile"))
        }
    }
}

/// Serialized data only. Constructing this value never certifies CAS bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessRecord {
    digest: Digest,
    length: u64,
    assurance: Assurance,
}

// Deserialize directly, not through Value: serde rejects duplicate fields and
// non-integer u64s; deny_unknown_fields rejects accidental schema promotion.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRecord {
    version: u8,
    digest: String,
    length: u64,
    assurance: Assurance,
}

impl ReadinessRecord {
    pub fn new(digest: Digest, length: u64, assurance: Assurance) -> Self {
        Self { digest, length, assurance }
    }

    pub fn digest(&self) -> Digest { self.digest }
    pub fn length(&self) -> u64 { self.length }
    pub fn assurance(&self) -> Assurance { self.assurance }

    /// Encode in the accepted fixed field order. This does not write a Store.
    pub fn encode(&self) -> io::Result<ReadinessFrame> {
        let body = serde_json::to_vec(&WireRecord {
            version: 1,
            digest: self.digest.to_hex(),
            length: self.length,
            assurance: self.assurance,
        }).map_err(invalid)?;
        if body.len() > MAX_READY_BODY { return Err(invalid("readiness body exceeds bound")); }
        let mut bytes = Vec::with_capacity(HEADER + body.len() + TRAILER);
        bytes.extend_from_slice(TAG);
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);
        let checksum = checksum(&bytes);
        bytes.extend_from_slice(&checksum.0);
        Ok(ReadinessFrame { record: self.clone(), bytes })
    }
}

/// Retains exact checked bytes, including legal whitespace/field order on read.
/// Re-emitting `as_bytes()` never invents a different historical identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessFrame {
    record: ReadinessRecord,
    bytes: Vec<u8>,
}

impl ReadinessFrame {
    pub fn record(&self) -> &ReadinessRecord { &self.record }
    pub fn as_bytes(&self) -> &[u8] { &self.bytes }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HEADER + TRAILER { return Err(invalid("truncated readiness frame")); }
        let size = body_len(&bytes[..HEADER])?;
        if bytes.len() != HEADER + size + TRAILER { return Err(invalid("readiness length/trailing bytes")); }
        let end = HEADER + size;
        if checksum(&bytes[..end]).0 != bytes[end..] { return Err(invalid("readiness checksum mismatch")); }
        let wire: WireRecord = serde_json::from_slice(&bytes[HEADER..end]).map_err(invalid)?;
        if wire.version != 1 { return Err(invalid("unsupported readiness version")); }
        if wire.digest.len() != 64 || !wire.digest.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
            return Err(invalid("noncanonical readiness digest"));
        }
        let digest = Digest::from_hex(&wire.digest).map_err(invalid)?;
        Ok(Self {
            record: ReadinessRecord::new(digest, wire.length, wire.assurance),
            bytes: bytes.to_vec(),
        })
    }

    /// Read a dedicated record stream. Declared size is checked before body
    /// allocation or reading; even an endless/hostile stream costs <= 4141 bytes.
    /// The caller must open a regular, non-symlink record under its own guard.
    pub fn read_from(mut reader: impl Read) -> io::Result<Self> {
        let mut header = [0u8; HEADER];
        reader.read_exact(&mut header)?;
        let size = body_len(&header)?;
        let mut bytes = vec![0; HEADER + size + TRAILER];
        bytes[..HEADER].copy_from_slice(&header);
        reader.read_exact(&mut bytes[HEADER..])?;
        let mut extra = [0u8; 1];
        loop {
            match reader.read(&mut extra) {
                Ok(0) => break,
                Ok(_) => return Err(invalid("trailing readiness bytes")),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Self::decode(&bytes)
    }
}

fn body_len(header: &[u8]) -> io::Result<usize> {
    if &header[..8] != TAG { return Err(invalid("unknown readiness tag")); }
    let size = u32::from_le_bytes(header[8..12].try_into().expect("fixed header")) as usize;
    if size > MAX_READY_BODY { return Err(invalid("readiness body exceeds bound")); }
    Ok(size)
}

fn checksum(prefix: &[u8]) -> Digest {
    // This control-metadata allocation is bounded, independent of payload size.
    let mut data = Vec::with_capacity(DOMAIN.len() + prefix.len());
    data.extend_from_slice(DOMAIN);
    data.extend_from_slice(prefix);
    Digest::of_bytes(&data)
}

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}
