use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub const KNOWN_SHA256: &str = "d07fdcde38cc4a1e57696332b68ff6f74bf8fb931659b71b414d28c1a89d8be0";
pub const KNOWN_BUILD: &str = "Steam build 26211342 / Alpha 5 patch 9 (0.5.9.65)";
const SETTER_FILE_OFFSET: u64 = 0x3F3E0F;
const SETTER_BYTES: [u8; 8] = [0xF2, 0x0F, 0x11, 0x05, 0x49, 0xAE, 0x25, 0x00];
const TARGET_FILE_OFFSET: u64 = 0x64EC60;
const TARGET_INITIAL_BYTES: [u8; 8] = 1f64.to_le_bytes();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub sha256: String,
    pub architecture: String,
    pub bytes: u64,
    pub recognized: bool,
}

pub fn inspect(path: &Path) -> Result<Fingerprint, String> {
    let mut file = File::open(path).map_err(|e| format!("Open executable: {e}"))?;
    let bytes = file
        .metadata()
        .map_err(|e| format!("File metadata: {e}"))?
        .len();
    let mut header = [0u8; 4096];
    let n = file
        .read(&mut header)
        .map_err(|e| format!("Read PE header: {e}"))?;
    let architecture = pe_architecture(&header[..n])?;
    let mut file = File::open(path).map_err(|e| format!("Reopen executable: {e}"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Hash executable: {e}"))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let sha256 = format!("{:x}", hash.finalize());
    let recognized =
        sha256 == KNOWN_SHA256 && architecture == "x86-64" && expected_profile_bytes(path)?;
    Ok(Fingerprint {
        sha256,
        architecture,
        bytes,
        recognized,
    })
}

fn expected_profile_bytes(path: &Path) -> Result<bool, String> {
    let mut file = File::open(path).map_err(|e| format!("Open profile bytes: {e}"))?;
    for (offset, expected) in [
        (SETTER_FILE_OFFSET, SETTER_BYTES),
        (TARGET_FILE_OFFSET, TARGET_INITIAL_BYTES),
    ] {
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Seek profile bytes: {e}"))?;
        let mut bytes = [0u8; 8];
        file.read_exact(&mut bytes)
            .map_err(|e| format!("Read profile bytes: {e}"))?;
        if bytes != expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn pe_architecture(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return Err("Not an MZ executable".into());
    }
    let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if pe.checked_add(6).is_none_or(|end| end > bytes.len()) || &bytes[pe..pe + 4] != b"PE\0\0" {
        return Err("Invalid or truncated PE header".into());
    }
    let machine = u16::from_le_bytes(bytes[pe + 4..pe + 6].try_into().unwrap());
    Ok(match machine {
        0x8664 => "x86-64".to_owned(),
        0x014c => "x86".to_owned(),
        other => format!("unknown (0x{other:04x})"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pe_parser_reads_machine_and_rejects_bad_headers() {
        let mut bytes = vec![0u8; 128];
        assert!(pe_architecture(&bytes).is_err());
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        assert_eq!(pe_architecture(&bytes).unwrap(), "x86-64");
    }
}
