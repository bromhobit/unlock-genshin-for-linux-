#!/bin/sh
set -eu
cd "$(dirname "$0")"
./build.sh
prefix="${HOME}/.local/lib/genshin-fps"
bindir="${HOME}/.local/bin"
mkdir -p "$prefix" "$bindir"
install -m 755 target/release/libgenshin_fps_preload.so "$prefix/libgenshin_fps_preload.so"
install -m 755 target/release/genshin-fps "$bindir/genshin-fps"
printf 'Installed to %s and %s\n' "$bindir/genshin-fps" "$prefix/libgenshin_fps_preload.so"
