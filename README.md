# Genshin FPS Linux

**Нативный анлокер FPS (10–1000 FPS) для Genshin Impact в Linux (Steam / Proton / Wine) · Native Linux FPS Unlocker (10–1000 FPS) for Genshin Impact (Steam / Proton / Wine)**

[Русский](#-возможности) · [English](#-english)

---

Нативный ELF preload и короткая команда `gfps` для разблокировки частоты кадров **10–1000 FPS** в Genshin Impact под Steam/Proton и Wine в Linux. Графическое приложение на GTK4 позволяет выбрать желаемый лимит FPS (по умолчанию 120), показывает готовую строку параметров запуска Steam и копирует её в буфер обмена. Приложение не изменяет файлы игры на диске и не размещает preload-библиотеку во время предпросмотра.

## Возможности

- Настраиваемый лимит частоты кадров от **10 до 1000 FPS** (по умолчанию **120 FPS**).
- Графический интерфейс **GTK4** (`Genshin FPS — Steam`) для выбора FPS и копирования параметров запуска Steam в один клик.
- Короткая обёртка запуска `gfps` (`/usr/bin/gfps --fps 144 %command%`) без громоздких переменных окружения в строке Steam.
- Безопасное кэширование preload-библиотеки в пользовательском каталоге по контрольной сумме **SHA-256** при запуске игры (обновление пакета не заменяет библиотеку у уже запущенного процесса).
- Сохранение существующего `LD_PRELOAD` (совместимость со Steam Overlay, MangoHud и другими инструментами) и прямая передача аргументов через `exec` без `sh -c`.
- Работа только в оперативной памяти процесса без модификации файлов клиента игры.

## Установка

В [релизе v0.2.4](https://github.com/bromhobit/unlock-genshin-for-linux-/releases/tag/v0.2.4) доступны готовые пакеты для **Linux x86-64**:

- **AppImage** — универсальный пакет для разных дистрибутивов, включая **SteamOS**. Скачайте файл, разрешите его выполнение и запустите:
  ```bash
  chmod +x Genshin-FPS-0.2.4-x86_64.AppImage
  ./Genshin-FPS-0.2.4-x86_64.AppImage
  ```
  Пакет работает со встроенным самоизвлечением (поддержка FUSE не требуется; также поддерживаются флаги `--appimage-extract` и `APPIMAGE_EXTRACT_AND_RUN=1`). На системах без установленного системного пакета первый запуск GUI автоматически размещает клиентскую утилиту в `~/.local/bin/gfps`, чтобы скопированная команда работала в Steam после закрытия окна.

- **DEB** — для **Debian, Ubuntu, Linux Mint, Pop!_OS**: скачайте `genshin-fps-linux_0.2.4_amd64.deb` и установите через менеджер пакетов либо командой:
  ```bash
  sudo apt install ./genshin-fps-linux_0.2.4_amd64.deb
  ```

- **RPM** — для **Fedora, RHEL, openSUSE**: скачайте `genshin-fps-linux-0.2.4-1.x86_64.rpm` и установите командой:
  ```bash
  sudo dnf install ./genshin-fps-linux-0.2.4-1.x86_64.rpm
  ```

- **Arch Linux и CachyOS** — скачайте `genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst` и установите командой:
  ```bash
  sudo pacman -U ./genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst
  ```
  Либо скачайте `genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst` и `install-genshin-fps.sh` в одну папку и запустите `./install-genshin-fps.sh`. Для удаления: `sudo pacman -Rns genshin-fps-linux`.

### Пользовательская установка из исходников (SteamOS / без прав root)

На стандартной SteamOS системный раздел защищён от записи; помимо AppImage можно установить проект в домашний каталог из исходного кода без прав администратора:

```bash
./install-user.sh
~/.local/bin/genshin-fps steam-short-options 144
```

Короткая строка будет `/home/ИМЯ/.local/bin/gfps --fps 144 %command%` с абсолютным путём текущего пользователя. Если `~/.local/bin/gfps` уже существует и не принадлежит этой установке, скрипт откажется его заменять. Для удаления выполните `./uninstall-user.sh`.

## Использование со Steam

1. Откройте приложение **Genshin FPS — Steam** в меню приложений (или запустите `genshin-fps-gui` / AppImage).
2. Выберите желаемый лимит FPS (например, `144`) и нажмите **Скопировать для Steam**.
3. Откройте **Steam → Библиотека → Genshin Impact → Свойства → Параметры запуска** и вставьте скопированную строку:

```text
/usr/bin/gfps --fps 144 %command%
```

При реальном запуске игры `gfps` создаёт в `${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps` копию библиотеки с именем по SHA-256 содержимого, проверяет уже созданную копию и повторно использует её. Команда передаёт Steam все аргументы напрямую, выставляет `GENSHIN_FPS_TARGET` в выбранное значение и добавляет библиотеку к существующему `LD_PRELOAD`. Вариант `/usr/bin/gfps %command%` без `--fps` по умолчанию выставляет **120 FPS**.

## Требования

- Linux x86-64 с `glibc >= 2.34` (Ubuntu 22.04+, Debian 12+, Fedora 36+, Arch Linux, CachyOS, SteamOS 3+).
- Python 3, PyGObject и GTK 4 для графического интерфейса (устанавливаются автоматически пакетным менеджером при установке `.deb`, `.rpm` или `.pkg.tar.zst`; для CLI `gfps` и `genshin-fps` требуется только `glibc`).
- Genshin Impact, запускаемый через Steam/Proton или Wine.

## Сборка из исходного кода и CLI

```bash
./build.sh
./target/release/genshin-fps steam-short-options 144
./target/release/gfps --fps 144 /usr/bin/env
```

Сборка пакетов для всех дистрибутивов и запуск автоматических проверок:

```bash
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
python3 -m unittest discover -s tests
bash scripts/build-linux-packages.sh
bash tests/test-linux-packages.sh
bash scripts/build-arch-package.sh
bash tests/test-packaging.sh
```

Доступные команды CLI:
- `genshin-fps steam-short-options [FPS]` — короткая строка запуска для Steam (`/.../gfps --fps FPS %command%`).
- `genshin-fps steam-options FPS` — полная строка запуска с явным `LD_PRELOAD`.
- `genshin-fps launch FPS -- WINE_OR_PROTON [ARG ...]` — прямой запуск игры через Wine или Proton.
- `genshin-fps diagnose` — самопроверка загрузки нативной ELF-библиотеки.

## Распространение

Готовые пакеты (`.AppImage`, `.deb`, `.rpm`, `.pkg.tar.zst`) находятся в разделе [Releases](https://github.com/bromhobit/unlock-genshin-for-linux-/releases).

## Важно

Библиотека изменяет целевое значение FPS в оперативной памяти процесса игры. Античит игры может обнаружить модификацию памяти; скрытность и отсутствие санкций не гарантируются. Проект не изменяет файлы игры на диске, не обходит античит и не скрывает процесс или модуль. Для диагностики preload можно задать `GENSHIN_FPS_LOG=1` (журнал сохраняется в `${XDG_STATE_HOME:-$HOME/.local/state}/genshin-fps`).

Алгоритм поиска сигнатур основан на [genshin-fps-unlock](https://github.com/34736384/genshin-fps-unlock) (`UnlockerStub/dllmain.cpp::SetupData`, коммит `2b85d61dd06f6e11ad86fdd6bd90339f9abc58eb`). Лицензия — [MIT](LICENSE). Подробности — [заметки к выпуску](docs/RELEASE-NOTES.md).

---

## 🌐 English

**Native Linux FPS unlocker (10–1000 FPS) for Genshin Impact running under Steam/Proton or Wine.**

Genshin FPS Linux provides a lightweight native ELF preload library (`libgenshin_fps_preload.so`), a short Steam wrapper command (`gfps`), and a **GTK4 GUI** (`Genshin FPS — Steam`) to unlock the frame rate limit in Genshin Impact on Linux. The GUI lets you pick a target between **10 and 1000 FPS** (default **120 FPS**) and copy the ready-to-use Steam Launch Options string in one click without modifying any game files on disk.

### Features

- Configurable FPS cap from **10 to 1000 FPS** (defaults to **120 FPS**).
- **GTK4 GUI** (`Genshin FPS — Steam`) to select your target FPS and copy the Steam Launch Options string to the clipboard.
- Short `gfps` wrapper (`/usr/bin/gfps --fps 144 %command%`) for clean Steam Launch Options.
- Content-addressed **SHA-256** preload staging under `${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps` at game launch so package updates never overwrite an in-use shared library.
- Preserves existing `LD_PRELOAD` entries (compatible with Steam Overlay, MangoHud, etc.) and forwards arguments directly via `exec` without shell evaluation.
- Does not modify game files on disk and does not stage files simply by opening the GUI preview.

### Installation

Pre-built packages for **Linux x86-64** are available in [Release v0.2.4](https://github.com/bromhobit/unlock-genshin-for-linux-/releases/tag/v0.2.4):

- **AppImage** — universal package for Linux distributions, including **SteamOS**. Download the file, make it executable, and run it:
  ```bash
  chmod +x Genshin-FPS-0.2.4-x86_64.AppImage
  ./Genshin-FPS-0.2.4-x86_64.AppImage
  ```
  The AppImage uses built-in self-extraction (FUSE is not required; `--appimage-extract` and `APPIMAGE_EXTRACT_AND_RUN=1` are also supported). On systems without a system package in `/usr/bin`, opening the AppImage GUI automatically installs the user-space `gfps` helper into `~/.local/bin/gfps` so the copied Steam command stays valid after closing the window.

- **DEB** — for **Debian, Ubuntu, Linux Mint, Pop!_OS**: download `genshin-fps-linux_0.2.4_amd64.deb` and install via your package manager or:
  ```bash
  sudo apt install ./genshin-fps-linux_0.2.4_amd64.deb
  ```

- **RPM** — for **Fedora, RHEL, openSUSE**: download `genshin-fps-linux-0.2.4-1.x86_64.rpm` and install with:
  ```bash
  sudo dnf install ./genshin-fps-linux-0.2.4-1.x86_64.rpm
  ```

- **Arch Linux & CachyOS** — download `genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst` and install with:
  ```bash
  sudo pacman -U ./genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst
  ```
  Alternatively, place `genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst` and `install-genshin-fps.sh` in the same folder and run `./install-genshin-fps.sh`. To uninstall: `sudo pacman -Rns genshin-fps-linux`.

#### User Installation from Source (SteamOS / Rootless)

To build and install into `~/.local` without administrator privileges:

```bash
./install-user.sh
~/.local/bin/genshin-fps steam-short-options 144
```

This generates `/home/USER/.local/bin/gfps --fps 144 %command%`. The installer refuses to overwrite an existing `~/.local/bin/gfps` that was not created by this installer. To remove the user installation, run `./uninstall-user.sh`.

### Steam Setup

1. Open **Genshin FPS — Steam** from your application menu (or run `genshin-fps-gui` / the AppImage).
2. Choose your target FPS limit (e.g. `144`) and click **Скопировать для Steam** (Copy for Steam).
3. In **Steam → Library → Genshin Impact → Properties → Launch Options**, paste the command:

```text
/usr/bin/gfps --fps 144 %command%
```

When the game starts, `gfps` stages a verified SHA-256 copy of `libgenshin_fps_preload.so` in `${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps`, sets `GENSHIN_FPS_TARGET`, prepends the library to `LD_PRELOAD`, and executes the game command. Omitting `--fps` (`/usr/bin/gfps %command%`) defaults to **120 FPS**.

### Requirements

- Linux x86-64 with `glibc >= 2.34` (Ubuntu 22.04+, Debian 12+, Fedora 36+, Arch Linux, CachyOS, SteamOS 3+).
- Python 3, PyGObject, and GTK 4 for the GUI (resolved automatically by `apt`, `dnf`, or `pacman` when installing system packages).
- Genshin Impact running under Steam/Proton or Wine.

### Building from Source & Testing

```bash
./build.sh
bash scripts/build-linux-packages.sh
bash tests/test-linux-packages.sh
bash scripts/build-arch-package.sh
bash tests/test-packaging.sh
```

### Distribution

Pre-built release packages (`.AppImage`, `.deb`, `.rpm`, `.pkg.tar.zst`) are published on the [Releases](https://github.com/bromhobit/unlock-genshin-for-linux-/releases) page.

### Disclaimer & Risks

This tool patches the in-memory FPS target of the running game process. In-memory modifications may be detected by the game's anti-cheat system; use at your own risk. The project does not modify game files on disk, does not bypass anti-cheat, and does not hide processes or modules. Set `GENSHIN_FPS_LOG=1` to write preload diagnostic logs to `${XDG_STATE_HOME:-$HOME/.local/state}/genshin-fps`.

Signature resolution is based on [genshin-fps-unlock](https://github.com/34736384/genshin-fps-unlock) (`UnlockerStub/dllmain.cpp::SetupData` at commit `2b85d61dd06f6e11ad86fdd6bd90339f9abc58eb`). Licensed under the [MIT License](LICENSE). See [Release Notes](docs/RELEASE-NOTES.md) for details.
