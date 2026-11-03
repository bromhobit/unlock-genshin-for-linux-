use std::os::unix::process::CommandExt;
use std::{
    env,
    ffi::{CString, OsString, c_void},
    fs::{self, OpenOptions},
    io::Write,
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, PermissionsExt},
    },
    path::{Component, PathBuf},
    process::{Command, exit},
    time::{SystemTime, UNIX_EPOCH},
};

mod sha256;

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
}

fn usage() {
    println!(
        "genshin-fps — Linux native FPS target for Genshin Impact under Wine/Proton\n\
Usage:\n\
  genshin-fps launch FPS -- WINE_OR_PROTON [ARG ...]\n\
  genshin-fps steam-options FPS\n\
  genshin-fps steam-short-options\n\
  gfps GAME_COMMAND [ARG ...]  (120 FPS)\n\
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

fn bundled_library() -> Result<PathBuf, String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("cannot locate executable directory")?;
    let local = dir.join("libgenshin_fps_preload.so");
    let path = if local.is_file() {
        local
    } else {
        dir.join("../lib/genshin-fps/libgenshin_fps_preload.so")
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("bundled library {}: {e}", path.display()))?;
    if !path.is_file() {
        return Err("bundled library is not a file".into());
    }
    Ok(path)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn short_options() -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let alias = exe
        .parent()
        .ok_or("cannot locate executable directory")?
        .join("gfps");
    let target = fs::canonicalize(&alias).map_err(|_| {
        format!(
            "gfps is not installed beside {}; run ./build.sh or install the package",
            exe.display()
        )
    })?;
    if target != exe.canonicalize().map_err(|e| e.to_string())? {
        return Err(format!("{} does not point to this build", alias.display()));
    }
    let path = alias.to_str().ok_or("gfps path is not UTF-8")?;
    if path
        .chars()
        .any(|c| c.is_whitespace() || c == '\'' || c == '"' || c == '%' || c == ':' || c == '\\')
    {
        return Err("gfps path contains characters unsupported in Steam Launch Options".into());
    }
    Ok(format!("{path} %command%"))
}

fn safe_directory(
    parent: &std::path::Path,
    name: &std::ffi::OsStr,
    uid: u32,
) -> Result<PathBuf, String> {
    let dir = parent.join(name);
    match fs::create_dir(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    }
    let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() || meta.uid() != uid {
        return Err(format!("unsafe directory: {}", dir.display()));
    }
    Ok(dir)
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
    if data
        .to_string_lossy()
        .chars()
        .any(|c| c.is_whitespace() || c == ':')
    {
        return Err("Steam preload path cannot contain whitespace or ':'".into());
    }
    let uid = unsafe { getuid() };
    if fs::symlink_metadata(&home)
        .map_err(|e| e.to_string())?
        .uid()
        != uid
    {
        return Err("HOME is not owned by the current user".into());
    }
    let mut data_dir = home.clone();
    for component in data
        .strip_prefix(&home)
        .map_err(|e| e.to_string())?
        .components()
    {
        let Component::Normal(name) = component else {
            return Err("unsafe XDG_DATA_HOME component".into());
        };
        data_dir = safe_directory(&data_dir, name, uid)?;
    }
    let dir = data_dir.join("genshin-fps");
    if dir
        .to_string_lossy()
        .chars()
        .any(|c| c.is_whitespace() || c == ':')
    {
        return Err("Steam preload path cannot contain whitespace or ':'".into());
    }
    let dir = safe_directory(&data_dir, "genshin-fps".as_ref(), uid)?;
    let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != uid
        || meta.permissions().mode() & 0o022 != 0
    {
        return Err("stage directory is not safely owned".into());
    }
    let bytes = fs::read(lib).map_err(|e| format!("library: {e}"))?;
    let digest = sha256::hex(&bytes);
    let target = dir.join(format!("libgenshin_fps_preload-{digest}.so"));
    let verify = || -> Result<bool, String> {
        match fs::symlink_metadata(&target) {
            Ok(meta) => {
                if !meta.is_file()
                    || meta.file_type().is_symlink()
                    || meta.uid() != uid
                    || meta.permissions().mode() & 0o022 != 0
                {
                    return Err("unsafe cached library file".into());
                }
                if fs::read(&target).map_err(|e| e.to_string())? != bytes {
                    return Err("cached library content mismatch".into());
                }
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    };
    if verify()? {
        return Ok(target);
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = dir.join(format!(".preload-{}-{stamp}.tmp", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        output.write_all(&bytes).map_err(|e| e.to_string())?;
        output
            .set_permissions(fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        match fs::hard_link(&temp, &target) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result?;
    if !verify()? {
        return Err("cached library disappeared during staging".into());
    }
    Ok(target)
}

unsafe extern "C" {
    fn getuid() -> u32;
}

fn run_alias(args: &[OsString]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: gfps GAME_COMMAND [ARG ...]".into());
    }
    let lib = stage_library(&bundled_library()?)?;
    let mut preload = lib.into_os_string();
    if let Some(old) = env::var_os("LD_PRELOAD")
        && !old.is_empty()
    {
        preload.push(":");
        preload.push(old);
    }
    let err = Command::new(&args[0])
        .args(&args[1..])
        .env("GENSHIN_FPS_TARGET", "120")
        .env("LD_PRELOAD", preload)
        .exec();
    Err(format!(
        "cannot launch {}: {err}",
        args[0].to_string_lossy()
    ))
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
        Some("steam-short-options") => {
            if args.len() != 1 {
                return Err("usage: genshin-fps steam-short-options".into());
            }
            println!("{}", short_options()?);
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
    if env::args_os().next().is_some_and(|arg| {
        std::path::Path::new(&arg)
            .file_name()
            .is_some_and(|name| name.as_bytes() == b"gfps")
    }) {
        if let Err(e) = run_alias(&env::args_os().skip(1).collect::<Vec<_>>()) {
            eprintln!("gfps: {e}");
            exit(2);
        }
        return;
    }
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
