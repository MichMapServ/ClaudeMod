import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("bridge_pack",ROOT/"scripts/build_fabric_jar_bridge.py")
pkg=importlib.util.module_from_spec(spec)
spec.loader.exec_module(pkg)

class FabricJarBridgePackageTests(unittest.TestCase):
    def test_manifest_is_pinned_and_contains_a_real_loader_hook(self):
        manifest,files=pkg.entries()
        self.assertEqual(manifest["id"],"fabric_jar_bridge")
        self.assertEqual(manifest["game"],"26.3")
        self.assertNotIn("hooks",manifest)
        for key in ("horizon","mixins"):self.assertEqual(len(manifest[key]),40)
        toml=files["mixins/fabric_jar_bridge/horizon-mixin.toml"].decode()
        self.assertIn('target = "horizon_client::game::mods::content_root"',toml)
        self.assertIn("cancellable = true",toml)

    def test_native_bridge_does_not_execute_java(self):
        src=(pkg.SRC/"runtime.rs").read_text()
        self.assertIn('metadata(&mut jar)',src)
        self.assertIn('horizon_mods::content::check_path',src)
        self.assertIn('mod_content::resource_root',src)
        self.assertIn('file.size()>MAX_ITEM',src)
        self.assertIn('MAX_TOTAL',src)
        self.assertNotIn('java -jar',src)
        self.assertNotIn('Command::new("java")',src)

    def test_reproducible_archive_and_fail_closed_verification(self):
        with tempfile.TemporaryDirectory() as d:
            a,b=Path(d)/"a.hnmod",Path(d)/"b.hnmod"
            self.assertEqual(pkg.pack(a),pkg.pack(b))
            self.assertEqual(a.read_bytes(),b.read_bytes())
            with zipfile.ZipFile(a,"a") as z:z.writestr("../evil.rs","!")
            with self.assertRaises(ValueError):pkg.verify(a)
