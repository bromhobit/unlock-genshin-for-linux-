#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Установка готового пакета Genshin FPS Linux на Arch Linux или CachyOS.
Положите этот файл, пакет .pkg.tar.zst и SHA256SUMS в один каталог.
Запуск: ./install-genshin-fps.sh
Справка: ./install-genshin-fps.sh --help
EOF
}

if [ "${1:-}" = '--help' ] || [ "${1:-}" = '-h' ]; then
  usage
  exit 0
fi
if [ "$#" -ne 0 ]; then
  usage >&2
  exit 2
fi

fail() { printf 'Ошибка: %s\n' "$*" >&2; exit 1; }
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
[ -r /etc/os-release ] || fail 'Не удалось определить дистрибутив (/etc/os-release).'
# Only the supported mutable Arch distributions are accepted here.
# shellcheck disable=SC1091
. /etc/os-release
case "${ID:-}" in
  arch|cachyos) ;;
  *) fail 'Этот установщик предназначен для Arch Linux и CachyOS. На SteamOS используйте install-user.sh.' ;;
esac
command -v pacman >/dev/null 2>&1 || fail 'Не найден pacman.'
command -v sha256sum >/dev/null 2>&1 || fail 'Не найден sha256sum.'

set -- "$script_dir"/genshin-fps-linux-*.pkg.tar.zst
[ -f "$1" ] || fail 'Рядом с установщиком нет пакета genshin-fps-linux-*.pkg.tar.zst.'
[ "$#" -eq 1 ] || fail 'Найдено несколько пакетов. Оставьте рядом с установщиком ровно один.'
package=$1
manifest="$script_dir/SHA256SUMS"
[ -f "$manifest" ] || fail 'Не найден SHA256SUMS рядом с пакетом.'
[ "$(wc -l < "$manifest")" -eq 1 ] || fail 'SHA256SUMS должен содержать ровно одну строку для пакета.'
expected_name=$(awk '{print $2}' "$manifest")
[ "$expected_name" = "$(basename -- "$package")" ] || fail 'Имя пакета в SHA256SUMS не совпадает.'
(cd "$script_dir" && sha256sum --check --status --strict SHA256SUMS) || fail 'Контрольная сумма пакета не совпадает.'

printf 'Контрольная сумма проверена: %s\n' "$(basename -- "$package")"
if [ "$(id -u)" -eq 0 ]; then
  exec pacman -U -- "$package"
elif command -v pkexec >/dev/null 2>&1; then
  exec pkexec pacman -U -- "$package"
elif command -v sudo >/dev/null 2>&1; then
  exec sudo pacman -U -- "$package"
else
  fail 'Для установки нужен pkexec или sudo.'
fi
