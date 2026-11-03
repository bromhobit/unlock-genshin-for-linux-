#!/usr/bin/env python3
"""GTK4 Steam launch options generator for the fixed 120 FPS command."""

import os
from pathlib import Path
import shutil
import subprocess
import sys


def find_cli():
    here = Path(__file__).resolve().parent
    candidates = (
        here / "target/release/genshin-fps",
        here.parent.parent / "bin/genshin-fps",
        Path("/usr/bin/genshin-fps"),
    )
    for path in candidates:
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    return shutil.which("genshin-fps")


def generate_steam_options(cli):
    if not cli:
        raise ValueError("CLI не найден. Установите пакет или выполните ./build.sh.")
    result = subprocess.run(
        [cli, "steam-short-options"], capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise ValueError(result.stderr.strip() or "Не удалось получить параметры Steam.")
    value = result.stdout.strip()
    if not value.endswith("/gfps %command%") or "\n" in value:
        raise ValueError("CLI вернул некорректные параметры Steam.")
    return value


def main():
    try:
        import gi
        gi.require_version("Gtk", "4.0")
        from gi.repository import Gdk, Gtk
    except (ImportError, ValueError) as exc:
        print(f"Нужны GTK4 и PyGObject: {exc}", file=sys.stderr)
        return 1

    class Window(Gtk.ApplicationWindow):
        def __init__(self, app):
            super().__init__(application=app, title="Genshin FPS — Steam")
            self.set_default_size(520, 340)
            self.set_size_request(380, 300)
            root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
            root.set_margin_top(28)
            root.set_margin_bottom(28)
            root.set_margin_start(30)
            root.set_margin_end(30)
            self.set_child(root)

            title = Gtk.Label(label="Genshin Impact · 120 FPS", xalign=0)
            title.add_css_class("title-1")
            root.append(title)
            description = Gtk.Label(
                label="Скопируйте команду и вставьте её в Steam → Свойства игры → Параметры запуска.",
                xalign=0, wrap=True,
            )
            root.append(description)
            self.options = Gtk.Entry(editable=False, hexpand=True)
            root.append(self.options)
            self.copy = Gtk.Button(label="Скопировать для Steam")
            self.copy.add_css_class("suggested-action")
            self.copy.add_css_class("pill")
            self.copy.connect("clicked", self.copy_options)
            root.append(self.copy)
            self.status = Gtk.Label(xalign=0, wrap=True)
            root.append(self.status)
            try:
                self.value = generate_steam_options(find_cli())
                self.options.set_text(self.value)
                self.status.set_text("Готово для вставки в Steam.")
            except (OSError, ValueError) as exc:
                self.value = None
                self.copy.set_sensitive(False)
                self.status.set_text(str(exc))

        def copy_options(self, _button):
            display = Gdk.Display.get_default()
            if display is None:
                self.status.set_text("Буфер обмена недоступен.")
                return
            display.get_clipboard().set_text(self.value)
            self.status.set_text("Скопировано. Вставьте команду в параметры запуска Steam.")

    app = Gtk.Application(application_id="org.genshinfps.Gui")
    app.connect("activate", lambda app: Window(app).present())
    return app.run(sys.argv)


if __name__ == "__main__":
    raise SystemExit(main())
