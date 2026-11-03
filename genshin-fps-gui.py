#!/usr/bin/env python3
"""Small GTK4 frontend for the separate native genshin-fps CLI."""

import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading


GAME_NAMES = {"GenshinImpact.exe", "YuanShen.exe"}


def find_cli():
    here = Path(__file__).resolve().parent
    candidates = (
        here / "target/release/genshin-fps",
        here.parent.parent / "bin/genshin-fps",  # installed under ~/.local/lib/genshin-fps
    )
    for path in candidates:
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    return shutil.which("genshin-fps")


def launch_args(cli, fps, game_text, runner_text):
    if not cli:
        raise ValueError("CLI genshin-fps не найден. Выполните ./build.sh или ./install-user.sh.")
    game = Path(game_text.strip()).expanduser()
    if game.name not in GAME_NAMES or not game.is_file():
        raise ValueError("Выберите существующий GenshinImpact.exe или YuanShen.exe.")
    try:
        runner = shlex.split(runner_text)
    except ValueError as exc:
        raise ValueError(f"Ошибка в команде Wine/Proton: {exc}") from exc
    if not runner:
        raise ValueError("Укажите команду Wine или Proton.")
    if not 10 <= fps <= 1000:
        raise ValueError("FPS должен быть от 10 до 1000.")
    game = game.absolute()
    return [cli, "launch", str(fps), "--", *runner, str(game)], str(game.parent)


def main():
    try:
        import gi

        gi.require_version("Gtk", "4.0")
        from gi.repository import Gdk, GLib, Gtk
    except (ImportError, ValueError) as exc:
        print(
            "Не удалось загрузить GTK4/PyGObject. Установите python-gobject и gtk4 "
            f"(Arch/CachyOS/SteamOS). Подробности: {exc}",
            file=sys.stderr,
        )
        return 1

    cli = find_cli()

    class Window(Gtk.ApplicationWindow):
        def __init__(self, app):
            super().__init__(application=app, title="Genshin FPS — Linux")
            self.set_default_size(760, 600)
            self.preview_text = ""
            self.active_process = None

            root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
            root.set_margin_top(20)
            root.set_margin_bottom(20)
            root.set_margin_start(24)
            root.set_margin_end(24)
            self.set_child(root)

            title = Gtk.Label(label="Genshin FPS для Linux", xalign=0)
            title.add_css_class("title-1")
            root.append(title)
            subtitle = Gtk.Label(
                label="Нативная библиотека через Wine/Proton. Изменение FPS в игре здесь не проверяется.",
                xalign=0,
                wrap=True,
            )
            root.append(subtitle)

            grid = Gtk.Grid(column_spacing=12, row_spacing=12)
            root.append(grid)

            def label(text, row):
                widget = Gtk.Label(label=text, xalign=0)
                widget.set_valign(Gtk.Align.CENTER)
                grid.attach(widget, 0, row, 1, 1)

            label("Целевой FPS", 0)
            self.fps = Gtk.SpinButton.new_with_range(10, 1000, 1)
            self.fps.set_value(120)
            self.fps.set_hexpand(True)
            grid.attach(self.fps, 1, 0, 1, 1)

            label("Файл игры", 1)
            game_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            self.game = Gtk.Entry(hexpand=True)
            self.game.set_placeholder_text("/путь/к/GenshinImpact.exe")
            game_row.append(self.game)
            browse = Gtk.Button(label="Выбрать…")
            browse.connect("clicked", self.choose_game)
            game_row.append(browse)
            grid.attach(game_row, 1, 1, 1, 1)

            label("Команда Wine/Proton", 2)
            self.runner = Gtk.Entry(hexpand=True)
            self.runner.set_text("wine")
            self.runner.set_placeholder_text("wine или /путь/к/proton run")
            grid.attach(self.runner, 1, 2, 1, 1)

            label("WINEPREFIX (необязательно)", 3)
            self.prefix = Gtk.Entry(hexpand=True)
            self.prefix.set_placeholder_text("/путь/к/prefix")
            grid.attach(self.prefix, 1, 3, 1, 1)

            actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            root.append(actions)
            self.launch_button = Gtk.Button(label="Запустить игру")
            self.launch_button.add_css_class("suggested-action")
            self.launch_button.connect("clicked", self.launch_game)
            actions.append(self.launch_button)
            copy_button = Gtk.Button(label="Скопировать параметры Steam")
            copy_button.connect("clicked", self.copy_steam)
            actions.append(copy_button)
            check_button = Gtk.Button(label="Проверить библиотеку")
            check_button.connect("clicked", self.diagnose)
            actions.append(check_button)

            preview_label = Gtk.Label(label="Параметры запуска Steam", xalign=0)
            preview_label.add_css_class("heading")
            root.append(preview_label)
            self.preview = Gtk.TextView(editable=False, cursor_visible=False, wrap_mode=Gtk.WrapMode.CHAR)
            self.preview.set_monospace(True)
            self.preview.set_size_request(-1, 72)
            preview_scroll = Gtk.ScrolledWindow()
            preview_scroll.set_child(self.preview)
            preview_scroll.set_min_content_height(80)
            root.append(preview_scroll)

            status_label = Gtk.Label(label="Состояние и сообщения", xalign=0)
            status_label.add_css_class("heading")
            root.append(status_label)
            self.status = Gtk.TextView(editable=False, cursor_visible=False, wrap_mode=Gtk.WrapMode.WORD_CHAR)
            self.status.set_monospace(True)
            status_scroll = Gtk.ScrolledWindow(vexpand=True)
            status_scroll.set_child(self.status)
            status_scroll.set_min_content_height(130)
            root.append(status_scroll)
            self.message("Готово. Запуск игры выполняется только по нажатию кнопки.")
            if not cli:
                self.message("CLI genshin-fps не найден. Выполните ./build.sh или ./install-user.sh.")
            self.fps.connect("value-changed", self.refresh_preview)
            self.refresh_preview()

        def message(self, value):
            buffer = self.status.get_buffer()
            current = buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), False)
            buffer.set_text((current + value + "\n")[-8000:])
            mark = buffer.get_insert()
            self.status.scroll_to_mark(mark, 0.0, True, 0.0, 1.0)

        def choose_game(self, _button):
            chooser = Gtk.FileChooserNative(
                title="Выберите файл игры",
                transient_for=self,
                action=Gtk.FileChooserAction.OPEN,
                accept_label="Выбрать",
                cancel_label="Отмена",
            )
            game_filter = Gtk.FileFilter()
            game_filter.set_name("GenshinImpact.exe / YuanShen.exe")
            for name in GAME_NAMES:
                game_filter.add_pattern(name)
            chooser.add_filter(game_filter)

            def selected(dialog, response):
                if response == Gtk.ResponseType.ACCEPT:
                    file = dialog.get_file()
                    if file and file.get_path():
                        self.game.set_text(file.get_path())
                dialog.destroy()

            chooser.connect("response", selected)
            chooser.show()

        def cli_output(self, args):
            if not cli:
                raise ValueError("CLI genshin-fps не найден. Выполните ./build.sh или ./install-user.sh.")
            try:
                result = subprocess.run([cli, *args], capture_output=True, text=True, timeout=15)
            except (OSError, subprocess.TimeoutExpired) as exc:
                raise RuntimeError(str(exc)) from exc
            if result.returncode:
                raise RuntimeError(result.stderr.strip() or f"CLI завершился с кодом {result.returncode}")
            return result.stdout.strip()

        def refresh_preview(self, *_args):
            try:
                self.preview_text = self.cli_output(["steam-options", str(self.fps.get_value_as_int())])
            except (ValueError, RuntimeError) as exc:
                self.preview_text = ""
                self.preview.get_buffer().set_text(f"Ошибка: {exc}")
                return
            self.preview.get_buffer().set_text(self.preview_text)

        def copy_steam(self, _button):
            self.refresh_preview()
            if not self.preview_text:
                self.message("Не удалось получить параметры Steam; подробности указаны в предпросмотре.")
                return
            display = Gdk.Display.get_default()
            if display is None:
                self.message("Буфер обмена недоступен: нет графического дисплея.")
                return
            display.get_clipboard().set_text(self.preview_text)
            self.message("Параметры Steam скопированы. Проверьте их в поле предпросмотра.")

        def launch_game(self, _button):
            if self.active_process and self.active_process.poll() is None:
                self.message("Команда запуска уже выполняется.")
                return
            try:
                args, cwd = launch_args(
                    cli, self.fps.get_value_as_int(), self.game.get_text(), self.runner.get_text()
                )
                env = os.environ.copy()
                prefix = self.prefix.get_text().strip()
                if prefix:
                    env["WINEPREFIX"] = str(Path(prefix).expanduser().absolute())
                errors = tempfile.TemporaryFile()
                try:
                    process = subprocess.Popen(
                        args, cwd=cwd, env=env, stdout=subprocess.DEVNULL, stderr=errors
                    )
                except OSError:
                    errors.close()
                    raise
                self.active_process = process
                self.message("Команда Wine/Proton запущена. FPS в игре не проверен.")
                self.launch_button.set_sensitive(False)

                def await_exit():
                    try:
                        code = process.wait()
                        errors.seek(max(0, errors.tell() - 6000))
                        details = errors.read().decode("utf-8", errors="replace").strip()
                    finally:
                        errors.close()
                    GLib.idle_add(self.launch_finished, code, details)

                threading.Thread(target=await_exit, daemon=True).start()
            except (OSError, ValueError) as exc:
                self.message(f"Ошибка запуска: {exc}")

        def launch_finished(self, code, details):
            self.active_process = None
            self.launch_button.set_sensitive(True)
            self.message(f"Команда Wine/Proton завершилась с кодом {code}.")
            if details and code:
                self.message(details)
            return GLib.SOURCE_REMOVE

        def diagnose(self, _button):
            self.message("Проверка загрузки нативной библиотеки…")

            def work():
                try:
                    output = self.cli_output(["diagnose"])
                    message = output or "Проверка завершилась без вывода."
                except (ValueError, RuntimeError) as exc:
                    message = f"Ошибка проверки: {exc}"
                GLib.idle_add(self.message, message)

            threading.Thread(target=work, daemon=True).start()

    app = Gtk.Application(application_id="org.genshinfps.Gui")
    app.connect("activate", lambda app: Window(app).present())
    return app.run(sys.argv)


if __name__ == "__main__":
    raise SystemExit(main())
