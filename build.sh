#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo build --release --offline --locked
ln -sfn genshin-fps target/release/gfps
printf 'Built target/release/genshin-fps and target/release/libgenshin_fps_preload.so\n'
