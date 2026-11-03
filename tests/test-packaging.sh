#!/bin/sh
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
release="$repo/release"
package_name=genshin-fps-linux-0.2.3-1-x86_64.pkg.tar.zst
archive_name=genshin-fps-linux-0.2.3.tar.gz
package="$release/$package_name"
archive="$release/$archive_name"
fail() { printf 'Packaging test failed: %s\n' "$*" >&2; exit 1; }
for command_name in bsdtar sha256sum desktop-file-validate python3; do
  command -v "$command_name" >/dev/null 2>&1 || fail "missing $command_name"
done
[ -f "$package" ] || fail 'build package first with scripts/build-arch-package.sh'
[ -f "$archive" ] && [ -f "$release/PKGBUILD" ] || fail 'source archive or staged PKGBUILD missing'
[ -x "$release/install-genshin-fps.sh" ] || fail 'standalone installer missing'
[ ! -e "$release/SHA256SUMS" ] || fail 'separate SHA256SUMS must not be produced'
! grep -Fq SHA256SUMS "$release/install-genshin-fps.sh" || fail 'installer must not reference removed file'
source_sha=$(sha256sum "$archive" | cut -d ' ' -f 1)
grep -Fq "sha256sums=('$source_sha')" "$release/PKGBUILD" || fail 'PKGBUILD source SHA256 mismatch'
grep -Fq "source=(\"\${pkgname}-\${pkgver}.tar.gz\")" "$release/PKGBUILD" || fail 'PKGBUILD source name mismatch'
grep -Fq "depends=('glibc' 'python' 'python-gobject' 'gtk4')" "$release/PKGBUILD" || fail 'package dependency list mismatch'
if tar -tzf "$archive" | grep -Eq '(^|/)(\.git|target|.*\.log|\.aws|\.codex|\.agents)(/|$)'; then
  fail 'source archive contains local data or build output'
fi
tar -tzf "$archive" | grep -Fq 'genshin-fps-linux-0.2.3/src/sha256.rs' || fail 'source archive omits hash implementation'
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck "$repo/install-user.sh" "$repo/uninstall-user.sh" \
    "$repo/packaging/install-package.sh" "$repo/scripts/build-arch-package.sh" "$0"
fi

temp=$(mktemp -d "${TMPDIR:-/tmp}/genshin-fps-test.XXXXXXXX")
trap 'rm -rf -- "$temp"' EXIT HUP INT TERM
root="$temp/root"
mkdir -p "$root" "$temp/mock" "$temp/installer"
bsdtar -xf "$package" -C "$root"
for path in \
  usr/bin/genshin-fps \
  usr/bin/gfps \
  usr/bin/genshin-fps-gui \
  usr/lib/genshin-fps/libgenshin_fps_preload.so \
  usr/lib/genshin-fps/genshin-fps-gui.py \
  usr/share/applications/genshin-fps.desktop \
  usr/share/icons/hicolor/scalable/apps/genshin-fps.svg \
  usr/share/licenses/genshin-fps-linux/LICENSE; do
  [ -f "$root/$path" ] || fail "missing package file $path"
done
[ -x "$root/usr/bin/genshin-fps" ] && [ -x "$root/usr/bin/genshin-fps-gui" ] || fail 'executables lack executable bit'
desktop-file-validate "$root/usr/share/applications/genshin-fps.desktop" || fail 'invalid desktop file'
grep -Fq 'Exec=/usr/bin/genshin-fps-gui' "$root/usr/share/applications/genshin-fps.desktop" || fail 'desktop Exec mismatch'
cat > "$temp/mock/python3" <<'EOF'
#!/bin/sh
printf '%s\n' "$1" > "$GUI_SCRIPT_CAPTURE"
EOF
chmod +x "$temp/mock/python3"
PATH="$temp/mock:$PATH" GUI_SCRIPT_CAPTURE="$temp/gui-script" \
  "$root/usr/bin/genshin-fps-gui" || fail 'desktop launcher failed'
[ "$(cat "$temp/gui-script")" = "$root/usr/bin/../lib/genshin-fps/genshin-fps-gui.py" ] || fail 'desktop launcher used wrong GUI'
rm -- "$temp/mock/python3"
"$root/usr/bin/genshin-fps" diagnose > "$temp/probe.out" 2> "$temp/probe.err" || fail 'extracted package probe failed'
grep -Fq 'native ELF library loaded' "$temp/probe.out" || fail 'extracted package did not load preload'
mkdir -p "$temp/steam-home/data"
HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  "$root/usr/bin/genshin-fps" steam-short-options > "$temp/options" || fail 'short options failed'
[ "$(cat "$temp/options")" = "$root/usr/bin/gfps %command%" ] || fail 'wrong short options'
[ ! -e "$temp/steam-home/data/genshin-fps" ] || fail 'preview staged the library'
[ -L "$root/usr/bin/gfps" ] && [ "$(readlink "$root/usr/bin/gfps")" = genshin-fps ] || fail 'gfps alias mismatch'
if "$root/usr/bin/gfps" > /dev/null 2>&1; then fail 'empty gfps accepted'; fi
[ ! -e "$temp/steam-home/data/genshin-fps" ] || fail 'empty gfps staged the library'
cat > "$temp/harmless" <<'EOF'
#!/bin/sh
printf '%s\n' "$1" "$2" "$GENSHIN_FPS_TARGET" "$LD_PRELOAD" > "$CAPTURE"
EOF
chmod +x "$temp/harmless"
CAPTURE="$temp/capture" HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  LD_PRELOAD="$root/usr/lib/genshin-fps/libgenshin_fps_preload.so" \
  "$root/usr/bin/gfps" "$temp/harmless" 'one two' 'three' || fail 'gfps forwarding failed'
staged=$(find "$temp/steam-home/data/genshin-fps" -name '*.so' -type f -print -quit)
[ -n "$staged" ] || fail 'gfps did not stage library'
cmp -s "$staged" "$root/usr/lib/genshin-fps/libgenshin_fps_preload.so" || fail 'staged library differs'
[ "$(sed -n '1p' "$temp/capture")" = 'one two' ] || fail 'argument with spaces changed'
[ "$(sed -n '2p' "$temp/capture")" = three ] || fail 'second argument changed'
[ "$(sed -n '3p' "$temp/capture")" = 120 ] || fail 'FPS target mismatch'
[ "$(sed -n '4p' "$temp/capture")" = "$staged:$root/usr/lib/genshin-fps/libgenshin_fps_preload.so" ] || fail 'LD_PRELOAD not merged'
first_count=$(find "$temp/steam-home/data/genshin-fps" -name '*.so' -type f | wc -l)
CAPTURE="$temp/capture" HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  "$root/usr/bin/gfps" "$temp/harmless" a b || fail 'second gfps failed'
[ "$(find "$temp/steam-home/data/genshin-fps" -name '*.so' -type f | wc -l)" -eq "$first_count" ] || fail 'cache grew on repeat'
printf 'corrupt\n' > "$staged"
if CAPTURE="$temp/capture" HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  "$root/usr/bin/gfps" "$temp/harmless" a b > /dev/null 2>&1; then
  fail 'gfps accepted a corrupted cached library'
fi
cmp -s "$staged" "$root/usr/lib/genshin-fps/libgenshin_fps_preload.so" && fail 'gfps replaced corrupted cache'
if HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data folder" \
  "$root/usr/bin/gfps" "$temp/harmless" a b > /dev/null 2>&1; then
  fail 'gfps accepted whitespace in preload path'
fi
mkdir -p "$temp/other-data"
ln -s "$temp/other-data" "$temp/steam-home/escape"
if HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/escape/new" \
  "$root/usr/bin/gfps" "$temp/harmless" a b > /dev/null 2>&1; then
  fail 'gfps accepted a data directory symlink escape'
fi
[ ! -e "$temp/other-data/new" ] || fail 'gfps wrote outside HOME'
python3 - "$root/usr/lib/genshin-fps/genshin-fps-gui.py" "$root/usr/bin/genshin-fps" <<'PY'
import importlib.util
import sys

spec = importlib.util.spec_from_file_location("genshin_fps_gui_test", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
assert module.find_cli() == sys.argv[2], module.find_cli()
assert module.generate_steam_options(sys.argv[2]) == str(__import__("pathlib").Path(sys.argv[2]).parent / "gfps") + " %command%"
PY

cp -- "$release/install-genshin-fps.sh" "$package" "$temp/installer/"
cat > "$temp/mock/id" <<'EOF'
#!/bin/sh
[ "$1" = '-u' ] && { printf '1000\n'; exit 0; }
exit 2
EOF
cat > "$temp/mock/pacman" <<'EOF'
#!/bin/sh
[ "$1" = '-Qq' ] && exit 1
printf '%s\n' "$*" >> "$INSTALLER_CALLS"
EOF
cat > "$temp/mock/pkexec" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >> "$INSTALLER_CALLS"
EOF
chmod +x "$temp/mock/id" "$temp/mock/pacman" "$temp/mock/pkexec"
calls="$temp/calls"
PATH="$temp/mock:$PATH" INSTALLER_CALLS="$calls" "$temp/installer/install-genshin-fps.sh" > "$temp/install.out" || fail 'installer dry run failed'
[ "$(wc -l < "$calls")" -eq 1 ] || fail 'installer made unexpected privileged calls'
grep -Fq "pacman -U -- $temp/installer/$package_name" "$calls" || fail 'installer did not call pacman on requested package'
cp -- "$package" "$temp/installer/genshin-fps-linux-0.2.3-2-x86_64.pkg.tar.zst"
if PATH="$temp/mock:$PATH" INSTALLER_CALLS="$calls" "$temp/installer/install-genshin-fps.sh" > /dev/null 2>&1; then
  fail 'installer accepted multiple packages'
fi
[ "$(wc -l < "$calls")" -eq 1 ] || fail 'installer called privilege escalation with multiple packages'
"$temp/installer/install-genshin-fps.sh" --help > /dev/null || fail 'installer help failed'

user_home="$temp/userhome"
user_data="$user_home/data"
mkdir -p "$user_data"
printf 'keep\n' > "$user_data/unrelated-file"
PATH="$temp/mock:$PATH" HOME="$user_home" XDG_DATA_HOME="$user_data" "$repo/install-user.sh" > "$temp/user-install.out" || fail 'user install failed'
[ -x "$user_home/.local/bin/genshin-fps" ] || fail 'user CLI missing'
[ -L "$user_home/.local/bin/gfps" ] || fail 'user alias missing'
desktop-file-validate "$user_data/applications/genshin-fps.desktop" || fail 'user desktop file invalid'
HOME="$user_home" "$user_home/.local/bin/genshin-fps" diagnose > "$temp/user-probe.out" 2> "$temp/user-probe.err" || fail 'user preload probe failed'
HOME="$user_home" XDG_DATA_HOME="$user_data" "$repo/uninstall-user.sh" > "$temp/user-uninstall.out" || fail 'user uninstall failed'
[ ! -e "$user_home/.local/bin/genshin-fps" ] || fail 'user CLI survived uninstall'
[ ! -L "$user_home/.local/bin/gfps" ] || fail 'user alias survived uninstall'
[ ! -e "$user_data/applications/genshin-fps.desktop" ] || fail 'user desktop survived uninstall'
[ -f "$user_data/unrelated-file" ] || fail 'uninstaller removed unrelated user file'
manual_home="$temp/manualhome"
mkdir -p "$manual_home/.local/bin"
printf '#!/bin/sh\nexit 0\n' > "$manual_home/.local/bin/gfps"
if PATH="$temp/mock:$PATH" HOME="$manual_home" "$repo/install-user.sh" > /dev/null 2>&1; then
  fail 'user installer overwrote manual gfps'
fi
grep -Fq 'exit 0' "$manual_home/.local/bin/gfps" || fail 'manual gfps changed'
if HOME="$manual_home" "$repo/uninstall-user.sh" > /dev/null 2>&1; then
  fail 'uninstaller accepted a home without its manifest'
fi
grep -Fq 'exit 0' "$manual_home/.local/bin/gfps" || fail 'uninstaller removed manual gfps'
printf 'Packaging test passed.\n'
