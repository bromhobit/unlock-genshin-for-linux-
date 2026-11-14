#!/bin/sh
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
release="$repo/release"
deb="$release/genshin-fps-linux_0.2.4_amd64.deb"
rpm_pkg="$release/genshin-fps-linux-0.2.4-1.x86_64.rpm"
appimage="$release/Genshin-FPS-0.2.4-x86_64.AppImage"

fail() { printf 'Linux packages test failed: %s\n' "$*" >&2; exit 1; }

[ -f "$deb" ] || fail "missing $deb (run scripts/build-linux-packages.sh)"
[ -x "$appimage" ] || fail "missing executable $appimage (run scripts/build-linux-packages.sh)"

if command -v shellcheck >/dev/null 2>&1; then
  shellcheck "$repo/scripts/build-linux-packages.sh" "$0"
fi

temp=$(mktemp -d "${TMPDIR:-/tmp}/genshin-fps-linux-test.XXXXXXXX")
trap 'rm -rf -- "$temp"' EXIT HUP INT TERM

# 1. Verify AppImage header and runtime options
python3 - "$appimage" <<'PY'
from pathlib import Path
import sys

data = Path(sys.argv[1]).read_bytes()
assert data[:4] == b"\x7fELF", "AppImage is not an ELF binary"
assert data[8:11] == b"AI\x02", "AppImage missing AI\\x02 Type 2 magic bytes"
PY

"$appimage" --appimage-version | grep -Fq '0.2.4' || fail 'AppImage version output mismatch'
"$appimage" --appimage-help | grep -Fq -- '--appimage-extract' || fail 'AppImage help output mismatch'
"$appimage" --smoke-test > "$temp/appimage-smoke.out" 2> "$temp/appimage-smoke.err" || fail 'AppImage smoke test failed'
grep -Fq 'native ELF library loaded' "$temp/appimage-smoke.out" || fail 'AppImage smoke test did not load preload'
"$appimage" --appimage-extract-and-run diagnose > "$temp/appimage-diag.out" 2> "$temp/appimage-diag.err" || fail 'AppImage extract-and-run diagnose failed'
grep -Fq 'native ELF library loaded' "$temp/appimage-diag.out" || fail 'AppImage extract-and-run did not load preload'

(
  cd "$temp"
  "$appimage" --appimage-extract > "$temp/extract.out"
  [ -x "$temp/squashfs-root/AppRun" ] || fail 'extracted AppDir missing AppRun'
  [ -x "$temp/squashfs-root/usr/bin/genshin-fps" ] || fail 'extracted AppDir missing genshin-fps'
  [ -L "$temp/squashfs-root/usr/bin/gfps" ] || fail 'extracted AppDir missing gfps symlink'
)

# Verify AppImage user staging when launching GUI without /usr/bin/genshin-fps
app_home="$temp/apphome"
app_data="$app_home/data"
mkdir -p "$app_data" "$temp/mock"
cat > "$temp/mock/python3" <<'EOF'
#!/bin/sh
if [ "${1:-}" = "-" ]; then
  exec /usr/bin/python3 "$@"
fi
printf '%s\n' "$1" > "$APPIMAGE_GUI_CAPTURE"
EOF
chmod +x "$temp/mock/python3"
PATH="$temp/mock:$PATH" HOME="$app_home" XDG_DATA_HOME="$app_data" \
  APPIMAGE_GUI_CAPTURE="$temp/appimage-gui" "$appimage" || fail 'AppImage GUI staging failed'
[ "$(cat "$temp/appimage-gui")" = "$app_home/.local/lib/genshin-fps/genshin-fps-gui.py" ] || fail 'AppImage did not launch staged user GUI'
[ -x "$app_home/.local/bin/genshin-fps" ] || fail 'AppImage did not stage user CLI'
[ -L "$app_home/.local/bin/gfps" ] || fail 'AppImage did not stage user gfps symlink'
[ ! -e "$app_data/genshin-fps" ] || fail 'AppImage staged preload cache during GUI startup'
HOME="$app_home" XDG_DATA_HOME="$app_data" \
  "$app_home/.local/bin/genshin-fps" steam-short-options 144 > "$temp/appimage-short.out" || fail 'AppImage staged CLI short options failed'
[ "$(cat "$temp/appimage-short.out")" = "$app_home/.local/bin/gfps --fps 144 %command%" ] || fail 'AppImage staged CLI returned wrong short options'

# 2. Verify DEB package
deb_root="$temp/deb-root"
mkdir -p "$deb_root"
dpkg-deb -x "$deb" "$deb_root"
for path in \
  usr/bin/genshin-fps \
  usr/bin/gfps \
  usr/bin/genshin-fps-gui \
  usr/lib/genshin-fps/libgenshin_fps_preload.so \
  usr/lib/genshin-fps/genshin-fps-gui.py \
  usr/share/applications/genshin-fps.desktop \
  usr/share/icons/hicolor/scalable/apps/genshin-fps.svg \
  usr/share/licenses/genshin-fps-linux/LICENSE \
  usr/share/doc/genshin-fps-linux/copyright; do
  [ -f "$deb_root/$path" ] || fail "missing DEB file $path"
done
if [ ! -L "$deb_root/usr/bin/gfps" ] || [ "$(readlink "$deb_root/usr/bin/gfps")" != "genshin-fps" ]; then
  fail 'DEB gfps symlink mismatch'
fi
"$deb_root/usr/bin/genshin-fps" diagnose > "$temp/deb-probe.out" 2> "$temp/deb-probe.err" || fail 'DEB diagnose failed'
grep -Fq 'native ELF library loaded' "$temp/deb-probe.out" || fail 'DEB preload probe failed'
[ "$("$deb_root/usr/bin/genshin-fps" steam-short-options 144)" = "$deb_root/usr/bin/gfps --fps 144 %command%" ] || fail 'DEB short options mismatch'

# 3. Verify RPM package if rpm is installed
if [ -f "$rpm_pkg" ] && command -v rpm >/dev/null 2>&1; then
  rpm -qlp "$rpm_pkg" > "$temp/rpm.files"
  for path in \
    /usr/bin/genshin-fps \
    /usr/bin/gfps \
    /usr/bin/genshin-fps-gui \
    /usr/lib/genshin-fps/libgenshin_fps_preload.so \
    /usr/lib/genshin-fps/genshin-fps-gui.py \
    /usr/share/applications/genshin-fps.desktop \
    /usr/share/icons/hicolor/scalable/apps/genshin-fps.svg \
    /usr/share/licenses/genshin-fps-linux/LICENSE; do
    grep -Fxq "$path" "$temp/rpm.files" || fail "missing RPM file $path"
  done
fi

printf 'Linux packages test passed.\n'
