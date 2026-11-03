use std::os::unix::process::CommandExt;
use std::{
    env,
    ffi::{CString, c_void},
    fs::{self, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Component, PathBuf},
    process::{Command, exit},
    time::{SystemTime, UNIX_EPOCH},
};

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
}

fn usage() {
    println!(
        "genshin-fps — Linux native FPS target for Genshin Impact under Wine/Proton\n\
Usage:\n\
  genshin-fps launch FPS -- WINE_OR_PROTON [ARG ...]\n\
  genshin-fps steam-options FPS\n\
  genshin-fps diagnose\n\
  genshin-fps help\n\n\
FPS: integer 10..1000. The launch command forwards each argument without a shell.\n\
Examples:\n\
  genshin-fps launch 120 -- wine /path/to/GenshinImpact.exe\n\
  genshin-fps launch 144 -- /path/to/proton run /path/to/GenshinImpact.exe\n\
  genshin-fps steam-options 120\n\n\
Set GENSHIN_FPS_LIB to override the .so path. GENSHIN_FPS_LOG=1 enables preload diagnostics."
    );
}

fn fps(value: Option<&String>) -> Result<i32, String> {
    let value = value.ok_or("missing FPS")?;
    let parsed = value
        .parse::<i32>()
        .map_err(|_| "FPS must be an integer 10..1000")?;
    if !(10..=1000).contains(&parsed) {
        return Err("FPS must be 10..1000".into());
    }
    Ok(parsed)
}

fn library() -> Result<PathBuf, String> {
    let path = if let Some(value) = env::var_os("GENSHIN_FPS_LIB") {
        PathBuf::from(value)
    } else {
        let exe = env::current_exe().map_err(|e| e.to_string())?;
        let dir = exe.parent().ok_or("cannot locate executable directory")?;
        let local = dir.join("libgenshin_fps_preload.so");
        if local.exists() {
            local
        } else {
            dir.join("../lib/genshin-fps/libgenshin_fps_preload.so")
        }
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("library {}: {e}", path.display()))?;
    if !path.is_file() {
        return Err(format!("not a file: {}", path.display()));
    }
    Ok(path)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn stage_library(lib: &std::path::Path) -> Result<PathBuf, String> {
    let home = fs::canonicalize(env::var_os("HOME").ok_or("HOME unset")?)
        .map_err(|e| format!("HOME: {e}"))?;
    let data = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    if !data.is_absolute() || !data.starts_with(&home) {
        return Err("XDG_DATA_HOME must be an absolute path under HOME".into());
    }
    let uid = fs::metadata(&home).map_err(|e| e.to_string())?.uid();
    let mut data_dir = home.clone();
    for component in data
        .strip_prefix(&home)
        .map_err(|e| e.to_string())?
        .components()
    {
        let Component::Normal(name) = component else {
            return Err("unsafe XDG_DATA_HOME component".into());
        };
        data_dir.push(name);
        if !data_dir.exists() {
            fs::create_dir(&data_dir).map_err(|e| format!("data directory: {e}"))?;
        }
        let meta = fs::symlink_metadata(&data_dir).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink() || meta.uid() != uid {
            return Err("data directory is not safely owned".into());
        }
    }
    let dir = data_dir.join("genshin-fps");
    if dir
        .to_string_lossy()
        .chars()
        .any(|c| c.is_whitespace() || c == ':')
    {
        return Err("Steam preload path cannot contain whitespace or ':'".into());
    }
    if !dir.exists() {
        fs::create_dir(&dir).map_err(|e| format!("stage directory: {e}"))?;
    }
    let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != uid
        || meta.permissions().mode() & 0o022 != 0
    {
        return Err("stage directory is not safely owned".into());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = dir.join(format!(".preload-{}-{stamp}.tmp", std::process::id()));
    let target = dir.join(format!(
        "libgenshin_fps_preload-{}-{stamp}.so",
        std::process::id()
    ));
    let result = (|| -> Result<(), String> {
        let mut input = fs::File::open(lib).map_err(|e| e.to_string())?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
        output
            .set_permissions(fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temp, &target).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;
    Ok(target)
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") | None => {
            usage();
            Ok(())
        }
        Some("steam-options") => {
            if args.len() != 2 {
                return Err("usage: genshin-fps steam-options FPS".into());
            }
            let target = fps(args.get(1))?;
            let lib = stage_library(&library()?)?;
            println!(
                "GENSHIN_FPS_TARGET={target} LD_PRELOAD={}\u{24}{{LD_PRELOAD:+:\u{24}LD_PRELOAD}} %command%",
                shell_quote(&lib.to_string_lossy())
            );
            Ok(())
        }
        Some("launch") => {
            if args.get(2).map(String::as_str) != Some("--") || args.len() < 4 {
                return Err("usage: genshin-fps launch FPS -- WINE_OR_PROTON [ARG ...]".into());
            }
            let target = fps(args.get(1))?;
            let lib = library()?;
            let preload = match env::var_os("LD_PRELOAD") {
                Some(old) if !old.is_empty() => {
                    let mut joined = lib.into_os_string();
                    joined.push(":");
                    joined.push(old);
                    joined
                }
                _ => lib.into_os_string(),
            };
            let err = Command::new(&args[3])
                .args(&args[4..])
                .env("GENSHIN_FPS_TARGET", target.to_string())
                .env("LD_PRELOAD", preload)
                .exec();
            Err(format!("cannot launch {}: {err}", args[3]))
        }
        Some("diagnose") => {
            if args.len() != 1 {
                return Err("usage: genshin-fps diagnose".into());
            }
            let lib = library()?;
            let exe = env::current_exe().map_err(|e| e.to_string())?;
            let output = Command::new(exe)
                .arg("__probe")
                .env_remove("GENSHIN_FPS_TARGET")
                .env("GENSHIN_FPS_LOG", "1")
                .env("LD_PRELOAD", &lib)
                .output()
                .map_err(|e| e.to_string())?;
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            if output.status.success() {
                Ok(())
            } else {
                Err("native preload did not load in probe process".into())
            }
        }
        Some("__probe") => {
            let name = CString::new("genshin_fps_preload_probe").unwrap();
            let symbol = unsafe { dlsym(std::ptr::null_mut(), name.as_ptr()) };
            if symbol.is_null() {
                return Err("preload probe symbol missing".into());
            }
            println!("native ELF library loaded and probe symbol found; game FPS patch untested");
            Ok(())
        }
        _ => Err("unknown command; run genshin-fps help".into()),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("genshin-fps: {e}");
        exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fps_bounds() {
        for value in ["10", "60", "1000"] {
            assert!(fps(Some(&value.to_string())).is_ok());
        }
        for value in ["0", "9", "1001", "abc", "-1"] {
            assert!(fps(Some(&value.to_string())).is_err());
        }
    }
    #[test]
    fn shell_path_quote() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }
}
