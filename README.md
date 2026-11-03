# Genshin FPS для Linux

Нативный preload для Genshin Impact в Steam/Proton и короткая команда `gfps` с настраиваемой целью **10–1000 FPS**. GTK4 приложение позволяет выбрать FPS (по умолчанию 120), показывает готовую строку параметров запуска Steam и копирует её в буфер обмена. Оно не запускает игру, не меняет её файлы и не размещает библиотеку при открытии окна.

FPS unlock версии 0.2.2 был подтверждён пользователем в реальной игре через Steam/Proton на Zenith. Для системного `gfps` версии 0.2.4 проверена сборка и передача аргументов в безвредных тестовых командах; доступность `/usr/bin/gfps` внутри конкретного Steam Runtime требует проверки после установки.

## Arch Linux / CachyOS

Скачайте из [выпуска v0.2.4-preview.1](https://github.com/bromhobit/unlock-genshin-for-linux-/releases/tag/v0.2.4-preview.1) пакет `genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst` и `install-genshin-fps.sh` в одну папку. Запустите `./install-genshin-fps.sh`. Установщик вызывает `pacman -U` с правами администратора только для установки пакета. Пакет ставит `/usr/bin/gfps` как ссылку на тот же Rust CLI `/usr/bin/genshin-fps`, библиотеку, GUI и ярлык приложения. Для удаления: `sudo pacman -Rns genshin-fps-linux`.

Откройте **Genshin FPS — Steam** в меню, выберите лимит FPS и нажмите **Скопировать для Steam**. Строка для системного пакета при 144 FPS:

```text
/usr/bin/gfps --fps 144 %command%
```

Вставьте её в Steam → Свойства Genshin Impact → Параметры запуска. При реальном запуске `gfps` создаёт в `${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps` копию библиотеки с именем по SHA-256 содержимого, проверяет уже созданную копию и повторно использует её. Старые версии не заменяются во время обновления. Команда передаёт Steam все аргументы без shell eval, ставит `GENSHIN_FPS_TARGET` в выбранное значение и добавляет библиотеку к существующему `LD_PRELOAD`. Старый вариант `/usr/bin/gfps %command%` по умолчанию оставляет 120 FPS. По умолчанию журнал не создаётся.

## SteamOS / пользовательская установка

Для установки из исходников без прав администратора:

```sh
./install-user.sh
~/.local/bin/genshin-fps steam-short-options 144
```

Короткая строка будет `/home/ИМЯ/.local/bin/gfps --fps 144 %command%` с абсолютным путём текущего пользователя. Если `~/.local/bin/gfps` уже существует и не принадлежит этой установке, скрипт **откажется** его заменять. Это относится и к ручному shell wrapper. `./uninstall-user.sh` удаляет ссылку только при совпадении метки установки и её ожидаемой цели. Пользовательский установщик не ставит GTK4/PyGObject; они должны быть доступны в системе. При установленном системном пакете пользовательская установка откажется работать.

## CLI и сборка

```sh
./build.sh
./target/release/genshin-fps steam-short-options 144
./target/release/gfps --fps 144 /usr/bin/env
```

`build.sh` создаёт `target/release/gfps` как ссылку на локальную сборку. `genshin-fps steam-short-options FPS` печатает точную строку и проверяет ссылку, не размещая `.so` и не запуская Steam. Без FPS выводит старую строку с целью 120. До сборки команда не доступна. Для совместимости остаются `genshin-fps diagnose`, `genshin-fps launch FPS -- ...` и `genshin-fps steam-options FPS`; последняя команда выдаёт длинную строку с preload. `gfps` без команды выводит ошибку использования.

Сборка и проверка пакета без установки на хост:

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
python3 -m unittest discover -s tests
bash scripts/build-arch-package.sh
bash tests/test-packaging.sh
```

Исходный архив собирается по явному списку файлов. В релизе нет отдельного `SHA256SUMS`; `PKGBUILD` содержит SHA-256 исходного архива.

## Риски и ограничения

Библиотека меняет значение FPS в памяти процесса игры. Античит игры может обнаружить модификацию памяти; скрытность и отсутствие санкций не гарантируются. Проект не меняет файлы игры, не обходит античит и не скрывает процесс или модуль. Новая версия игры либо ограничения Steam Runtime могут нарушить работу. Для диагностики preload можно явно задать `GENSHIN_FPS_LOG=1`; журнал окажется в `${XDG_STATE_HOME:-$HOME/.local/state}/genshin-fps`.

Алгоритм FPS основан на [genshin-fps-unlock](https://github.com/34736384/genshin-fps-unlock), `UnlockerStub/dllmain.cpp::SetupData` на коммите `2b85d61dd06f6e11ad86fdd6bd90339f9abc58eb`. Лицензия — [MIT](LICENSE). Подробности — [заметки к выпуску](docs/RELEASE-NOTES.md).
