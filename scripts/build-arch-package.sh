#!/bin/sh
set -eu

fail() { printf 'Ошибка: %s\n' "$*" >&2; exit 1; }
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
[ "$(id -u)" -ne 0 ] || fail 'makepkg должен запускаться обычным пользователем.'
command -v makepkg >/dev/null 2>&1 || fail 'Установите pacman/makepkg.'
command -v sha256sum >/dev/null 2>&1 || fail 'Не найден sha256sum.'
version=0.2.0
name=genshin-fps-linux
archive="$name-$version.tar.gz"
package="$name-$version-1-x86_64.pkg.tar.zst"
work=$(mktemp -d "${TMPDIR:-/tmp}/genshin-fps-package.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT HUP INT TERM
source_dir="$work/source/$name-$version"
build_dir="$work/build"
mkdir -p "$source_dir/src" "$source_dir/data" "$build_dir" "$repo/release"

# Explicit allowlist: the archive cannot pick up .git, target, logs or local secrets.
for file in Cargo.toml Cargo.lock LICENSE genshin-fps-gui genshin-fps-gui.py; do
  cp -- "$repo/$file" "$source_dir/$file"
done
for file in main.rs lib.rs resolver.rs; do
  cp -- "$repo/src/$file" "$source_dir/src/$file"
done
for file in genshin-fps.desktop.in genshin-fps.svg; do
  cp -- "$repo/data/$file" "$source_dir/data/$file"
done
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
  -czf "$build_dir/$archive" -C "$work/source" "$name-$version"
source_sha=$(sha256sum "$build_dir/$archive" | cut -d ' ' -f 1)
sed "s/REPLACE_WITH_SOURCE_SHA256/$source_sha/" "$repo/packaging/PKGBUILD" \
  > "$build_dir/PKGBUILD"

(
  cd "$build_dir"
  PKGDEST="$build_dir" makepkg --force --noconfirm --clean --cleanbuild
)
[ -f "$build_dir/$package" ] || fail "makepkg не создал $package"
install -m 644 "$build_dir/$package" "$repo/release/$package"
install -m 644 "$build_dir/$archive" "$repo/release/$archive"
install -m 644 "$build_dir/PKGBUILD" "$repo/release/PKGBUILD"
install -m 755 "$repo/packaging/install-package.sh" "$repo/release/install-genshin-fps.sh"
(cd "$repo/release" && sha256sum "$package" > SHA256SUMS)
printf 'Готово: %s\n' "$repo/release/$package"
printf 'SHA256: %s\n' "$(cut -d ' ' -f 1 "$repo/release/SHA256SUMS")"
