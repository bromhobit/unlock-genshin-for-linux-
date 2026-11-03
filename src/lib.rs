#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod resolver;

use std::{
    ffi::c_void,
    fs, thread,
    time::{Duration, Instant},
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
            .rsplit('/')
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

fn read_image(base: usize, maps: &[Mapping]) -> Result<Vec<u8>, String> {
    let mut header = [0u8; 0x1000];
    read_self(base, &mut header)?;
    let nt = u32::from_le_bytes(header[0x3c..0x40].try_into().unwrap()) as usize;
    if &header[..2] != b"MZ"
        || nt.checked_add(0x60).is_none_or(|v| v > header.len())
        || header.get(nt..nt + 4) != Some(b"PE\0\0")
    {
        return Err("mapped executable lacks valid PE headers".into());
    }
    let size = u32::from_le_bytes(header[nt + 24 + 56..nt + 24 + 60].try_into().unwrap()) as usize;
    if !(0x1000..=256 * 1024 * 1024).contains(&size) {
        return Err("invalid or excessive PE image size".into());
    }
    let end = base.checked_add(size).ok_or("PE image address overflow")?;
    let mut image = vec![0u8; size];
    for map in maps
        .iter()
        .filter(|m| m.readable && m.end > base && m.start < end)
    {
        let mut address = map.start.max(base);
        let stop = map.end.min(end);
        while address < stop {
            let count = (stop - address).min(64 * 1024);
            read_self(address, &mut image[address - base..address - base + count])?;
            address += count;
        }
    }
    Ok(image)
}

fn writable(address: usize, maps: &[Mapping]) -> bool {
    address.is_multiple_of(4)
        && address.checked_add(4).is_some_and(|end| {
            maps.iter()
                .any(|m| m.writable && m.start <= address && m.end >= end)
        })
}

fn resolve_live(maps: &[Mapping]) -> Result<Option<usize>, String> {
    let candidates: Vec<_> = maps
        .iter()
        .filter(|m| m.offset == 0 && m.readable && game_path(&m.path))
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() != 1 {
        return Err("multiple game PE base mappings".into());
    }
    let base = candidates[0].start;
    let image = read_image(base, maps)?;
    let offset = resolver::resolve(&image).map_err(str::to_owned)?;
    let address = base.checked_add(offset).ok_or("FPS address overflow")?;
    if !writable(address, maps) {
        return Err("FPS target is not fully writable and aligned".into());
    }
    Ok(Some(address))
}

fn log(enabled: bool, message: &str) {
    if enabled {
        eprintln!("[genshin-fps] {message}");
    }
}

fn worker(fps: i32, logging: bool) {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut resolution_failures = 0;
    let address = loop {
        let maps = match mappings() {
            Ok(m) => m,
            Err(e) => {
                log(logging, &format!("maps error: {e}"));
                return;
            }
        };
        match resolve_live(&maps) {
            Ok(Some(address)) => break address,
            Ok(None) => (),
            Err(e) => {
                resolution_failures += 1;
                if resolution_failures == 1 || resolution_failures == 20 {
                    log(logging, &format!("resolution failed: {e}"));
                }
                // The image may still be loading when its first file map appears.
                // Retry briefly; never write until the complete validation succeeds.
                if resolution_failures >= 20 {
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
        if !maps.iter().any(|m| m.offset == 0 && game_path(&m.path)) || !writable(address, &maps) {
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
