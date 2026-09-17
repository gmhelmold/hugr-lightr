//! ADR-0020 readiness frame. Bounded reads, strict typed JSON and exact framing.
use lightr_core::Digest;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

const TAG: &[u8; 8] = b"\0\0LSIR01";
const DOMAIN: &[u8] = b"lightr/snapshot-integrity/metadata/v1/";
const LIMIT: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Assurance {
    #[serde(rename = "unix-file-directory-v1")]
    UnixFileDirectory,
    #[serde(rename = "windows-file-v1")]
    WindowsFile,
}

impl Assurance {
    pub fn native() -> Self {
        #[cfg(unix)] { Self::UnixFileDirectory }
        #[cfg(windows)] { Self::WindowsFile }
    }
}

/// Persistent evidence to validate under a lease; not a `PreparedObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readiness {
    digest: Digest,
    length: u64,
    assurance: Assurance,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    version: u32,
    digest: String,
    length: u64,
    assurance: Assurance,
}

pub(super) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

impl Readiness {
    pub fn new(digest: Digest, length: u64, assurance: Assurance) -> Self {
        Self { digest, length, assurance }
    }
    pub fn digest(&self) -> Digest { self.digest }
    pub fn length(&self) -> u64 { self.length }
    pub fn assurance(&self) -> Assurance { self.assurance }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let body = serde_json::to_vec(&Body { version: 1, digest: self.digest.to_hex(),
                                            length: self.length, assurance: self.assurance })?;
        if body.len() > LIMIT { return Err(invalid("readiness body exceeds bound")); }
        let mut frame = Vec::with_capacity(44 + body.len());
        frame.extend_from_slice(TAG);
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(&body);
        let mut checksum_input = Vec::with_capacity(DOMAIN.len() + frame.len());
        checksum_input.extend_from_slice(DOMAIN);
        checksum_input.extend_from_slice(&frame);
        frame.extend_from_slice(&Digest::of_bytes(&checksum_input).0);
        Ok(frame)
    }

    /// Reads at most 4,141 bytes, even for malicious streams. Length is checked
    /// before allocating/reading the body; an extra byte is an explicit error.
    pub fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut header = [0u8; 12];
        reader.read_exact(&mut header)?;
        if &header[..8] != TAG { return Err(invalid("bad readiness tag")); }
        let length = u32::from_le_bytes(header[8..].try_into().expect("fixed header")) as usize;
        if length > LIMIT { return Err(invalid("readiness declared length exceeds bound")); }
        let mut tail = vec![0u8; length + 32];
        reader.read_exact(&mut tail)?;
        let mut extra = [0];
        loop {
            match reader.read(&mut extra) {
                Ok(0) => break,
                Ok(_) => return Err(invalid("trailing readiness bytes")),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        let mut input = Vec::with_capacity(DOMAIN.len() + 12 + length);
        input.extend_from_slice(DOMAIN);
        input.extend_from_slice(&header);
        input.extend_from_slice(&tail[..length]);
        if Digest::of_bytes(&input).0.as_slice() != &tail[length..] {
            return Err(invalid("readiness checksum mismatch"));
        }
        let body: Body = serde_json::from_slice(&tail[..length])?;
        if body.version != 1 || body.digest.len() != 64 ||
            !body.digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(invalid("invalid readiness version or digest"));
        }
        let digest = Digest::from_hex(&body.digest).map_err(|e| invalid(e.to_string()))?;
        Ok(Self { digest, length: body.length, assurance: body.assurance })
    }
}
