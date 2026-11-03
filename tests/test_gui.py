"""The generator invokes only the side-effect-free CLI command."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
from types import SimpleNamespace

GUI = Path(__file__).resolve().parents[1] / "genshin-fps-gui.py"
spec = importlib.util.spec_from_file_location("genshin_fps_gui", GUI)
gui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gui)


class GeneratorTests(unittest.TestCase):
    def test_exact_options_and_single_safe_command(self):
        with patch.object(gui.subprocess, "run", return_value=SimpleNamespace(
            returncode=0, stdout="/usr/bin/gfps --fps 144 %command%\n", stderr=""
        )) as run:
            self.assertEqual(gui.generate_steam_options("/usr/bin/genshin-fps", 144),
                             "/usr/bin/gfps --fps 144 %command%")
            run.assert_called_once_with(
                ["/usr/bin/genshin-fps", "steam-short-options", "144"],
                capture_output=True, text=True, check=False,
            )

    def test_selected_fps_is_used_on_every_request(self):
        def result(argv, **_kwargs):
            return SimpleNamespace(returncode=0,
                stdout=f"/usr/bin/gfps --fps {argv[2]} %command%\n", stderr="")

        with patch.object(gui.subprocess, "run", side_effect=result) as run:
            self.assertEqual(gui.generate_steam_options("/usr/bin/genshin-fps", 120),
                             "/usr/bin/gfps --fps 120 %command%")
            self.assertEqual(gui.generate_steam_options("/usr/bin/genshin-fps", 1000),
                             "/usr/bin/gfps --fps 1000 %command%")
            self.assertEqual(run.call_count, 2)
            for call in run.call_args_list:
                self.assertEqual(call.args[0][1], "steam-short-options")

    def test_missing_cli_does_nothing(self):
        with patch.object(gui.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                gui.generate_steam_options(None, 120)
            run.assert_not_called()

    def test_invalid_cli_output_rejected(self):
        with patch.object(gui.subprocess, "run", return_value=SimpleNamespace(
            returncode=0, stdout="wrong\n", stderr=""
        )):
            with self.assertRaises(ValueError):
                gui.generate_steam_options("/tmp/cli", 120)

    def test_wrong_target_and_invalid_fps_rejected(self):
        with patch.object(gui.subprocess, "run", return_value=SimpleNamespace(
            returncode=0, stdout="/usr/bin/gfps --fps 120 %command%\n", stderr=""
        )) as run:
            with self.assertRaises(ValueError):
                gui.generate_steam_options("/tmp/cli", 144)
            for target in (9, 1001, "120", 120.5, True):
                with self.assertRaises(ValueError):
                    gui.generate_steam_options("/tmp/cli", target)
            run.assert_called_once()

    def test_cli_error_rejected(self):
        with patch.object(gui.subprocess, "run", return_value=SimpleNamespace(
            returncode=2, stdout="", stderr="bad fps"
        )):
            with self.assertRaisesRegex(ValueError, "bad fps"):
                gui.generate_steam_options("/tmp/cli", 120)

if __name__ == "__main__":
    unittest.main()
