import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from new_mod import adapt_cargo, add_workspace_member

class ScaffoldTests(unittest.TestCase):
    def test_crate(self):
        source = '''[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
horizon-mod-sdk = { git = "https://github.com/horizon-rust/horizon" }

[profile.release]
opt-level = "s"
'''
        output = adapt_cargo(source)
        for value in ('version.workspace = true', 'edition.workspace = true',
                      'license.workspace = true', 'repository.workspace = true',
                      'horizon-mod-sdk.workspace = true'):
            self.assertIn(value, output)
        self.assertNotIn('[profile.release]', output)

    def test_members(self):
        base = '[workspace]\nresolver = "3"\nmembers = []\n'
        updated = add_workspace_member(add_workspace_member(base, 'mods/b'), 'mods/a')
        self.assertIn('members = ["mods/a", "mods/b"]', updated)

    def test_invalid_template(self):
        with self.assertRaises(ValueError):
            adapt_cargo('[package]\nname = "invalid"')

if __name__ == '__main__':
    unittest.main()
