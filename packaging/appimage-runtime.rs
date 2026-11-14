use std::{
    env,
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, exit},
    time::{SystemTime, UNIX_EPOCH},
};

const PAYLOAD: &[u8] = include_bytes!(env!("APPDIR_ARCHIVE"));

fn create_temp_dir() -> Result<PathBuf, String> {
    let base = env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let pid = std::process::id();
    for attempt in 0..100u32 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let candidate = base.join(format!(".mount_GenshinFPS-{pid}-{stamp}-{attempt}"));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                fs::set_permissions(&candidate, fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", candidate.display())),
        }
    }
    Err("unable to create temporary AppImage mount directory".into())
}

fn extract_payload(dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let archive_path = dest.join(".appdir.tar.gz");
    {
        let mut f = File::create(&archive_path).map_err(|e| e.to_string())?;
        f.write_all(PAYLOAD).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(dest)
        .status()
        .map_err(|e| format!("failed to execute tar: {e}"))?;
    let _ = fs::remove_file(&archive_path);
    if !status.success() {
        return Err("failed to extract AppImage payload".into());
    }
    Ok(())
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.first().map(String::as_str) == Some("--appimage-version") {
        println!("AppImage runtime (self-extracting x86_64) 0.2.4");
        return;
    }
    if raw_args.first().map(String::as_str) == Some("--appimage-help") {
        println!(
            "AppImage options:\n  \
             --appimage-extract              Extract content to squashfs-root\n  \
             --appimage-extract-and-run      Extract and run AppImage without FUSE\n  \
             --appimage-version              Print AppImage runtime version\n  \
             --appimage-help                 Print this help"
        );
        return;
    }
    if raw_args.first().map(String::as_str) == Some("--appimage-extract") {
        let dest = PathBuf::from("squashfs-root");
        let _ = fs::remove_dir_all(&dest);
        if let Err(e) = extract_payload(&dest) {
            eprintln!("AppImage error: {e}");
            exit(1);
        }
        println!("squashfs-root");
        return;
    }

    let forward_args = if raw_args.first().map(String::as_str) == Some("--appimage-extract-and-run")
    {
        &raw_args[1..]
    } else {
        &raw_args[..]
    };

    let temp_dir = match create_temp_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("AppImage error: {e}");
            exit(1);
        }
    };

    if let Err(e) = extract_payload(&temp_dir) {
        let _ = fs::remove_dir_all(&temp_dir);
        eprintln!("AppImage error: {e}");
        exit(1);
    }

    let appimage_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("Genshin-FPS.AppImage"));
    let owd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let apprun = temp_dir.join("AppRun");

    let status = Command::new(&apprun)
        .args(forward_args)
        .env("APPIMAGE", &appimage_path)
        .env("APPDIR", &temp_dir)
        .env("OWD", &owd)
        .status();

    let _ = fs::remove_dir_all(&temp_dir);

    match status {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("AppImage failed to run AppRun: {e}");
            exit(1);
        }
    }
}
