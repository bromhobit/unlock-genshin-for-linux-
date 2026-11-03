use std::os::unix::process::CommandExt;
use std::{
    env,
    ffi::{CString, c_void},
    path::PathBuf,
    process::{Command, exit},
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
            let lib = library()?;
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
