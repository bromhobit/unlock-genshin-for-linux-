# Genshin FPS Linux

Нативный Linux ELF порт части [genshin-fps-unlock](https://github.com/34736384/genshin-fps-unlock), которая задаёт FPS. Цель — Genshin Impact под 64-битным Wine/Proton на Arch, CachyOS и SteamOS. Это не Windows `.exe`, не замена игре и не модификация файлов игры. Программа распространяется под MIT; исходный алгоритм взят из upstream `netcore` на коммите `2b85d61dd06f6e11ad86fdd6bd90339f9abc58eb` (`UnlockerStub/dllmain.cpp::SetupData`).

**Статус:** сборка и тесты Linux проходят без игры. Работа в реальном Genshin Impact и прохождение цепочки Steam/Proton **не проверены**. Если Steam runtime убирает `LD_PRELOAD`, не видит путь к `.so` или Wine загружает PE без имени файла в `/proc/self/maps`, патч не применится. Сообщение `diagnose` подтверждает загрузку ELF в тестовый процесс, но не доказывает применение FPS в игре.

## Сборка

Нужны Linux x86_64, Rust/Cargo и стандартная glibc. Зависимостей из crates.io нет; сеть не нужна.

```sh
./build.sh
./target/release/genshin-fps diagnose
```

Результат: `target/release/genshin-fps` и `target/release/libgenshin_fps_preload.so`. Для установки только в домашний каталог:

```sh
./install-user.sh
~/.local/bin/genshin-fps diagnose
```

Установка создаёт `~/.local/bin/genshin-fps` и `~/.local/lib/genshin-fps/libgenshin_fps_preload.so`. `sudo`, `sysctl`, системные изменения и Windows-загрузки не нужны. Можно запускать прямо из `target/release` без установки.

## Запуск

Передайте FPS от 10 до 1000 и команду запуска целиком после `--`. Аргументы передаются напрямую как отдельные токены, без shell:

```sh
./target/release/genshin-fps launch 120 -- wine /path/to/GenshinImpact.exe
./target/release/genshin-fps launch 144 -- /path/to/proton run /path/to/GenshinImpact.exe
```

Используйте свою существующую установку игры и Wine/Proton. Выбор Wine prefix, каталога игры и параметров Proton остаётся у вашей команды запуска. Для Steam скопируйте вывод команды в *Properties → Launch Options*:

```sh
./target/release/genshin-fps steam-options 120
```

Пример вывода (абсолютный путь будет вашим):

```sh
GENSHIN_FPS_TARGET=120 LD_PRELOAD='/home/user/path/libgenshin_fps_preload.so'${LD_PRELOAD:+:$LD_PRELOAD} %command%
```

Сохранение прежнего `LD_PRELOAD` нужно для компонентов запуска Steam. Чтобы увидеть состояние preload в stderr/журнале Steam, добавьте `GENSHIN_FPS_LOG=1` перед `%command%` или перед `LD_PRELOAD`. Если `.so` находится в другом месте, задайте `GENSHIN_FPS_LIB=/absolute/path/libgenshin_fps_preload.so` при вызове CLI. Для справки: `genshin-fps help`.

## Проверка и ограничения

`genshin-fps diagnose` запускает собственный тестовый процесс с `LD_PRELOAD`, проверяет экспортированный символ библиотеки и выводит `native ELF library loaded...`. В игре `GENSHIN_FPS_LOG=1` сообщает `loaded`, затем `ready` с адресом цели либо конкретную причину отказа. Отсутствие `loaded` означает, что ELF не попал в процесс. `ready` означает, что сигнатура разрешена и цель проверена; окончательное изменение FPS нужно проверить в игре.

Библиотека ищет **только** отображённый файл с точным именем `GenshinImpact.exe` или `YuanShen.exe`, загруженный как PE32+ AMD64. Она проверяет заголовки и диапазон секции `il2cpp`, ищет `B9 3C 00 00 00 E8`, требует переход `E9` в адресе вызова, следует по ограниченной цепочке `E8`/`E9` и разрешает RIP-смещение у инструкции `89 0D` (`mov [rip+disp32], ecx`). Это соответствует последовательности upstream, с дополнительной проверкой типа инструкции, циклов, границ, единственности совпадения, выравнивания и прав страницы. Запись выполняется из **того же Linux процесса** через `process_vm_writev` каждые 62 мс; библиотека не прикрепляется к другому процессу. Несовпадение с новой версией игры, отсутствие подходящей области памяти или запрет записи приводят к отказу. Никакие устаревшие смещения не используются.

Текущие ограничения: FPS-переменная должна находиться внутри PE-образа и в доступном для записи отображении; некоторые сборки Wine/Proton могут не показывать имя PE-файла в `/proc/self/maps`; preload может не пройти через контейнер Steam. Графического интерфейса нет. Функции upstream вроде энергосбережения, HDR, mobile UI, инъекции DLL и обхода античита не включены.

## English summary

Native x86_64 Linux ELF `LD_PRELOAD` library and CLI for the FPS target in Genshin Impact running under Wine/Proton. Build offline with `./build.sh`; run `genshin-fps diagnose`, then `genshin-fps launch 120 -- wine /path/to/GenshinImpact.exe` or generate Steam options with `genshin-fps steam-options 120`. The resolver follows the pinned upstream PE signature algorithm with additional bounds and permission checks. Linux unit tests pass; the real game and Steam/Proton path have not been tested. No privileged access or cross-process attach is required.
