"""Command construction stays testable without GTK or a display."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


GUI = Path(__file__).resolve().parents[1] / "genshin-fps-gui.py"
spec = importlib.util.spec_from_file_location("genshin_fps_gui", GUI)
gui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gui)


class LaunchArgsTests(unittest.TestCase):
    def test_game_path_with_spaces_is_one_argument(self):
        with tempfile.TemporaryDirectory(prefix="game folder ") as directory:
            game = Path(directory) / "GenshinImpact.exe"
            game.touch()
            args, cwd = gui.launch_args("/tmp/genshin-fps", 120, str(game), '"/proton path/proton" run')
            self.assertEqual(
                args,
                ["/tmp/genshin-fps", "launch", "120", "--", "/proton path/proton", "run", str(game)],
            )
            self.assertEqual(cwd, directory)

    def test_rejects_other_files_and_unbalanced_runner(self):
        with tempfile.TemporaryDirectory() as directory:
            wrong = Path(directory) / "other.exe"
            wrong.touch()
            with self.assertRaises(ValueError):
                gui.launch_args("/tmp/genshin-fps", 120, str(wrong), "wine")
            game = Path(directory) / "YuanShen.exe"
            game.touch()
            with self.assertRaises(ValueError):
                gui.launch_args("/tmp/genshin-fps", 120, str(game), "'wine")


if __name__ == "__main__":
    unittest.main()
