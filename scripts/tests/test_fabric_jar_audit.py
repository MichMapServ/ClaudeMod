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


    def test_missing_entrypoint_and_malformed_metadata_block_execution(self):
        cases = [
            ("missing_class", {"id": "proof", "version": "1",
                "entrypoints": {"main": ["missing.Main"]}}, "missing_entrypoint_class"),
            ("invalid_entrypoint", {"id": "proof", "version": "1",
                "entrypoints": {"main": [33]}}, "invalid_entrypoint"),
            ("custom_entrypoint", {"id": "proof", "version": "1",
                "entrypoints": {"custom": ["test.Main"]}}, "unsupported_entrypoint_kind"),
            ("wrong_entrypoints", {"id": "proof", "version": "1",
                "entrypoints": ["test.Main"]}, "invalid_entrypoints"),
            ("wrong_depends", {"id": "proof", "version": "1",
                "depends": ["fabric-api"]}, "invalid_dependencies"),
            ("wrong_mixins", {"id": "proof", "version": "1",
                "mixins": 18}, "invalid_mixins"),
            ("wrong_jars", {"id": "proof", "version": "1",
                "jars": {"file": "nested.jar"}}, "nested_jars"),
            ("wrong_environment", {"id": "proof", "version": "1",
                "environment": "both"}, "environment"),
        ]
        with tempfile.TemporaryDirectory() as d:
            for name, meta, expected in cases:
                jar = Path(d) / (name + ".jar")
                make_jar(jar, meta)
                report = audit_mod.audit(jar)
                self.assertEqual(report["execution"], "blocked", name)
                self.assertIn(expected, [i["code"] for i in report["issues"]], name)

    def test_dependency_versions_are_unverified_not_implicitly_compatible(self):
        with tempfile.TemporaryDirectory() as d:
            jar = Path(d) / "version.jar"
            make_jar(jar, {"id": "proof", "version": "1",
                           "depends": {"minecraft": "1.20.1", "java": ">=21"}})
            report = audit_mod.audit(jar)
            self.assertIn("unchecked_version_constraint",
                          [i["code"] for i in report["issues"]])

    def test_audit_set_finds_local_approved_candidate_without_executing(self):
        with tempfile.TemporaryDirectory() as d:
            lib = Path(d) / "library.jar"
            consumer = Path(d) / "consumer.jar"
            make_jar(lib, {"id": "library", "version": "1.2.0"})
            make_jar(consumer, {"id": "consumer", "version": "1.0.0",
                                "depends": {"library": ">=1.0 <2"},
                                "entrypoints": {"main": ["consumer.Main"]}},
                     {"consumer/Main.class": b"\xca\xfe\xba\xbe\x00\x00\x00\x41"})
            result = audit_mod.audit_set([consumer, lib])
            self.assertEqual(result[0]["execution"], "candidate_only")
            self.assertIn("local_dependency_found",
                          [x["code"] for x in result[0]["issues"]])
            self.assertEqual(audit_mod.audit(consumer)["execution"], "blocked")
            self.assertEqual(result[1]["execution"], "no_entrypoint")
            self.assertEqual(audit_mod._predicate("1.5.0", "^1.2.3"), True)
            self.assertEqual(audit_mod._predicate("2.0", "^1.2.3"), False)

    def test_audit_set_rejects_cycles_and_wrong_versions(self):
        with tempfile.TemporaryDirectory() as d:
            a, b = Path(d) / "alpha.jar", Path(d) / "beta.jar"
            make_jar(a, {"id": "alpha", "version": "1.0",
                         "depends": {"beta": "*"}})
            make_jar(b, {"id": "beta", "version": "1.0",
                         "depends": {"alpha": "*"}})
            result = audit_mod.audit_set([a, b])
            self.assertEqual([x["execution"] for x in result], ["blocked", "blocked"])
            for report in result:
                self.assertIn("dependency_cycle", [x["code"] for x in report["issues"]])
            make_jar(b, {"id": "beta", "version": "2.0.0"})
            make_jar(a, {"id": "alpha", "version": "1.0",
                         "depends": {"beta": "<2"}})
            result = audit_mod.audit_set([a, b])
            self.assertIn("dependency_version_mismatch",
                          [x["code"] for x in result[0]["issues"]])

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
