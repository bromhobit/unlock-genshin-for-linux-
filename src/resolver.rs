//! Bounded PE and x86-64 resolver, based on upstream UnlockerStub::SetupData.
const PATTERN: &[u8] = &[0xB9, 0x3C, 0, 0, 0, 0xE8];

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn i32_at(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn relative(at: usize, instruction_len: usize, disp: i32) -> Option<usize> {
    at.checked_add(instruction_len)?
        .checked_add_signed(disp as isize)
}

/// Read addresses relative to image base; the caller must only supply bytes safely copied
/// from a mapped PE image. No pointer dereferences occur here.
pub fn resolve(image: &[u8]) -> Result<usize, &'static str> {
    if image.get(..2) != Some(b"MZ") {
        return Err("missing MZ header");
    }
    let nt = u32_at(image, 0x3c).ok_or("short DOS header")? as usize;
    if image.get(nt..nt.checked_add(4).ok_or("NT offset overflow")?) != Some(b"PE\0\0") {
        return Err("missing PE signature");
    }
    let sections = u16_at(image, nt + 6).ok_or("short COFF header")? as usize;
    let optional_len = u16_at(image, nt + 20).ok_or("short COFF header")? as usize;
    if sections == 0 || sections > 96 || optional_len < 0x70 {
        return Err("invalid PE headers");
    }
    let optional = nt.checked_add(24).ok_or("PE offset overflow")?;
    if u16_at(image, optional) != Some(0x20b) {
        return Err("not PE32+ x86-64");
    }
    if u16_at(image, nt + 4) != Some(0x8664) {
        return Err("not AMD64 PE");
    }
    let image_size = u32_at(image, optional + 56).ok_or("short optional header")? as usize;
    if image_size != image.len() {
        return Err("image size mismatch");
    }
    let table = optional
        .checked_add(optional_len)
        .ok_or("section table overflow")?;
    let end = table
        .checked_add(sections.checked_mul(40).ok_or("section count overflow")?)
        .ok_or("section table overflow")?;
    if end > image.len() {
        return Err("section table outside image");
    }
    let mut range = None;
    for i in 0..sections {
        let at = table + i * 40;
        if image[at..at + 8] == *b"il2cpp\0\0" {
            if range.is_some() {
                return Err("duplicate il2cpp section");
            }
            let len = u32_at(image, at + 8).ok_or("short section")? as usize;
            let start = u32_at(image, at + 12).ok_or("short section")? as usize;
            let stop = start.checked_add(len).ok_or("section overflow")?;
            if len == 0 || stop > image.len() {
                return Err("il2cpp section outside image");
            }
            range = Some(start..stop);
        }
    }
    let range = range.ok_or("missing il2cpp section")?;
    let mut matches = Vec::new();
    for offset in range.start..=range.end.saturating_sub(PATTERN.len()) {
        if image.get(offset..offset + PATTERN.len()) != Some(PATTERN) {
            continue;
        }
        let call = offset + 5;
        let Some(dest) = i32_at(image, call + 1).and_then(|d| relative(call, 5, d)) else {
            continue;
        };
        // Upstream accepts only calls into an E9 jump thunk.
        if image.get(dest) != Some(&0xE9) {
            continue;
        }
        let mut rip = call;
        let mut visited = Vec::new();
        for _ in 0..16 {
            if !matches!(image.get(rip), Some(0xE8 | 0xE9)) {
                break;
            }
            if visited.contains(&rip) {
                rip = image.len();
                break;
            }
            visited.push(rip);
            rip = match i32_at(image, rip + 1).and_then(|d| relative(rip, 5, d)) {
                Some(next) if next < image.len() => next,
                _ => image.len(),
            };
        }
        // SetupData reads a displacement at rip+2. Require the expected 32-bit
        // MOV [RIP+disp32], ECX opcode, so an arbitrary instruction cannot qualify.
        if image.get(rip..rip.saturating_add(2)) != Some(&[0x89, 0x0D][..]) {
            continue;
        }
        let Some(target) = i32_at(image, rip + 2).and_then(|d| relative(rip, 6, d)) else {
            continue;
        };
        if target.checked_add(4).is_some_and(|end| end <= image.len()) && target.is_multiple_of(4) {
            matches.push(target);
        }
    }
    match matches.as_slice() {
        [target] => Ok(*target),
        [] => Err("no safe FPS signature match"),
        _ => Err("ambiguous FPS signature matches"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut b = vec![0u8; 0x2000];
        b[..2].copy_from_slice(b"MZ");
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        b[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        b[0x94..0x96].copy_from_slice(&0xf0u16.to_le_bytes());
        b[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        b[0xd0..0xd4].copy_from_slice(&0x2000u32.to_le_bytes());
        b[0x188..0x190].copy_from_slice(b"il2cpp\0\0");
        b[0x190..0x194].copy_from_slice(&0x400u32.to_le_bytes());
        b[0x194..0x198].copy_from_slice(&0x300u32.to_le_bytes());
        b[0x320..0x326].copy_from_slice(PATTERN);
        b[0x326..0x32a].copy_from_slice(&(0x380i32 - 0x32ai32).to_le_bytes());
        b[0x380] = 0xE9;
        b[0x381..0x385].copy_from_slice(&(0x400i32 - 0x385i32).to_le_bytes());
        b[0x400..0x402].copy_from_slice(&[0x89, 0x0d]);
        b[0x402..0x406].copy_from_slice(&(0x800i32 - 0x406i32).to_le_bytes());
        b
    }

    #[test]
    fn resolves_upstream_chain() {
        assert_eq!(resolve(&fixture()), Ok(0x800));
    }
    #[test]
    fn rejects_bad_section() {
        let mut b = fixture();
        b[0x194..0x198].copy_from_slice(&0x1ff0u32.to_le_bytes());
        assert!(resolve(&b).is_err());
    }
    #[test]
    fn rejects_bad_signature_and_opcode() {
        let mut b = fixture();
        b[0x320] = 0;
        assert!(resolve(&b).is_err());
        let mut b = fixture();
        b[0x400] = 0;
        assert!(resolve(&b).is_err());
    }
    #[test]
    fn rejects_cycles_and_ambiguous_matches() {
        let mut b = fixture();
        b[0x381..0x385].copy_from_slice(&(0x380i32 - 0x385i32).to_le_bytes());
        assert!(resolve(&b).is_err());
        let mut b = fixture();
        b.copy_within(0x320..0x32a, 0x340);
        b[0x346..0x34a].copy_from_slice(&(0x380i32 - 0x34ai32).to_le_bytes());
        assert!(resolve(&b).is_err());
    }
    #[test]
    fn garbage_never_panics() {
        for size in 0..1024 {
            assert!(resolve(&vec![0xff; size]).is_err());
        }
        let mut b = fixture();
        b[0x3c..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(resolve(&b).is_err());
    }
}
