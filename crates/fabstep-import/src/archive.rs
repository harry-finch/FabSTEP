use crate::{error, MAX_INPUT_BYTES};
use fabstep_core::FabError;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

const MAX_ENTRIES: usize = 4096;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_PART: usize = 64 * 1024 * 1024;

fn invalid() -> FabError {
    error(
        "invalid_archive",
        "Archive ZIP invalide, ambiguë ou non prise en charge.",
    )
}
fn limit() -> FabError {
    error(
        "archive_limit",
        "Limite ZIP dépassée (64 Mio par partie, 128 Mio cumulés, 4 096 entrées).",
    )
}
fn u16_at(b: &[u8], p: usize) -> Result<usize, FabError> {
    Ok(u16::from_le_bytes(b.get(p..p + 2).ok_or_else(invalid)?.try_into().unwrap()) as usize)
}
fn u32_at(b: &[u8], p: usize) -> Result<usize, FabError> {
    Ok(u32::from_le_bytes(b.get(p..p + 4).ok_or_else(invalid)?.try_into().unwrap()) as usize)
}

/// Deliberately accepts canonical package names only. No host paths, URI
/// schemes, escaping or dot-segment resolution is ever delegated to a filesystem.
pub(crate) fn part_name(name: &str, absolute: bool) -> Result<String, FabError> {
    let s = if absolute {
        name.strip_prefix('/').unwrap_or(name)
    } else {
        name
    };
    if s.is_empty()
        || s.len() > 1024
        || s.chars().any(|c| c.is_control() || "\\:%?#".contains(c))
        || s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(error(
            "invalid_path",
            "Nom de partie ou cible OPC non canonique.",
        ));
    }
    Ok(s.to_owned())
}

pub(crate) struct Package<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    names: BTreeMap<String, String>,
    read_bytes: usize,
}
impl<'a> Package<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self, FabError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(error("archive_limit", "Archive limitée à 64 Mio."));
        }
        // Preflight ZIP32 before ZipArchive allocates its directory. No fallback
        // to a second EOCD interpretation; offsets must match this exact buffer.
        if bytes.len() < 22 {
            return Err(invalid());
        }
        let first = bytes.len().saturating_sub(65535 + 22);
        let eocd = (first..=bytes.len() - 22)
            .rev()
            .find(|&p| {
                bytes[p..p + 4] == *b"PK\x05\x06"
                    && u16_at(bytes, p + 20).is_ok_and(|n| p + 22 + n == bytes.len())
            })
            .ok_or_else(invalid)?;
        let count = u16_at(bytes, eocd + 10)?;
        if u16_at(bytes, eocd + 4)? != 0
            || u16_at(bytes, eocd + 6)? != 0
            || u16_at(bytes, eocd + 8)? != count
        {
            return Err(invalid());
        }
        if count == 0 || count > MAX_ENTRIES {
            return Err(limit());
        }
        let start = u32_at(bytes, eocd + 16)?;
        let size = u32_at(bytes, eocd + 12)?;
        if start.checked_add(size) != Some(eocd) || size > 8 * 1024 * 1024 {
            return Err(invalid());
        }
        let mut pos = start;
        let mut total = 0_u64;
        let mut names = BTreeMap::new();
        let mut unique = BTreeSet::new();
        let mut ranges = Vec::new();
        for _ in 0..count {
            if bytes.get(pos..pos + 4) != Some(b"PK\x01\x02") {
                return Err(invalid());
            }
            let flags = u16_at(bytes, pos + 8)?;
            let method = u16_at(bytes, pos + 10)?;
            if flags & (1 | 64) != 0 || ![0, 8].contains(&method) {
                return Err(error(
                    "unsupported_compression",
                    "Seules les parties ZIP non chiffrées Stored/Deflate sont prises en charge.",
                ));
            }
            let compressed = u32_at(bytes, pos + 20)?;
            let expanded = u32_at(bytes, pos + 24)?;
            if expanded > MAX_PART {
                return Err(limit());
            }
            total = total.checked_add(expanded as u64).ok_or_else(limit)?;
            if total > MAX_TOTAL {
                return Err(limit());
            }
            let name_len = u16_at(bytes, pos + 28)?;
            let extra_len = u16_at(bytes, pos + 30)?;
            let comment_len = u16_at(bytes, pos + 32)?;
            if u16_at(bytes, pos + 34)? != 0 {
                return Err(invalid());
            }
            let raw_name = bytes
                .get(pos + 46..pos + 46 + name_len)
                .ok_or_else(invalid)?;
            let name = std::str::from_utf8(raw_name).map_err(|_| invalid())?;
            let directory = name.ends_with('/');
            let canonical = part_name(name.trim_end_matches('/'), false)?;
            let key = canonical.to_ascii_lowercase();
            if !unique.insert(key.clone()) {
                return Err(error(
                    "duplicate_part",
                    "Parties ZIP dupliquées ou ambiguës par leur casse.",
                ));
            }
            if !directory {
                names.insert(key, canonical);
            }
            let local = u32_at(bytes, pos + 42)?;
            if bytes.get(local..local + 4) != Some(b"PK\x03\x04")
                || u16_at(bytes, local + 6)? != flags
                || u16_at(bytes, local + 8)? != method
            {
                return Err(invalid());
            }
            let local_name = u16_at(bytes, local + 26)?;
            let local_extra = u16_at(bytes, local + 28)?;
            if bytes.get(local + 30..local + 30 + local_name) != Some(raw_name) {
                return Err(invalid());
            }
            if flags & 8 == 0
                && (u32_at(bytes, local + 18)? != compressed
                    || u32_at(bytes, local + 22)? != expanded
                    || u32_at(bytes, local + 14)? != u32_at(bytes, pos + 16)?)
            {
                return Err(invalid());
            }
            let data_start = local
                .checked_add(30 + local_name + local_extra)
                .ok_or_else(invalid)?;
            let mut end = data_start.checked_add(compressed).ok_or_else(invalid)?;
            if end > start {
                return Err(invalid());
            }
            if flags & 8 != 0 {
                let crc = u32_at(bytes, pos + 16)?;
                // ZIP32 data descriptor has an optional signature. Verify
                // both layouts against the central record, never merely skip
                // these bytes or leave them out of overlap checks.
                let matches = |at: usize| -> bool {
                    at.checked_add(12).is_some_and(|n| n <= start)
                        && u32_at(bytes, at).is_ok_and(|v| v == crc)
                        && u32_at(bytes, at + 4).is_ok_and(|v| v == compressed)
                        && u32_at(bytes, at + 8).is_ok_and(|v| v == expanded)
                };
                let unsigned = matches(end);
                let signed = bytes.get(end..end + 4) == Some(b"PK\x07\x08") && matches(end + 4);
                let length = match (unsigned, signed) {
                    (true, false) => 12,
                    (false, true) => 16,
                    _ => return Err(invalid()),
                };
                end += length;
                for (offset, expected) in [(14, crc), (18, compressed), (22, expanded)] {
                    let local_value = u32_at(bytes, local + offset)?;
                    if local_value != 0 && local_value != expected {
                        return Err(invalid());
                    }
                }
            }
            ranges.push((local, end));
            pos = pos
                .checked_add(46 + name_len + extra_len + comment_len)
                .ok_or_else(invalid)?;
            if pos > eocd {
                return Err(invalid());
            }
        }
        if pos != eocd {
            return Err(invalid());
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|r| r[0].1 > r[1].0) {
            return Err(invalid());
        }
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid())?;
        if zip.len() != count || zip.metadata().has_duplicate_names() {
            return Err(invalid());
        }
        Ok(Self {
            zip,
            names,
            read_bytes: 0,
        })
    }
    pub fn read(&mut self, name: &str) -> Result<String, FabError> {
        let original = self
            .names
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| error("missing_part", "Partie OPC requise absente de l'archive."))?;
        let mut file = self.zip.by_name(original).map_err(|_| invalid())?;
        if file.size() > MAX_PART as u64 {
            return Err(limit());
        }
        let mut data = Vec::with_capacity((file.size() as usize).min(MAX_PART));
        (&mut file)
            .take((MAX_PART + 1) as u64)
            .read_to_end(&mut data)
            .map_err(|_| invalid())?;
        if data.len() > MAX_PART {
            return Err(limit());
        }
        if data.len() as u64 != file.size() {
            return Err(invalid());
        }
        self.read_bytes = self.read_bytes.checked_add(data.len()).ok_or_else(limit)?;
        if self.read_bytes as u64 > MAX_TOTAL {
            return Err(limit());
        }
        String::from_utf8(data)
            .map_err(|_| error("invalid_xml", "Le XML 3MF doit être encodé en UTF-8."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(out: &mut Vec<u8>, v: u16) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    fn dword(out: &mut Vec<u8>, v: u32) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    // Independent ZIP32 Stored fixture with a descriptor, not ZipWriter output.
    fn descriptor_zip(signed: bool, bad_crc: bool, bad_size: bool) -> Vec<u8> {
        let name = b"x.xml";
        let crc = 0x352441c2;
        let mut b = Vec::new();
        dword(&mut b, 0x04034b50);
        word(&mut b, 20);
        word(&mut b, 8);
        word(&mut b, 0);
        word(&mut b, 0);
        word(&mut b, 0);
        dword(&mut b, 0);
        dword(&mut b, 0);
        dword(&mut b, 0);
        word(&mut b, name.len() as u16);
        word(&mut b, 0);
        b.extend_from_slice(name);
        b.extend_from_slice(b"abc");
        if signed {
            dword(&mut b, 0x08074b50);
        }
        dword(&mut b, crc + u32::from(bad_crc));
        dword(&mut b, 3 + u32::from(bad_size));
        dword(&mut b, 3);
        let central = b.len() as u32;
        dword(&mut b, 0x02014b50);
        word(&mut b, 20);
        word(&mut b, 20);
        word(&mut b, 8);
        word(&mut b, 0);
        word(&mut b, 0);
        word(&mut b, 0);
        dword(&mut b, crc);
        dword(&mut b, 3);
        dword(&mut b, 3);
        word(&mut b, name.len() as u16);
        word(&mut b, 0);
        word(&mut b, 0);
        word(&mut b, 0);
        word(&mut b, 0);
        dword(&mut b, 0);
        dword(&mut b, 0);
        b.extend_from_slice(name);
        let size = b.len() as u32 - central;
        dword(&mut b, 0x06054b50);
        word(&mut b, 0);
        word(&mut b, 0);
        word(&mut b, 1);
        word(&mut b, 1);
        dword(&mut b, size);
        dword(&mut b, central);
        word(&mut b, 0);
        b
    }
    #[test]
    fn descriptors_are_verified_with_and_without_the_optional_signature() {
        for signed in [false, true] {
            let bytes = descriptor_zip(signed, false, false);
            assert_eq!(Package::open(&bytes).unwrap().read("x.xml").unwrap(), "abc");
            for (crc, size) in [(true, false), (false, true)] {
                let bytes = descriptor_zip(signed, crc, size);
                assert_eq!(
                    Package::open(&bytes).err().unwrap().code,
                    "threemf.invalid_archive"
                );
            }
        }
    }
}
