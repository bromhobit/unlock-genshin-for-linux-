#!/bin/sh
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
release="$repo/release"
package_name=genshin-fps-linux-0.2.2-1-x86_64.pkg.tar.zst
archive_name=genshin-fps-linux-0.2.2.tar.gz
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
"$root/usr/bin/genshin-fps" diagnose > "$temp/probe.out" 2> "$temp/probe.err" || fail 'extracted package probe failed'
grep -Fq 'native ELF library loaded' "$temp/probe.out" || fail 'extracted package did not load preload'
mkdir -p "$temp/steam-home/data"
HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  "$root/usr/bin/genshin-fps" steam-options 120 > "$temp/options" || fail 'Steam generator failed'
staged=$(find "$temp/steam-home/data/genshin-fps" -name '*.so' -type f -print -quit)
[ -n "$staged" ] || fail 'Steam preload was not staged in user data'
cmp -s "$staged" "$root/usr/lib/genshin-fps/libgenshin_fps_preload.so" || fail 'staged library differs'
grep -Fq "GENSHIN_FPS_TARGET=120 LD_PRELOAD='$staged'" "$temp/options" || fail 'Steam option path mismatch'
# shellcheck disable=SC2016
grep -Fq '${LD_PRELOAD:+:$LD_PRELOAD} %command%' "$temp/options" || fail 'existing LD_PRELOAD was not preserved'
printf 'keep\n' > "$temp/steam-home/data/genshin-fps/unrelated.txt"
HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data" \
  "$root/usr/bin/genshin-fps" steam-options 120 > /dev/null || fail 'second Steam generator failed'
[ "$(cat "$temp/steam-home/data/genshin-fps/unrelated.txt")" = keep ] || fail 'Steam staging changed unrelated file'
if HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/data folder" \
  "$root/usr/bin/genshin-fps" steam-options 120 > /dev/null 2>&1; then
  fail 'Steam generator accepted whitespace in LD_PRELOAD path'
fi
mkdir -p "$temp/other-data"
ln -s "$temp/other-data" "$temp/steam-home/escape"
if HOME="$temp/steam-home" XDG_DATA_HOME="$temp/steam-home/escape/new" \
  "$root/usr/bin/genshin-fps" steam-options 120 > /dev/null 2>&1; then
  fail 'Steam generator accepted a data directory symlink escape'
fi
[ ! -e "$temp/other-data/new" ] || fail 'Steam generator wrote outside HOME'
HOME="$temp/steam-home" GENSHIN_FPS_LOG=1 GENSHIN_FPS_TARGET=120 \
  LD_PRELOAD="$staged" /bin/true 2> "$temp/preload.err" || fail 'staged preload failed'
log=$(find "$temp/steam-home/.local/state/genshin-fps" -name 'preload-*.log' -type f -print -quit)
if [ -z "$log" ] || ! grep -Fq 'native ELF preload loaded' "$log"; then
  fail 'state log missing preload diagnostic'
fi
python3 - "$root/usr/lib/genshin-fps/genshin-fps-gui.py" "$root/usr/bin/genshin-fps" <<'PY'
import importlib.util
import sys

spec = importlib.util.spec_from_file_location("genshin_fps_gui_test", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
assert module.find_cli() == sys.argv[2], module.find_cli()
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
cp -- "$package" "$temp/installer/genshin-fps-linux-0.2.2-2-x86_64.pkg.tar.zst"
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
desktop-file-validate "$user_data/applications/genshin-fps.desktop" || fail 'user desktop file invalid'
HOME="$user_home" "$user_home/.local/bin/genshin-fps" diagnose > "$temp/user-probe.out" 2> "$temp/user-probe.err" || fail 'user preload probe failed'
HOME="$user_home" XDG_DATA_HOME="$user_data" "$repo/uninstall-user.sh" > "$temp/user-uninstall.out" || fail 'user uninstall failed'
[ ! -e "$user_home/.local/bin/genshin-fps" ] || fail 'user CLI survived uninstall'
[ ! -e "$user_data/applications/genshin-fps.desktop" ] || fail 'user desktop survived uninstall'
[ -f "$user_data/unrelated-file" ] || fail 'uninstaller removed unrelated user file'
printf 'Packaging test passed.\n'
