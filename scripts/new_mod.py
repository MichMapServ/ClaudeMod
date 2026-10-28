#!/usr/bin/env python3
"""Create an official Horizon mod and register code crates in Cargo workspace."""
import argparse
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SLUG = re.compile(r"[a-z][a-z0-9-]*\Z")

def adapt_cargo(text):
    for pattern, replacement in [
        (r'^version = "0\.1\.0"$', 'version.workspace = true\nlicense.workspace = true\nrepository.workspace = true'),
        (r'^edition = "2024"$', 'edition.workspace = true'),
        (r'^horizon-mod-sdk = \{ git = "https://github\.com/(?:horizon-rust|ludovic111)/horizon" \}$', 'horizon-mod-sdk.workspace = true')
    ]:
        text, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
        if count != 1:
            raise ValueError('Unexpected Horizon template format: ' + pattern)
    return re.sub(r'\n\[profile\.release\][\s\S]*\Z', '\n', text)

def add_workspace_member(manifest, member):
    match = re.search(r'(?m)^members = \[(.*?)\]$', manifest)
    if not match:
        raise ValueError('Missing Cargo workspace members declaration')
    names = sorted(set(re.findall(r'"([^"]+)"', match.group(1)) + [member]))
    replacement = 'members = [' + ', '.join('"' + name + '"' for name in names) + ']'
    return manifest[:match.start()] + replacement + manifest[match.end():]

def main():
    parser = argparse.ArgumentParser(description='Create a Horizon mod in mods/')
    parser.add_argument('slug')
    parser.add_argument('--name')
    parser.add_argument('--template', choices=['code', 'content'], default='code')
    args = parser.parse_args()
    if not SLUG.fullmatch(args.slug):
        parser.error('Use lowercase letters, digits and hyphens; start with a letter.')
    folder = ROOT / 'mods' / args.slug
    if folder.exists():
        parser.error('Mod folder already exists: ' + str(folder))
    root_file = ROOT / 'Cargo.toml'
    root_manifest = root_file.read_text(encoding='utf-8')
    updated_root = (add_workspace_member(root_manifest, 'mods/' + args.slug)
                    if args.template == 'code' else root_manifest)
    cmd = ['cargo', 'horizon-mod', 'new', str(folder), '--template', args.template]
    if args.name:
        cmd.extend(['--name', args.name])
    subprocess.run(cmd, cwd=ROOT, check=True)
    if args.template == 'code':
        mod_manifest = folder / 'Cargo.toml'
        mod_manifest.write_text(adapt_cargo(mod_manifest.read_text(encoding='utf-8')), encoding='utf-8')
        root_file.write_text(updated_root, encoding='utf-8')
    print('Created mods/' + args.slug + ' (' + args.template + ')')

if __name__ == '__main__':
    main()
