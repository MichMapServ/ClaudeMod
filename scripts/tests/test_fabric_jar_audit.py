import importlib.util
import json
import struct
from pathlib import Path
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("fabric_audit", ROOT / "scripts/audit_fabric_jars.py")
audit_mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit_mod)


def make_jar(path, meta, content=None):
    with ZipFile(path, "w") as out:
        out.writestr("fabric.mod.json", json.dumps(meta))
        for name, value in (content or {}).items():
            out.writestr(name, value)


class FabricJarAuditTests(unittest.TestCase):
    def test_simple_entrypoint_is_only_candidate(self):
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "simple.jar"
            make_jar(jar, {
                "schemaVersion": 1, "id": "proof", "version": "1.0",
                "entrypoints": {"main": ["proof.Example"]}
            }, {
                "assets/proof/lang/en_us.json": '{"hello":"world"}',
                "proof/Example.class": b"\xca\xfe\xba\xbe\x00\x00\x00\x41"
            })
            report = audit_mod.audit(jar)
            self.assertEqual(report["execution"], "candidate_only")
            self.assertEqual(report["resources"], "candidate_only")
            self.assertEqual(report["resource_candidates"], 1)
            self.assertEqual(report["classfile_major_versions"], [65])
            self.assertIsNotNone(report["sha256"])

    def test_bytecode_missing_minecraft_api_is_reported_without_execution(self):
        name = b"net/minecraft/world/World"
        classfile = (b"\xca\xfe\xba\xbe"
                     + struct.pack(">HHH", 0, 65, 3)
                     + b"\x01" + struct.pack(">H", len(name)) + name
                     + b"\x07\x00\x01")
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "unsupported.jar"
            make_jar(jar, {
                "id": "unsupported", "version": "1",
                "entrypoints": {"main": ["test.Entry"]}
            }, {"test/Entry.class": classfile})
            report = audit_mod.audit(jar)
            self.assertEqual(report["format"], "fabric")
            self.assertIn("net/minecraft/world/World", report["unsupported_java_api"])
            self.assertIn("missing_java_api", [x["code"] for x in report["issues"]])
            self.assertEqual(report["execution"], "candidate_only")

    def test_mixin_and_dependency_are_explicit_blockers(self):
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "complex.jar"
            make_jar(jar, {
                "id": "complex", "version": "1", "mixins": ["complex.mixins.json"],
                "depends": {"fabricloader": ">=0.16", "fabric-api": "*"},
                "entrypoints": {"client": ["mod.Example"]}
            })
            report = audit_mod.audit(jar)
            self.assertEqual(report["execution"], "blocked")
            self.assertIn("mixin_required", [x["code"] for x in report["issues"]])
            self.assertIn("external_dependency", [x["code"] for x in report["issues"]])

    def test_fabric_id_with_hyphen_is_recognized_but_not_imported(self):
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "fabric-api.jar"
            make_jar(jar, {"id": "fabric-api", "version": "1",
                           "entrypoints": {"main": ["mod.Main"]}})
            report = audit_mod.audit(jar)
            self.assertEqual(report["format"], "fabric")
            self.assertIn("horizon_id_rewrite", [x["code"] for x in report["issues"]])
            self.assertEqual(report["resource_candidates"], 0)

    def test_rejects_traversal_and_duplicate_archive_names(self):
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "bad.jar"
            with ZipFile(jar, "w") as out:
                out.writestr("fabric.mod.json", '{"id":"proof","version":"1"}')
                out.writestr("assets/proof/../evil", b"test")
                out.writestr("assets/proof/../evil", b"dup")
            report = audit_mod.audit(jar)
            codes = [x["code"] for x in report["issues"]]
            self.assertIn("unsafe_zip_path", codes)
            self.assertIn("duplicate_entries", codes)

    def test_non_fabric_and_unreadable(self):
        with tempfile.TemporaryDirectory() as d:
            plain = Path(d) / "plain.jar"
            with ZipFile(plain, "w") as out:
                out.writestr("plugin.yml", "name: test")
            self.assertEqual(audit_mod.audit(plain)["format"], "other_jar")
            invalid = Path(d) / "invalid.jar"
            invalid.write_bytes(b"not zip")
            self.assertEqual(audit_mod.audit(invalid)["execution"], "blocked")


if __name__ == "__main__":
    unittest.main()
