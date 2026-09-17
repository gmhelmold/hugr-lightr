//! Exact-byte retention around the single shared readiness decoder from #155.
//! No second wire schema, serializer, digest parser or checksum implementation.

use super::Readiness;
use std::io::{self, Read};

pub const MAX_READY_BODY: usize = 4096;
const MAX_FRAME: usize = MAX_READY_BODY + 44;

/// Decoded metadata with its exact input bytes. This is not a preparation proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessFrame {
    record: Readiness,
    bytes: Vec<u8>,
}

impl ReadinessFrame {
    pub fn record(&self) -> &Readiness {
        &self.record
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        Self::read_from(bytes)
    }

    /// The canonical decoder bounds reads before allocation. The recorder also
    /// caps retained bytes (including its one-byte EOF probe) independently.
    pub fn read_from(reader: impl Read) -> io::Result<Self> {
        let mut capture = Capture {
            reader,
            bytes: Vec::new(),
        };
        let record = Readiness::read(&mut capture)?;
        Ok(Self {
            record,
            bytes: capture.bytes,
        })
    }
}

struct Capture<R> {
    reader: R,
    bytes: Vec<u8>,
}

impl<R: Read> Read for Capture<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let limit = output.len().min(MAX_FRAME + 1 - self.bytes.len());
        if limit == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "readiness capture exceeds bound",
            ));
        }
        let count = self.reader.read(&mut output[..limit])?;
        self.bytes.extend_from_slice(&output[..count]);
        Ok(count)
    }
}
