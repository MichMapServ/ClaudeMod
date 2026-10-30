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

    def test_resource_import_does_not_execute_java_implicitly(self):
        src=(pkg.SRC/"runtime.rs").read_text()
        self.assertIn('metadata(&mut jar)',src)
        self.assertIn('horizon_mods::content::check_path',src)
        self.assertIn('mod_content::resource_root',src)
        self.assertIn('file.size()>MAX_ITEM',src)
        self.assertIn('MAX_TOTAL',src)
        self.assertNotIn('java -jar',src)
        self.assertNotIn('Command::new("java")',src)

    def test_opt_in_jvm_sidecar_is_auditable_and_packaged(self):
        manifest, files = pkg.entries()
        self.assertEqual(manifest["version"], "0.4.0")
        java = files["mixins/fabric_jar_bridge/java_runtime.rs"].decode()
        self.assertIn('Command::new("java")', java)
        self.assertIn('Command::new("javac")', java)
        self.assertIn("fabricbridge-trusted.txt", java)
        self.assertIn("fn trusted(", java)
        self.assertIn("runs_actual_fabric_initializer_and_exposes_command", java)
        toml = files["mixins/fabric_jar_bridge/horizon-mixin.toml"].decode()
        self.assertIn('name = "fabric_java_bridge"', toml)
        self.assertIn('source = "java_runtime.rs"', toml)
        self.assertIn('target = "horizon_client::game::native_hooks::Game<\'_>::native_tick"', toml)
        self.assertIn("ClientTickEvents.END_CLIENT_TICK", java)
        self.assertIn("fn execute(jar:", java)
        self.assertIn("fn execute_group(", java)
        self.assertIn("fn plan_mods(", java)
        self.assertIn("inspect_and_run_all", files["mixins/fabric_jar_bridge/runtime.rs"].decode())
        self.assertIn("fn actual_client_tick_event_uses_live_horizon_position()", java)
        self.assertIn("native_player()", files["mixins/fabric_jar_bridge/tick.rs"].decode())

    def test_reproducible_archive_and_fail_closed_verification(self):
        with tempfile.TemporaryDirectory() as d:
            a,b=Path(d)/"a.hnmod",Path(d)/"b.hnmod"
            self.assertEqual(pkg.pack(a),pkg.pack(b))
            self.assertEqual(a.read_bytes(),b.read_bytes())
            with zipfile.ZipFile(a,"a") as z:z.writestr("../evil.rs","!")
            with self.assertRaises(ValueError):pkg.verify(a)
