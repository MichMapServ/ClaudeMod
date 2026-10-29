"""Static archive integrity checks; these are NOT Rust/game tests."""
import pathlib
import re
import tomllib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
MOD = ROOT / 'mods' / 'HorizonUtilities'

class ArchiveTests(unittest.TestCase):
    def test_workspace_mod_path(self):
        root = tomllib.loads((ROOT / 'Cargo.toml').read_text())
        self.assertIn('mods/HorizonUtilities', root['workspace']['members'])
        self.assertTrue((MOD / 'Cargo.toml').exists())

    def test_manifest_and_version_consistency(self):
        manifest = tomllib.loads((MOD / 'horizon-mod.toml').read_text())
        workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
        self.assertEqual(manifest['version'], workspace['workspace']['package']['version'])
        self.assertEqual(manifest['id'], 'horizon_utilities')
        self.assertEqual(manifest['side'], 'client')
        self.assertEqual(set(manifest['experimental']), {'input', 'render', 'ui'})
        self.assertNotIn('inventory', manifest['capabilities'])

    def test_internal_api_does_not_claim_missing_host_imports(self):
        libs = (MOD / 'src' / 'lib.rs').read_text()
        extras = (MOD / 'src' / 'extras.rs').read_text()
        self.assertIn('mod extras;', libs)
        self.assertIn('WaypointStore', libs)
        self.assertIn('previous.slot == current.slot', extras)
        self.assertIn('last_tick.saturating_add(1)', extras)
        self.assertIn('cause inconnue', extras)
        self.assertNotIn('unsafe {', libs + extras)

    def test_native_sdk_is_opt_in_and_separate(self):
        workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
        self.assertIn('experimental/horizon-utilities-ext', workspace['workspace']['exclude'])
        experiment = tomllib.loads((ROOT / 'experimental/horizon-utilities-ext' / 'Cargo.toml').read_text())
        self.assertIn('native', experiment['features']['default'])

    def test_validation_and_artifact_checks(self):
        yml = (ROOT / '.github/workflows/hmod.yml').read_text()
        self.assertIn('cargo horizon-mod check "$file"', yml)
        self.assertIn('cargo test --lib', yml)
        self.assertIn('sha256sum', yml)
        self.assertIn('horizon-mod.toml', yml)
        self.assertNotIn('find "$GITHUB_WORKSPACE" -type f -path', yml)

    def test_no_compiled_binaries_in_source_archive(self):
        # The project *publishes* verified binaries under dist/. They must
        # never be committed inside editable mod source directories.
        roots = [ROOT / 'mods', ROOT / 'experimental', ROOT / 'native']
        binaries = [p for folder in roots for pattern in ('*.wasm', '*.hmod', '*.hnmod')
                    for p in folder.rglob(pattern)]
        self.assertEqual(binaries, [])

    def test_published_hmod_checksum_if_present(self):
        import hashlib
        archive = ROOT / 'dist' / 'HorizonUtilities.hmod'
        if archive.exists():
            checksum = archive.with_name(archive.name + '.sha256')
            self.assertTrue(checksum.is_file(), 'published binary lacks checksum')
            expected = checksum.read_text().split()[0]
            self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), expected)

if __name__ == '__main__':
    unittest.main()
