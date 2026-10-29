import importlib.util,tempfile,unittest,zipfile
from pathlib import Path
path=Path(__file__).resolve().parents[1]/"build_hnmod.py"
spec=importlib.util.spec_from_file_location("pack_hnmod",path)
mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
class TestHnmod(unittest.TestCase):
    def test_pins_and_sources(self):
        info,entries=mod.entries()
        self.assertEqual(info["id"],"horizon_utilities")
        self.assertEqual(len(info["horizon"]),40)
        self.assertEqual(info["hooks"],2)
        self.assertGreaterEqual(len(entries),10)
    def test_determinism_and_integrity(self):
        with tempfile.TemporaryDirectory() as d:
            a,b=Path(d)/"one.hnmod",Path(d)/"two.hnmod"
            self.assertEqual(mod.pack(a),mod.pack(b))
            self.assertEqual(a.read_bytes(),b.read_bytes())
            with zipfile.ZipFile(a) as z:
                import json
                manifest=json.loads(z.read("horizon-native.json"))
                self.assertEqual(manifest["hooks"],2)
            with zipfile.ZipFile(a,"a") as z: z.writestr("../unexpected.rs","evil")
            with self.assertRaises(AssertionError):mod.check(a)
