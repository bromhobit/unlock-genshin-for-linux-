#!/usr/bin/env python3
"""GTK4 Steam launch options generator with a selectable FPS target."""

import os
from pathlib import Path
import re
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


def generate_steam_options(cli, fps):
    if not cli:
        raise ValueError("CLI не найден. Установите пакет или выполните ./build.sh.")
    if isinstance(fps, bool) or not isinstance(fps, int) or not 10 <= fps <= 1000:
        raise ValueError("FPS должен быть целым числом от 10 до 1000.")
    result = subprocess.run(
        [cli, "steam-short-options", str(fps)],
        capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise ValueError(result.stderr.strip() or "Не удалось получить параметры Steam.")
    value = result.stdout.strip()
    if not re.fullmatch(rf"/[^\s'\"%:\\]+/gfps --fps {fps} %command%", value):
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
            self.set_default_size(520, 280)
            self.set_size_request(380, 250)
            root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
            root.set_margin_top(24)
            root.set_margin_bottom(24)
            root.set_margin_start(28)
            root.set_margin_end(28)
            self.set_child(root)

            title = Gtk.Label(label="Genshin Impact · FPS", xalign=0)
            title.add_css_class("title-1")
            root.append(title)
            description = Gtk.Label(
                label="Скопируйте команду и вставьте её в Steam → Свойства игры → Параметры запуска.",
                xalign=0, wrap=True,
            )
            root.append(description)
            fps_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            fps_row.append(Gtk.Label(label="Лимит FPS", xalign=0, hexpand=True))
            self.fps = Gtk.SpinButton.new_with_range(10, 1000, 1)
            self.fps.set_numeric(True)
            self.fps.set_value(120)
            self.fps.connect("value-changed", self.update_options)
            fps_row.append(self.fps)
            root.append(fps_row)
            self.options = Gtk.Entry(editable=False, hexpand=True)
            root.append(self.options)
            self.copy = Gtk.Button(label="Скопировать для Steam")
            self.copy.add_css_class("suggested-action")
            self.copy.add_css_class("pill")
            self.copy.connect("clicked", self.copy_options)
            root.append(self.copy)
            self.status = Gtk.Label(xalign=0, wrap=True)
            root.append(self.status)
            self.cli = find_cli()
            self.update_options()

        def update_options(self, *_args):
            try:
                value = generate_steam_options(self.cli, self.fps.get_value_as_int())
            except (OSError, ValueError) as exc:
                self.options.set_text("")
                self.copy.set_sensitive(False)
                self.status.set_text(str(exc))
                return None
            self.options.set_text(value)
            self.copy.set_sensitive(True)
            self.status.set_text("Готово для вставки в Steam.")
            return value

        def copy_options(self, _button):
            self.fps.update()
            value = self.update_options()
            if value is None:
                return
            display = Gdk.Display.get_default()
            if display is None:
                self.status.set_text("Буфер обмена недоступен.")
                return
            display.get_clipboard().set_text(value)
            self.status.set_text("Скопировано. Вставьте команду в параметры запуска Steam.")

    app = Gtk.Application(application_id="org.genshinfps.Gui")
    app.connect("activate", lambda app: Window(app).present())
    return app.run(sys.argv)


if __name__ == "__main__":
    raise SystemExit(main())
