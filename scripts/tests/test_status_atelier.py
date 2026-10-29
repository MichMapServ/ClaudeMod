import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("status_atelier_pack",ROOT/"scripts/build_status_atelier.py")
pkg=importlib.util.module_from_spec(spec)
spec.loader.exec_module(pkg)

class NativeStatusAtelierTests(unittest.TestCase):
    def test_manifest_and_overrides(self):
        manifest, files=pkg.entries()
        self.assertEqual(manifest["id"], "status_effect_atelier")
        self.assertEqual(manifest["game"], "26.3")
        self.assertNotIn("hooks", manifest)
        toml=files["mixins/status_effect_atelier/horizon-mixin.toml"].decode()
        self.assertEqual(toml.count('kind = "OVERWRITE"'),2)
        self.assertIn("draw_effect_icons",toml)
        self.assertIn("draw_inventory_effects",toml)

    def test_real_game_textures_and_fallback(self):
        source=(pkg.BUNDLE/"runtime.rs").read_text()
        self.assertIn("mob_effect/{id}",source)
        self.assertIn("game.potions.effects.list",source)
        self.assertIn("vanilla_hud",source)
        self.assertIn("inventory_effects::paint",source)

    def test_deterministic_package_and_tamper_detection(self):
        with tempfile.TemporaryDirectory() as t:
            a,b=Path(t)/"first.hnmod",Path(t)/"second.hnmod"
            self.assertEqual(pkg.pack(a),pkg.pack(b))
            self.assertEqual(a.read_bytes(),b.read_bytes())
            with zipfile.ZipFile(a,"a") as z:
                z.writestr("../malicious.rs", "bad")
            with self.assertRaises(ValueError):
                pkg.inspect(a)
