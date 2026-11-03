#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod resolver;

use std::{
    ffi::c_void,
    fs::{self, OpenOptions},
    io::Write,
    ops::Range,
    os::unix::fs::MetadataExt,
    path::PathBuf,
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[repr(C)]
struct IoVec {
    base: *mut c_void,
    len: usize,
}
unsafe extern "C" {
    fn getpid() -> i32;
    fn process_vm_readv(
        pid: i32,
        local: *const IoVec,
        local_count: usize,
        remote: *const IoVec,
        remote_count: usize,
        flags: usize,
    ) -> isize;
    fn process_vm_writev(
        pid: i32,
        local: *const IoVec,
        local_count: usize,
        remote: *const IoVec,
        remote_count: usize,
        flags: usize,
    ) -> isize;
}

#[derive(Clone)]
struct Mapping {
    start: usize,
    end: usize,
    readable: bool,
    writable: bool,
    offset: usize,
    path: String,
}

fn mappings() -> Result<Vec<Mapping>, String> {
    let text = fs::read_to_string("/proc/self/maps").map_err(|e| e.to_string())?;
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let addresses = fields.next()?;
            let permissions = fields.next()?;
            let offset = usize::from_str_radix(fields.next()?, 16).ok()?;
            fields.next()?; // device
            fields.next()?; // inode
            let path = fields.collect::<Vec<_>>().join(" ");
            let (start, end) = addresses.split_once('-')?;
            let start = usize::from_str_radix(start, 16).ok()?;
            let end = usize::from_str_radix(end, 16).ok()?;
            if start >= end {
                return None;
            }
            Some(Mapping {
                start,
                end,
                readable: permissions.starts_with('r'),
                writable: permissions.as_bytes().get(1) == Some(&b'w'),
                offset,
                path,
            })
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

fn game_path(path: &str) -> bool {
    matches!(
        path.strip_suffix(" (deleted)")
            .unwrap_or(path)
            .rsplit(['/', '\\'])
            .next(),
        Some("GenshinImpact.exe" | "YuanShen.exe")
    )
}

fn read_self(address: usize, out: &mut [u8]) -> Result<(), String> {
    let local = IoVec {
        base: out.as_mut_ptr().cast(),
        len: out.len(),
    };
    let remote = IoVec {
        base: address as *mut c_void,
        len: out.len(),
    };
    // Kernel copy reports an error for stale or unreadable mappings; Rust never dereferences them.
    let count = unsafe { process_vm_readv(getpid(), &local, 1, &remote, 1, 0) };
    if count == out.len() as isize {
        Ok(())
    } else {
        Err(format!("read failed at {address:#x}: {count}"))
    }
}

fn write_self(address: usize, fps: i32) -> Result<(), String> {
    let bytes = fps.to_le_bytes();
    let local = IoVec {
        base: bytes.as_ptr() as *mut c_void,
        len: 4,
    };
    let remote = IoVec {
        base: address as *mut c_void,
        len: 4,
    };
    let count = unsafe { process_vm_writev(getpid(), &local, 1, &remote, 1, 0) };
    if count == 4 {
        Ok(())
    } else {
        Err(format!("write failed at {address:#x}: {count}"))
    }
}

fn mapped_read(base: usize, maps: &[Mapping], offset: usize, out: &mut [u8]) -> Result<(), String> {
    let mut address = base.checked_add(offset).ok_or("PE address overflow")?;
    let end = address.checked_add(out.len()).ok_or("PE read overflow")?;
    let mut done = 0;
    while address < end {
        let map = maps
            .iter()
            .find(|m| m.readable && m.start <= address && address < m.end)
            .ok_or_else(|| format!("unreadable PE mapping at {address:#x}"))?;
        let len = (map.end - address).min(end - address);
        read_self(address, &mut out[done..done + len])?;
        done += len;
        address += len;
    }
    Ok(())
}

fn readable_ranges(
    base: usize,
    size: usize,
    maps: &[Mapping],
) -> Result<Vec<Range<usize>>, String> {
    let end = base.checked_add(size).ok_or("PE address overflow")?;
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for m in maps
        .iter()
        .filter(|m| m.readable && m.end > base && m.start < end)
    {
        let range = m.start.max(base) - base..m.end.min(end) - base;
        if let Some(last) = ranges.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
            continue;
        }
        ranges.push(range);
    }
    Ok(ranges)
}

fn writable(address: usize, maps: &[Mapping]) -> bool {
    address.is_multiple_of(4)
        && address.checked_add(4).is_some_and(|end| {
            maps.iter()
                .any(|m| m.writable && m.start <= address && m.end >= end)
        })
}

fn game_process() -> bool {
    fs::read("/proc/self/cmdline").ok().is_some_and(|raw| {
        raw.split(|b| *b == 0).any(|arg| {
            let path = String::from_utf8_lossy(arg);
            game_path(&path)
        })
    })
}

fn resolve_base(base: usize, maps: &[Mapping]) -> Result<usize, String> {
    let mut header = [0u8; 8192];
    mapped_read(base, maps, 0, &mut header)?;
    let pe = resolver::parse(&header).map_err(str::to_owned)?;
    let ranges = readable_ranges(base, pe.size, maps)?;
    let offset = resolver::resolve(&pe, &ranges, |off, out| mapped_read(base, maps, off, out))
        .map_err(str::to_owned)?;
    let address = base.checked_add(offset).ok_or("FPS address overflow")?;
    if !writable(address, maps) {
        return Err("FPS target is not fully writable and aligned".into());
    }
    Ok(address)
}

fn resolve_live(maps: &[Mapping]) -> Result<Option<(usize, usize)>, String> {
    let mut candidates: Vec<_> = maps
        .iter()
        .filter(|m| m.offset == 0 && m.readable && game_path(&m.path))
        .map(|m| m.start)
        .collect();
    // Wine can reserve the PE as private anonymous memory. Only inspect anonymous
    // bases in a process whose own command line names the game executable.
    if candidates.is_empty() && game_process() {
        candidates = maps
            .iter()
            .filter(|m| m.readable && m.offset == 0 && m.path.is_empty() && m.end - m.start >= 8192)
            .filter(|m| {
                let mut magic = [0; 2];
                read_self(m.start, &mut magic).is_ok() && magic == *b"MZ"
            })
            .map(|m| m.start)
            .collect();
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let mut resolved = None;
    let mut last_error = String::new();
    for base in candidates {
        match resolve_base(base, maps) {
            Ok(address) => {
                if resolved.replace((base, address)).is_some() {
                    return Err("multiple valid game PE images".into());
                }
            }
            Err(e) => {
                last_error = e;
            }
        }
    }
    if resolved.is_none() {
        Err(format!("no validated game PE target: {last_error}"))
    } else {
        Ok(resolved)
    }
}

fn log(enabled: bool, message: &str) {
    if enabled {
        eprintln!("[genshin-fps] {message}");
        static FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
        let path = FILE.get_or_init(|| {
            let home = fs::canonicalize(std::env::var_os("HOME")?).ok()?;
            let state = std::env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/state"));
            if !state.is_absolute() || !state.starts_with(&home) {
                return None;
            }
            fs::create_dir_all(&state).ok()?;
            let state = state.canonicalize().ok()?;
            if !state.starts_with(&home) {
                return None;
            }
            let dir = state.join("genshin-fps");
            if !dir.exists() {
                fs::create_dir(&dir).ok()?;
            }
            let meta = fs::symlink_metadata(&dir).ok()?;
            if !meta.is_dir()
                || meta.file_type().is_symlink()
                || meta.uid() != fs::metadata(&home).ok()?.uid()
            {
                return None;
            }
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(dir.join(format!("preload-{}-{stamp}.log", std::process::id())))
        });
        if let Some(path) = path {
            let existing = fs::symlink_metadata(path).ok();
            if existing
                .as_ref()
                .is_some_and(|m| !m.is_file() || m.len() >= 64 * 1024)
            {
                return;
            }
            if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(path) {
                let _ = writeln!(file, "[genshin-fps] {message}");
            }
        }
    }
}

fn worker(fps: i32, logging: bool) {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut resolution_failures = 0;
    let (base, address) = loop {
        let maps = match mappings() {
            Ok(m) => m,
            Err(e) => {
                log(logging, &format!("maps error: {e}"));
                return;
            }
        };
        match resolve_live(&maps) {
            Ok(Some(found)) => break found,
            Ok(None) => (),
            Err(e) => {
                resolution_failures += 1;
                if resolution_failures == 1 || resolution_failures % 20 == 0 {
                    log(logging, &format!("resolution failed: {e}"));
                }
                // The image may still be loading when its first file map appears.
                // Retry until the deadline; never write until validation succeeds.
                if Instant::now() >= deadline {
                    log(logging, "no validated game PE target within 120 seconds");
                    return;
                }
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        }
        if Instant::now() >= deadline {
            log(logging, "game PE not mapped within 120 seconds");
            return;
        }
        thread::sleep(Duration::from_millis(250));
    };
    log(
        logging,
        &format!("ready: FPS target at {address:#x}; requested {fps}"),
    );
    loop {
        let Ok(maps) = mappings() else {
            log(logging, "maps unavailable; stopped");
            break;
        };
        if !maps.iter().any(|m| m.start == base && m.readable) || !writable(address, &maps) {
            log(logging, "game or writable FPS target disappeared; stopped");
            break;
        }
        if let Err(e) = write_self(address, fps) {
            log(logging, &e);
            break;
        }
        thread::sleep(Duration::from_millis(62));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn genshin_fps_preload_probe() -> u32 {
    1
}

extern "C" fn init() {
    let logging = std::env::var_os("GENSHIN_FPS_LOG").is_some_and(|v| v == "1");
    log(logging, "native ELF preload loaded");
    let Some(raw) = std::env::var_os("GENSHIN_FPS_TARGET") else {
        log(logging, "inactive: GENSHIN_FPS_TARGET unset");
        return;
    };
    let Ok(fps) = raw.to_string_lossy().parse::<i32>() else {
        log(logging, "inactive: invalid FPS target");
        return;
    };
    if !(10..=1000).contains(&fps) {
        log(logging, "inactive: FPS must be 10..1000");
        return;
    }
    if let Err(e) = thread::Builder::new()
        .name("genshin-fps".into())
        .spawn(move || {
            thread::sleep(Duration::from_millis(500));
            worker(fps, logging);
        })
    {
        log(logging, &format!("cannot start worker: {e}"));
    }
}

#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = init;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_do_not_bridge_unreadable_holes() {
        let map = |start, end, readable, writable| Mapping {
            start,
            end,
            readable,
            writable,
            offset: 0,
            path: String::new(),
        };
        let maps = [
            map(0x1000, 0x2000, true, false),
            map(0x2000, 0x3000, false, false),
            map(0x3000, 0x4000, true, true),
        ];
        assert_eq!(
            readable_ranges(0x1000, 0x3000, &maps).unwrap(),
            [0..0x1000, 0x2000..0x3000]
        );
        assert!(!writable(0x2ffc, &maps));
        assert!(writable(0x3000, &maps));
    }
}
