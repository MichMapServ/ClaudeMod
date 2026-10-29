#!/usr/bin/env python3
"""Deterministic, verified source-only hnmod packer for Status Effect Atelier."""
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / "native" / "StatusEffectAtelier"
BUNDLE = MOD / "mixins" / "status_effect_atelier"
OUTPUT = ROOT / "dist" / "StatusEffectAtelier.hnmod"

def entries():
    comp = json.loads((MOD / "compatibility.json").read_text())
    mixin = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text())
    if mixin["mod"]["id"] != "status_effect_atelier":
        raise ValueError("Invalid mod ID")
    if mixin["compatibility"]["horizon_revision"] != comp["horizon_revision"]:
        raise ValueError("Horizon revision mismatch")
    for key in ("horizon_revision", "horizon_mixins_revision"):
        if not re.fullmatch(r"[0-9a-f]{40}", comp[key]):
            raise ValueError("Expected exact source SHA for " + key)
    if "hooks" in comp:
        raise ValueError("Non-stable rendering overwrites require an exact Horizon pin")
    helpers = [i["helper"] for i in mixin["injection"]]
    if len(set(helpers)) != len(helpers):
        raise ValueError("Duplicate helpers")
    if not all(re.fullmatch(r"[a-z][a-z0-9_]*\.rs", h) and (BUNDLE / h).is_file() for h in helpers):
        raise ValueError("Missing or unsafe helper")
    manifest = {"schema": 1, "id": mixin["mod"]["id"], "version": mixin["mod"]["version"],
                "game": comp["game"], "horizon": comp["horizon_revision"], "mixins": comp["horizon_mixins_revision"]}
    files = {"horizon-native.json": (json.dumps(manifest,sort_keys=True,indent=2)+"\n").encode()}
    for path in sorted(BUNDLE.iterdir()):
        if not path.is_file() or path.is_symlink() or path.suffix not in (".rs", ".toml") or path.stat().st_size > 1_048_576:
            raise ValueError("Unexpected native source " + str(path))
        files["mixins/status_effect_atelier/"+path.name] = path.read_bytes()
    return manifest, files

def inspect(path):
    _, files = entries()
    with zipfile.ZipFile(path) as z:
        if len(z.namelist()) != len(set(z.namelist())) or set(z.namelist()) != set(files):
            raise ValueError("Archive paths mismatch")
        for info in z.infolist():
            if info.file_size > 1_048_576 or (info.external_attr >> 16) & 0o170000 != 0o100000:
                raise ValueError("Unsafe ZIP entry " + info.filename)
            if z.read(info) != files[info.filename]:
                raise ValueError("Archive content mismatch " + info.filename)
    return hashlib.sha256(path.read_bytes()).hexdigest()

def pack(path=OUTPUT):
    _, files = entries()
    path.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(path,"w") as z:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            z.writestr(info,data)
    return inspect(path)

if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv)>1 else "pack"
    if mode not in ("pack", "verify"):
        raise SystemExit("Usage: python scripts/build_status_atelier.py pack|verify")
    digest = pack() if mode=="pack" else inspect(OUTPUT)
    checksum = OUTPUT.with_name(OUTPUT.name+".sha256")
    if mode=="pack":
        checksum.write_text(digest+"  "+OUTPUT.name+"\n")
    elif not checksum.is_file() or checksum.read_text().split()[0] != digest:
        raise SystemExit("Checksum mismatch")
    print(OUTPUT, digest)
