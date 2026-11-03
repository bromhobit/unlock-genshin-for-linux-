//! Bounded live PE and x86-64 FPS resolver.
use std::ops::Range;

const PATTERN: &[u8] = &[0xB9, 0x3C, 0, 0, 0, 0xE8];
const CHUNK: usize = 64 * 1024;
const MAX_IMAGE: usize = 1024 * 1024 * 1024;

fn u16_at(b: &[u8], n: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(n..n.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32_at(b: &[u8], n: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(n..n.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn relative(at: usize, len: usize, disp: i32) -> Option<usize> {
    at.checked_add(len)?.checked_add_signed(disp as isize)
}

#[derive(Debug)]
pub struct Pe {
    pub size: usize,
    pub il2cpp: Range<usize>,
}

pub fn parse(header: &[u8]) -> Result<Pe, &'static str> {
    if header.get(..2) != Some(b"MZ") {
        return Err("missing MZ header");
    }
    let nt = u32_at(header, 0x3c).ok_or("short DOS header")? as usize;
    if header
        .get(nt..nt.checked_add(24).ok_or("NT offset overflow")?)
        .is_none()
        || header.get(nt..nt + 4) != Some(b"PE\0\0")
    {
        return Err("missing PE signature");
    }
    let sections = u16_at(header, nt + 6).ok_or("short COFF header")? as usize;
    let optional_len = u16_at(header, nt + 20).ok_or("short COFF header")? as usize;
    if sections == 0 || sections > 96 || optional_len < 0x70 {
        return Err("invalid PE headers");
    }
    let optional = nt.checked_add(24).ok_or("PE offset overflow")?;
    let table = optional
        .checked_add(optional_len)
        .ok_or("PE offset overflow")?;
    let table_end = table
        .checked_add(sections * 40)
        .ok_or("section table overflow")?;
    if table_end > header.len() {
        return Err("section table outside bounded header");
    }
    if u16_at(header, optional) != Some(0x20b) || u16_at(header, nt + 4) != Some(0x8664) {
        return Err("not PE32+ AMD64");
    }
    let size = u32_at(header, optional + 56).ok_or("short optional header")? as usize;
    if !(0x1000..=MAX_IMAGE).contains(&size) || table_end > size {
        return Err("invalid or excessive PE image size");
    }
    let mut il2cpp = None;
    for i in 0..sections {
        let at = table + i * 40;
        if header[at..at + 8] == *b"il2cpp\0\0" {
            if il2cpp.is_some() {
                return Err("duplicate il2cpp section");
            }
            let len = u32_at(header, at + 8).ok_or("short section")? as usize;
            let start = u32_at(header, at + 12).ok_or("short section")? as usize;
            let end = start.checked_add(len).ok_or("section overflow")?;
            if len == 0 || end > size {
                return Err("il2cpp section outside image");
            }
            il2cpp = Some(start..end);
        }
    }
    Ok(Pe {
        size,
        il2cpp: il2cpp.ok_or("missing il2cpp section")?,
    })
}

fn read<const N: usize>(
    at: usize,
    pe: &Pe,
    reader: &mut impl FnMut(usize, &mut [u8]) -> Result<(), String>,
) -> Option<[u8; N]> {
    if at.checked_add(N)? > pe.size {
        return None;
    }
    let mut b = [0; N];
    reader(at, &mut b).ok()?;
    Some(b)
}

fn target_for(
    call: usize,
    pe: &Pe,
    reader: &mut impl FnMut(usize, &mut [u8]) -> Result<(), String>,
) -> Option<usize> {
    let d = i32::from_le_bytes(read::<4>(call + 1, pe, reader)?);
    let dest = relative(call, 5, d)?;
    if read::<1>(dest, pe, reader)? != [0xe9] {
        return None;
    }
    let mut rip = call;
    let mut visited = [usize::MAX; 16];
    for index in 0..visited.len() {
        let op = read::<1>(rip, pe, reader)?[0];
        if op != 0xe8 && op != 0xe9 {
            break;
        }
        if visited[..index].contains(&rip) {
            return None;
        }
        visited[index] = rip;
        let d = i32::from_le_bytes(read::<4>(rip + 1, pe, reader)?);
        rip = relative(rip, 5, d)?;
    }
    if read::<2>(rip, pe, reader)? != [0x89, 0x0d] {
        return None;
    }
    let d = i32::from_le_bytes(read::<4>(rip + 2, pe, reader)?);
    let target = relative(rip, 6, d)?;
    (target.is_multiple_of(4) && target.checked_add(4)? <= pe.size).then_some(target)
}

/// Scan readable mapped intervals only. Each interval is a contiguous readable byte range
/// relative to image base. All reads, including thunk reads, go through the validated reader.
pub fn resolve(
    pe: &Pe,
    ranges: &[Range<usize>],
    mut reader: impl FnMut(usize, &mut [u8]) -> Result<(), String>,
) -> Result<usize, &'static str> {
    let mut found = None;
    let mut chunk = vec![0; CHUNK + PATTERN.len() - 1];
    for range in ranges {
        let start = range.start.max(pe.il2cpp.start);
        let stop = range.end.min(pe.il2cpp.end);
        let mut at = start;
        while at.checked_add(PATTERN.len()).is_some_and(|n| n <= stop) {
            let len = (stop - at).min(chunk.len());
            if reader(at, &mut chunk[..len]).is_err() {
                break;
            }
            for i in 0..=len - PATTERN.len() {
                if chunk[i..i + PATTERN.len()] != *PATTERN {
                    continue;
                }
                if let Some(target) = target_for(at + i + 5, pe, &mut reader) {
                    // Multiple call sites may legitimately reach the same FPS variable.
                    // Only *different* validated target addresses are ambiguous.
                    match found {
                        None => found = Some(target),
                        Some(previous) if previous == target => (),
                        Some(_) => return Err("ambiguous FPS signature matches"),
                    }
                }
            }
            at += len - (PATTERN.len() - 1);
        }
    }
    found.ok_or("no safe FPS signature match")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(size: u32) -> Vec<u8> {
        let mut b = vec![0; 8192];
        b[..2].copy_from_slice(b"MZ");
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        b[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        b[0x94..0x96].copy_from_slice(&0xf0u16.to_le_bytes());
        b[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        b[0xd0..0xd4].copy_from_slice(&size.to_le_bytes());
        b[0x188..0x190].copy_from_slice(b"il2cpp\0\0");
        b[0x190..0x194].copy_from_slice(&0x131a05b0u32.to_le_bytes());
        b[0x194..0x198].copy_from_slice(&0x6c04000u32.to_le_bytes());
        b
    }
    #[test]
    fn accepts_large_sparse_image_and_chunk_boundary() {
        let pe = parse(&header(0x1aff7000)).unwrap();
        let mut bytes = vec![0; CHUNK * 2];
        let pos = CHUNK - 3;
        bytes[pos..pos + 6].copy_from_slice(PATTERN);
        bytes[pos + 6..pos + 10].copy_from_slice(&(0x100i32 - (pos as i32 + 10)).to_le_bytes());
        bytes[0x100] = 0xe9;
        bytes[0x101..0x105].copy_from_slice(&(0x180i32 - 0x105i32).to_le_bytes());
        bytes[0x180..0x182].copy_from_slice(&[0x89, 0x0d]);
        bytes[0x182..0x186].copy_from_slice(&(0x200i32 - 0x186i32).to_le_bytes());
        let base = pe.il2cpp.start;
        let reader = |off: usize, out: &mut [u8]| {
            let s = off.checked_sub(base).ok_or("hole")?;
            out.copy_from_slice(bytes.get(s..s + out.len()).ok_or("hole")?);
            Ok(())
        };
        assert_eq!(
            resolve(
                &pe,
                std::slice::from_ref(&(base..base + bytes.len())),
                reader
            ),
            Ok(base + 0x200)
        );
        assert!(pe.size > 256 * 1024 * 1024);
        let second = pos + 32;
        bytes[second..second + 6].copy_from_slice(PATTERN);
        bytes[second + 6..second + 10]
            .copy_from_slice(&(0x100i32 - (second as i32 + 10)).to_le_bytes());
        let reader = |off: usize, out: &mut [u8]| {
            let s = off.checked_sub(base).ok_or("hole")?;
            out.copy_from_slice(bytes.get(s..s + out.len()).ok_or("hole")?);
            Ok(())
        };
        assert_eq!(
            resolve(
                &pe,
                std::slice::from_ref(&(base..base + bytes.len())),
                reader
            ),
            Ok(base + 0x200)
        );
        // A second, genuinely different writable-target candidate is still rejected.
        bytes[second + 6..second + 10]
            .copy_from_slice(&(0x300i32 - (second as i32 + 10)).to_le_bytes());
        bytes[0x300] = 0xe9;
        bytes[0x301..0x305].copy_from_slice(&(0x380i32 - 0x305i32).to_le_bytes());
        bytes[0x380..0x382].copy_from_slice(&[0x89, 0x0d]);
        bytes[0x382..0x386].copy_from_slice(&(0x400i32 - 0x386i32).to_le_bytes());
        let reader = |off: usize, out: &mut [u8]| {
            let s = off.checked_sub(base).ok_or("hole")?;
            out.copy_from_slice(bytes.get(s..s + out.len()).ok_or("hole")?);
            Ok(())
        };
        assert_eq!(
            resolve(
                &pe,
                std::slice::from_ref(&(base..base + bytes.len())),
                reader
            ),
            Err("ambiguous FPS signature matches")
        );
    }
    #[test]
    fn rejects_bad_pe_and_unreadable_thunk() {
        assert!(parse(&header(0x5000)).is_err());
        let pe = parse(&header(0x1aff7000)).unwrap();
        assert!(
            resolve(&pe, std::slice::from_ref(&pe.il2cpp), |_, _| Err(
                "unreadable".into()
            ))
            .is_err()
        );
        let mut h = header(0x1aff7000);
        h[0x188] = 0;
        assert!(parse(&h).is_err());
    }
}
