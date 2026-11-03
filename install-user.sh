#!/bin/sh
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
prefix="${HOME}/.local/lib/genshin-fps"
bindir="${HOME}/.local/bin"
datadir="${XDG_DATA_HOME:-${HOME}/.local/share}"
desktop="$datadir/applications/genshin-fps.desktop"
icon="$datadir/icons/hicolor/scalable/apps/genshin-fps.svg"
manifest="$prefix/.install-user.manifest"

if command -v pacman >/dev/null 2>&1 && pacman -Qq genshin-fps-linux >/dev/null 2>&1; then
  printf 'Ошибка: системный пакет уже установлен. Используйте его GUI и CLI.\n' >&2
  exit 1
fi
if [ -f "$manifest" ]; then
  old_desktop=$(sed -n '1p' "$manifest")
  old_icon=$(sed -n '2p' "$manifest")
  if [ "$old_desktop" != "$desktop" ] || [ "$old_icon" != "$icon" ]; then
    printf 'Ошибка: XDG_DATA_HOME изменился. Сначала запустите ./uninstall-user.sh с прежним XDG_DATA_HOME.\n' >&2
    exit 1
  fi
  if [ -e "$bindir/gfps" ] || [ -L "$bindir/gfps" ]; then
    if [ "$(sed -n '3p' "$manifest")" != 'gfps=genshin-fps' ] || \
       [ ! -L "$bindir/gfps" ] || [ "$(readlink -- "$bindir/gfps")" != 'genshin-fps' ]; then
      printf 'Ошибка: %s уже существует и не принадлежит этой установке. Оставляем файл без изменений.\n' "$bindir/gfps" >&2
      exit 1
    fi
  fi
else
  for path in "$prefix/libgenshin_fps_preload.so" "$prefix/genshin-fps-gui.py" \
    "$bindir/genshin-fps" "$bindir/gfps" "$bindir/genshin-fps-gui" "$desktop" "$icon"; do
    if [ -e "$path" ] || [ -L "$path" ]; then
      printf 'Ошибка: файл уже существует и не отмечен как установленный этим скриптом: %s\n' "$path" >&2
      exit 1
    fi
  done
fi

"$repo/build.sh"
mkdir -p "$prefix" "$bindir" "$(dirname -- "$desktop")" "$(dirname -- "$icon")"
install -m 755 "$repo/target/release/libgenshin_fps_preload.so" "$prefix/libgenshin_fps_preload.so"
install -m 755 "$repo/target/release/genshin-fps" "$bindir/genshin-fps"
ln -sfn genshin-fps "$bindir/gfps"
install -m 644 "$repo/genshin-fps-gui.py" "$prefix/genshin-fps-gui.py"
install -m 755 "$repo/genshin-fps-gui" "$bindir/genshin-fps-gui"
install -m 644 "$repo/data/genshin-fps.svg" "$icon"
python3 - "$repo/data/genshin-fps.desktop.in" "$desktop" "$bindir/genshin-fps-gui" <<'PY'
from pathlib import Path
import sys

template = Path(sys.argv[1]).read_text(encoding="utf-8")
target = Path(sys.argv[2])
executable = sys.argv[3]
quoted = '"' + executable.replace('\\', '\\\\').replace('"', '\\"').replace('$', '\\$').replace('`', '\\`') + '"'
target.write_text(template.replace('@GUI_EXEC@', quoted), encoding="utf-8")
target.chmod(0o644)
PY
printf '%s\n%s\ngfps=genshin-fps\n' "$desktop" "$icon" > "$manifest"
chmod 600 "$manifest"
printf 'Установлено: %s и %s\nУдаление: %s/uninstall-user.sh\n' "$bindir/genshin-fps" "$bindir/genshin-fps-gui" "$repo"
