//! Test-support only: batch BLAKE3 using the repository's locked implementation.
//! Reads one bounded hex message per line; never opens a Store or writes data.
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::BufWriter::new(io::stdout().lock());
    loop {
        let mut line = Vec::new();
        // Frames max out at 1 MiB; the larger ceiling accommodates hex/domain.
        let n = std::io::Read::take(&mut input, 2_200_001).read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        if n > 2_200_000 || line.last() != Some(&b'\n') {
            return Err("overlong or unterminated hash input".into());
        }
        line.pop();
        if line.len() % 2 != 0 {
            return Err("odd hex input".into());
        }
        let mut bytes = Vec::with_capacity(line.len() / 2);
        for pair in line.chunks_exact(2) {
            let text = std::str::from_utf8(pair)?;
            if !text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
                return Err("noncanonical hex input".into());
            }
            bytes.push(u8::from_str_radix(text, 16)?);
        }
        writeln!(output, "{}", lightr_core::Digest::of_bytes(&bytes).to_hex())?;
        output.flush()?;
    }
    Ok(())
}
