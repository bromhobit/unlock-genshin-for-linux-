#!/bin/sh
set -eu
cd "$(dirname "$0")"
./build.sh
prefix="${HOME}/.local/lib/genshin-fps"
bindir="${HOME}/.local/bin"
desktopdir="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
mkdir -p "$prefix" "$bindir" "$desktopdir"
install -m 755 target/release/libgenshin_fps_preload.so "$prefix/libgenshin_fps_preload.so"
install -m 755 target/release/genshin-fps "$bindir/genshin-fps"
install -m 644 genshin-fps-gui.py "$prefix/genshin-fps-gui.py"
install -m 755 genshin-fps-gui "$bindir/genshin-fps-gui"
python3 - "$desktopdir/genshin-fps.desktop" "$bindir/genshin-fps-gui" <<'PY'
from pathlib import Path
import sys

template = Path("data/genshin-fps.desktop.in").read_text(encoding="utf-8")
target, executable = map(Path, sys.argv[1:])
quoted = '"' + str(executable).replace('\\', '\\\\').replace('"', '\\"').replace('$', '\\$').replace('`', '\\`') + '"'
target.write_text(template.replace("@GUI_EXEC@", quoted), encoding="utf-8")
target.chmod(0o644)
PY
printf 'Installed CLI: %s\nGUI: %s\nMenu entry: %s\n' \
    "$bindir/genshin-fps" "$bindir/genshin-fps-gui" "$desktopdir/genshin-fps.desktop"
