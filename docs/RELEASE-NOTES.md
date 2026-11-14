# Genshin FPS Linux 0.2.4

## Русский

Пакеты для **Linux x86-64**: **AppImage** (для разных дистрибутивов, включая SteamOS), **DEB** (Debian / Ubuntu / Linux Mint), **RPM** (Fedora / RHEL / openSUSE) и **Arch Linux / CachyOS** (`.pkg.tar.zst`).

- **GTK4 приложение** (`Genshin FPS — Steam`) позволяет выбрать лимит **10–1000 FPS** (по умолчанию 120), показывает короткую команду запуска для Steam (`/usr/bin/gfps --fps 144 %command%`) и копирует её в буфер обмена.
- `genshin-fps steam-short-options 144` печатает абсолютную команду `/usr/bin/gfps --fps 144 %command%` (или путь в `~/.local/bin/gfps` для пользовательской установки / AppImage). Без параметра FPS используется цель `120`.
- `gfps --fps 144` передаёт игре выбранную цель и все аргументы через `exec`, сохраняя существующий `LD_PRELOAD`. Значения вне диапазона `10–1000` отклоняются до размещения библиотеки.
- Preload-библиотека размещается только в момент запуска игры в пользовательском каталоге (`${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps`) под именем с SHA-256 содержимого.

### Установка

- **AppImage (универсальный / SteamOS):**
  ```bash
  chmod +x Genshin-FPS-0.2.4-x86_64.AppImage
  ./Genshin-FPS-0.2.4-x86_64.AppImage
  ```
- **Debian / Ubuntu (DEB):**
  ```bash
  sudo apt install ./genshin-fps-linux_0.2.4_amd64.deb
  ```
- **Fedora / openSUSE (RPM):**
  ```bash
  sudo dnf install ./genshin-fps-linux-0.2.4-1.x86_64.rpm
  ```
- **Arch Linux / CachyOS:**
  ```bash
  sudo pacman -U ./genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst
  ```

---

## English

Packages for **Linux x86-64**: **AppImage** (universal Linux / SteamOS), **DEB** (Debian / Ubuntu / Linux Mint), **RPM** (Fedora / RHEL / openSUSE), and **Arch Linux / CachyOS** (`.pkg.tar.zst`).

- **GTK4 GUI** (`Genshin FPS — Steam`) lets you select a target cap of **10–1000 FPS** (default 120), previews the short Steam Launch Options command (`/usr/bin/gfps --fps 144 %command%`), and copies it to the clipboard.
- `genshin-fps steam-short-options 144` prints `/usr/bin/gfps --fps 144 %command%` (or `~/.local/bin/gfps` for user/AppImage installs). Without an FPS argument, the default is `120`.
- `gfps --fps 144` forwards the selected FPS target and all arguments via `exec` while preserving any existing `LD_PRELOAD`. Values outside `10–1000` are rejected before staging the library.
- The preload library is staged only when launching the game under `${XDG_DATA_HOME:-$HOME/.local/share}/genshin-fps` using a content-addressed SHA-256 filename.

### Installation

- **AppImage (Universal / SteamOS):**
  ```bash
  chmod +x Genshin-FPS-0.2.4-x86_64.AppImage
  ./Genshin-FPS-0.2.4-x86_64.AppImage
  ```
- **Debian / Ubuntu (DEB):**
  ```bash
  sudo apt install ./genshin-fps-linux_0.2.4_amd64.deb
  ```
- **Fedora / openSUSE (RPM):**
  ```bash
  sudo dnf install ./genshin-fps-linux-0.2.4-1.x86_64.rpm
  ```
- **Arch Linux / CachyOS:**
  ```bash
  sudo pacman -U ./genshin-fps-linux-0.2.4-1-x86_64.pkg.tar.zst
  ```
