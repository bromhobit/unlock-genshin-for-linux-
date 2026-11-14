#!/bin/sh
set -eu
umask 022

fail() { printf 'Error: %s\n' "$*" >&2; exit 1; }
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)

for cmd in cargo rustc tar python3 dpkg-deb; do
  command -v "$cmd" >/dev/null 2>&1 || fail "Missing required tool: $cmd"
done

version=0.2.4
name=genshin-fps-linux
deb_name="${name}_${version}_amd64.deb"
rpm_name="${name}-${version}-1.x86_64.rpm"
appimage_name="Genshin-FPS-${version}-x86_64.AppImage"

"$repo/build.sh"

work=$(mktemp -d "${TMPDIR:-/tmp}/genshin-fps-linux-pkgs.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT HUP INT TERM

mkdir -p "$repo/release"
rm -f -- \
  "$repo/release"/genshin-fps-linux_*_amd64.deb \
  "$repo/release"/genshin-fps-linux-*.x86_64.rpm \
  "$repo/release"/Genshin-FPS-*-x86_64.AppImage

desktop_rendered="$work/genshin-fps.desktop"
sed 's|@GUI_EXEC@|/usr/bin/genshin-fps-gui|' "$repo/data/genshin-fps.desktop.in" > "$desktop_rendered"

# 1. Build Debian / Ubuntu package (.deb)
deb_root="$work/deb"
mkdir -p \
  "$deb_root/DEBIAN" \
  "$deb_root/usr/bin" \
  "$deb_root/usr/lib/genshin-fps" \
  "$deb_root/usr/share/applications" \
  "$deb_root/usr/share/icons/hicolor/scalable/apps" \
  "$deb_root/usr/share/licenses/$name" \
  "$deb_root/usr/share/doc/$name"

install -m 755 "$repo/target/release/genshin-fps" "$deb_root/usr/bin/genshin-fps"
ln -s genshin-fps "$deb_root/usr/bin/gfps"
install -m 755 "$repo/target/release/libgenshin_fps_preload.so" \
  "$deb_root/usr/lib/genshin-fps/libgenshin_fps_preload.so"
install -m 755 "$repo/genshin-fps-gui" "$deb_root/usr/bin/genshin-fps-gui"
install -m 644 "$repo/genshin-fps-gui.py" "$deb_root/usr/lib/genshin-fps/genshin-fps-gui.py"
install -m 644 "$desktop_rendered" "$deb_root/usr/share/applications/genshin-fps.desktop"
install -m 644 "$repo/data/genshin-fps.svg" \
  "$deb_root/usr/share/icons/hicolor/scalable/apps/genshin-fps.svg"
install -m 644 "$repo/LICENSE" "$deb_root/usr/share/licenses/$name/LICENSE"
install -m 644 "$repo/LICENSE" "$deb_root/usr/share/doc/$name/copyright"

cat > "$deb_root/DEBIAN/control" <<EOF
Package: $name
Version: $version
Architecture: amd64
Maintainer: Genshin FPS Linux contributors
Depends: libc6 (>= 2.34), python3, python3-gi, gir1.2-gtk-4.0
Section: games
Priority: optional
Homepage: https://github.com/bromhobit/unlock-genshin-for-linux-
Description: Native Linux FPS unlocker (10-1000 FPS) for Genshin Impact under Steam/Proton
 Native ELF preload library, short gfps launcher, and GTK4 GUI for configuring
 custom FPS targets (10-1000 FPS) in Genshin Impact on Linux via Steam/Proton
 or Wine without modifying game files.
EOF

dpkg-deb --build --root-owner-group "$deb_root" "$repo/release/$deb_name" >/dev/null
[ -f "$repo/release/$deb_name" ] || fail "Failed to build $deb_name"

# 2. Build Fedora / RHEL / openSUSE package (.rpm) if rpmbuild is available
if command -v rpmbuild >/dev/null 2>&1; then
  rpm_top="$work/rpmbuild"
  mkdir -p "$rpm_top/BUILD" "$rpm_top/RPMS" "$rpm_top/SOURCES" "$rpm_top/SPECS" "$rpm_top/SRPMS"
  cat > "$rpm_top/SPECS/$name.spec" <<EOF
Name:           $name
Version:        $version
Release:        1
Summary:        Native Linux FPS target helper for Genshin Impact under Wine/Proton
License:        MIT
URL:            https://github.com/bromhobit/unlock-genshin-for-linux-
BuildArch:      x86_64
AutoReqProv:    no
Requires:       glibc >= 2.34, python3, python3-gobject, gtk4

%description
Native ELF preload library, short gfps launcher, and GTK4 GUI for configuring
custom FPS targets (10-1000 FPS) in Genshin Impact on Linux via Steam/Proton
or Wine without modifying game files.

%prep

%build

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/lib/genshin-fps
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/scalable/apps
mkdir -p %{buildroot}/usr/share/licenses/%{name}
install -m 755 "$repo/target/release/genshin-fps" %{buildroot}/usr/bin/genshin-fps
ln -s genshin-fps %{buildroot}/usr/bin/gfps
install -m 755 "$repo/target/release/libgenshin_fps_preload.so" %{buildroot}/usr/lib/genshin-fps/libgenshin_fps_preload.so
install -m 755 "$repo/genshin-fps-gui" %{buildroot}/usr/bin/genshin-fps-gui
install -m 644 "$repo/genshin-fps-gui.py" %{buildroot}/usr/lib/genshin-fps/genshin-fps-gui.py
install -m 644 "$desktop_rendered" %{buildroot}/usr/share/applications/genshin-fps.desktop
install -m 644 "$repo/data/genshin-fps.svg" %{buildroot}/usr/share/icons/hicolor/scalable/apps/genshin-fps.svg
install -m 644 "$repo/LICENSE" %{buildroot}/usr/share/licenses/%{name}/LICENSE

%files
/usr/bin/genshin-fps
/usr/bin/gfps
/usr/bin/genshin-fps-gui
/usr/lib/genshin-fps/libgenshin_fps_preload.so
/usr/lib/genshin-fps/genshin-fps-gui.py
/usr/share/applications/genshin-fps.desktop
/usr/share/icons/hicolor/scalable/apps/genshin-fps.svg
/usr/share/licenses/%{name}/LICENSE
EOF
  mkdir -p "$work/tmp"
  rpmbuild --define "_topdir $rpm_top" --define "_tmppath $work/tmp" --define "debug_package %{nil}" --define "_build_id_links none" -bb "$rpm_top/SPECS/$name.spec" >/dev/null 2>&1
  rpm_built=$(find "$rpm_top/RPMS" -name '*.rpm' -type f -print -quit)
  [ -n "$rpm_built" ] || fail "rpmbuild did not produce an RPM"
  install -m 644 "$rpm_built" "$repo/release/$rpm_name"
fi

# 3. Build universal Linux x86-64 AppImage (works with or without FUSE)
appdir="$work/AppDir"
mkdir -p \
  "$appdir/usr/bin" \
  "$appdir/usr/lib/genshin-fps" \
  "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/scalable/apps" \
  "$appdir/usr/share/licenses/$name"

install -m 755 "$repo/target/release/genshin-fps" "$appdir/usr/bin/genshin-fps"
ln -s genshin-fps "$appdir/usr/bin/gfps"
install -m 755 "$repo/target/release/libgenshin_fps_preload.so" \
  "$appdir/usr/lib/genshin-fps/libgenshin_fps_preload.so"
install -m 755 "$repo/genshin-fps-gui" "$appdir/usr/bin/genshin-fps-gui"
install -m 644 "$repo/genshin-fps-gui.py" "$appdir/usr/lib/genshin-fps/genshin-fps-gui.py"
install -m 644 "$desktop_rendered" "$appdir/usr/share/applications/genshin-fps.desktop"
install -m 644 "$desktop_rendered" "$appdir/genshin-fps.desktop"
install -m 644 "$repo/data/genshin-fps.svg" \
  "$appdir/usr/share/icons/hicolor/scalable/apps/genshin-fps.svg"
install -m 644 "$repo/data/genshin-fps.svg" "$appdir/genshin-fps.svg"
install -m 644 "$repo/LICENSE" "$appdir/usr/share/licenses/$name/LICENSE"

cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
set -eu

appdir=${APPDIR:-$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)}

case "${1:-}" in
  --smoke-test)
    "$appdir/usr/bin/genshin-fps" diagnose
    python3 - "$appdir/usr/lib/genshin-fps/genshin-fps-gui.py" "$appdir/usr/bin/genshin-fps" <<'PY'
import importlib.util
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location("genshin_fps_gui_smoke", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
expected = str(Path(sys.argv[2]).parent / "gfps") + " --fps 144 %command%"
assert module.generate_steam_options(sys.argv[2], 144) == expected
PY
    exit 0
    ;;
  diagnose|steam-options|steam-short-options|launch|help|--help|-h)
    exec "$appdir/usr/bin/genshin-fps" "$@"
    ;;
esac

if [ ! -x /usr/bin/genshin-fps ] && [ -n "${HOME:-}" ] && [ -d "${HOME}" ]; then
  prefix="${HOME}/.local/lib/genshin-fps"
  bindir="${HOME}/.local/bin"
  datadir="${XDG_DATA_HOME:-${HOME}/.local/share}"
  desktop="$datadir/applications/genshin-fps.desktop"
  icon="$datadir/icons/hicolor/scalable/apps/genshin-fps.svg"
  manifest="$prefix/.install-user.manifest"
  can_stage=1
  if [ -e "$bindir/gfps" ] || [ -L "$bindir/gfps" ]; then
    if [ ! -L "$bindir/gfps" ] || [ "$(readlink -- "$bindir/gfps")" != "genshin-fps" ]; then
      can_stage=0
    fi
  fi
  if [ "$can_stage" -eq 1 ]; then
    mkdir -p "$prefix" "$bindir" "$(dirname -- "$desktop")" "$(dirname -- "$icon")"
    install -m 755 "$appdir/usr/lib/genshin-fps/libgenshin_fps_preload.so" "$prefix/libgenshin_fps_preload.so"
    install -m 755 "$appdir/usr/bin/genshin-fps" "$bindir/genshin-fps"
    ln -sfn genshin-fps "$bindir/gfps"
    install -m 644 "$appdir/usr/lib/genshin-fps/genshin-fps-gui.py" "$prefix/genshin-fps-gui.py"
    install -m 755 "$appdir/usr/bin/genshin-fps-gui" "$bindir/genshin-fps-gui"
    install -m 644 "$appdir/usr/share/icons/hicolor/scalable/apps/genshin-fps.svg" "$icon"
    python3 - "$appdir/usr/share/applications/genshin-fps.desktop" "$desktop" "$bindir/genshin-fps-gui" <<'PY'
from pathlib import Path
import sys

template = Path(sys.argv[1]).read_text(encoding="utf-8")
target = Path(sys.argv[2])
executable = sys.argv[3]
quoted = '"' + executable.replace('\\', '\\\\').replace('"', '\\"').replace('$', '\\$').replace('`', '\\`') + '"'
target.write_text(template.replace('/usr/bin/genshin-fps-gui', quoted), encoding="utf-8")
target.chmod(0o644)
PY
    printf '%s\n%s\ngfps=genshin-fps\n' "$desktop" "$icon" > "$manifest"
    chmod 600 "$manifest"
    exec python3 "$prefix/genshin-fps-gui.py" "$@"
  fi
fi

exec python3 "$appdir/usr/lib/genshin-fps/genshin-fps-gui.py" "$@"
EOF
chmod 755 "$appdir/AppRun"

appdir_archive="$work/appdir.tar.gz"
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
  -czf "$appdir_archive" -C "$appdir" .

APPDIR_ARCHIVE="$appdir_archive" rustc --edition=2024 -C opt-level=s -C strip=symbols \
  "$repo/packaging/appimage-runtime.rs" -o "$work/$appimage_name"

# Mark ELF header bytes 8..11 with the AppImage Type 2 magic bytes (0x41 0x49 0x02)
python3 - "$work/$appimage_name" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
data = bytearray(path.read_bytes())
assert data[:4] == b"\x7fELF", "Expected ELF binary"
data[8:11] = b"AI\x02"
path.write_bytes(data)
PY
chmod 755 "$work/$appimage_name"
install -m 755 "$work/$appimage_name" "$repo/release/$appimage_name"

printf 'Built packages in %s/release:\n' "$repo"
ls -lh "$repo/release/$appimage_name" "$repo/release/$deb_name"
if [ -f "$repo/release/$rpm_name" ]; then
  ls -lh "$repo/release/$rpm_name"
fi
