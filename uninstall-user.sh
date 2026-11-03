#!/bin/sh
set -eu

prefix="${HOME}/.local/lib/genshin-fps"
bindir="${HOME}/.local/bin"
manifest="$prefix/.install-user.manifest"
if [ ! -f "$manifest" ]; then
  printf 'Пользовательская установка этого скрипта не найдена: %s\n' "$manifest" >&2
  exit 1
fi
desktop=$(sed -n '1p' "$manifest")
icon=$(sed -n '2p' "$manifest")
expected_data="${XDG_DATA_HOME:-${HOME}/.local/share}"
if [ "$desktop" != "$expected_data/applications/genshin-fps.desktop" ] || \
   [ "$icon" != "$expected_data/icons/hicolor/scalable/apps/genshin-fps.svg" ]; then
  printf 'Ошибка: XDG_DATA_HOME не совпадает с установкой; используйте прежнее значение.\n' >&2
  exit 1
fi
if [ "$(sed -n '3p' "$manifest")" = 'gfps=genshin-fps' ]; then
  if [ -e "$bindir/gfps" ] || [ -L "$bindir/gfps" ]; then
    if [ ! -L "$bindir/gfps" ] || [ "$(readlink -- "$bindir/gfps")" != 'genshin-fps' ]; then
      printf 'Ошибка: %s изменён после установки; удаление остановлено.\n' "$bindir/gfps" >&2
      exit 1
    fi
    rm -- "$bindir/gfps"
  fi
fi
rm -f -- "$bindir/genshin-fps" "$bindir/genshin-fps-gui" \
  "$prefix/libgenshin_fps_preload.so" "$prefix/genshin-fps-gui.py" \
  "$desktop" "$icon" "$manifest"
rmdir -- "$prefix" 2>/dev/null || :
printf 'Пользовательская установка удалена.\n'
