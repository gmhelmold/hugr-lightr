use super::error::LightrError;
use super::error::Result;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub fn of_bytes(data: &[u8]) -> Self {
        Digest(*blake3::hash(data).as_bytes())
    }

    pub fn of_file(path: &Path) -> Result<Self> {
        let mut hasher = blake3::Hasher::new();
        hasher.update_mmap_rayon(path)?;
        Ok(Digest(*hasher.finalize().as_bytes()))
    }

    /// Hash a reader without mapping mutable files or buffering the whole input.
    /// Returns the digest and the actual byte count; interrupted reads are retried.
    pub fn of_reader(reader: &mut impl std::io::Read) -> std::io::Result<(Self, u64)> {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut length = 0u64;
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            length = length.checked_add(count as u64).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "stream length overflow")
            })?;
            hasher.update(&buffer[..count]);
        }
        Ok((Self(*hasher.finalize().as_bytes()), length))
    }

    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        if s.len() != 64 {
            return Err(LightrError::InvalidManifest(format!(
                "invalid digest hex: {s}"
            )));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0])
                .ok_or_else(|| LightrError::InvalidManifest(format!("invalid digest hex: {s}")))?;
            let lo = hex_nibble(chunk[1])
                .ok_or_else(|| LightrError::InvalidManifest(format!("invalid digest hex: {s}")))?;
            bytes[i] = (hi << 4) | lo;
        }
        Ok(Digest(bytes))
    }
}

pub(super) fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl std::fmt::Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
