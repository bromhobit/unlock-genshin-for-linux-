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
            returncode=0, stdout="/usr/bin/gfps %command%\n", stderr=""
        )) as run:
            self.assertEqual(gui.generate_steam_options("/usr/bin/genshin-fps"),
                             "/usr/bin/gfps %command%")
            run.assert_called_once_with(
                ["/usr/bin/genshin-fps", "steam-short-options"],
                capture_output=True, text=True, check=False,
            )

    def test_missing_cli_does_nothing(self):
        with patch.object(gui.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                gui.generate_steam_options(None)
            run.assert_not_called()

    def test_invalid_cli_output_rejected(self):
        with patch.object(gui.subprocess, "run", return_value=SimpleNamespace(
            returncode=0, stdout="wrong\n", stderr=""
        )):
            with self.assertRaises(ValueError):
                gui.generate_steam_options("/tmp/cli")

if __name__ == "__main__":
    unittest.main()
