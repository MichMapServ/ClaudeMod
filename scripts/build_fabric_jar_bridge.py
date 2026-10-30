#!/usr/bin/env python3
"""Build/check a reproducible source-only Fabric Jar Bridge native bundle."""
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
import zipfile

ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/"native/FabricJarBridge"
SRC=BASE/"mixins/fabric_jar_bridge"
OUT=ROOT/"dist/FabricJarBridge.hnmod"
def entries():
    compat=json.loads((BASE/"compatibility.json").read_text())
    mixin=tomllib.loads((SRC/"horizon-mixin.toml").read_text())
    if mixin["mod"]["id"]!="fabric_jar_bridge":
        raise ValueError("Unexpected mod identifier")
    if mixin["compatibility"]["horizon_revision"]!=compat["horizon_revision"]:
        raise ValueError("Mismatched Horizon revisions")
    if not all(re.fullmatch(r"[0-9a-f]{40}",compat[k]) for k in ("horizon_revision","horizon_mixins_revision")):
        raise ValueError("Expected immutable source pin")
    hooks=mixin["injection"]
    if len({h["id"] for h in hooks})!=len(hooks):
        raise ValueError("Duplicate hook ids")
    if not any(h["kind"]=="HEAD" and h.get("cancellable") and h["target"].endswith("::content_root") for h in hooks):
        raise ValueError("Missing real resource-loader injection")
    for h in hooks:
        name=h["helper"]
        if not re.fullmatch(r"[a-z_]+\.rs",name) or not (SRC/name).is_file():
            raise ValueError("Missing/unsafe injection helper")
    assert mixin["module"][0]["source"]=="runtime.rs"
    manifest=dict(schema=1,id="fabric_jar_bridge",version=mixin["mod"]["version"],
                  game=compat["game"],horizon=compat["horizon_revision"],
                  mixins=compat["horizon_mixins_revision"])
    files={"horizon-native.json":(json.dumps(manifest,sort_keys=True,indent=2)+"\n").encode()}
    for file in sorted(SRC.iterdir()):
        if not file.is_file() or file.is_symlink() or file.suffix not in (".rs",".toml") or file.stat().st_size>1_048_576:
            raise ValueError("Unexpected source file")
        files[f"mixins/fabric_jar_bridge/{file.name}"]=file.read_bytes()
    return manifest,files

def verify(path=OUT):
    _,source=entries()
    with zipfile.ZipFile(path) as z:
        names=z.namelist()
        if set(names)!=set(source) or len(names)!=len(set(names)):
            raise ValueError("Invalid ZIP file names")
        for i in z.infolist():
            if i.file_size>1_048_576 or ((i.external_attr>>16)&0o170000)!=0o100000:
                raise ValueError("Unsafe ZIP entry")
            if z.read(i)!=source[i.filename]:
                raise ValueError("Source contents mismatch")
    return hashlib.sha256(path.read_bytes()).hexdigest()

def pack(path=OUT):
    _,source=entries()
    path.parent.mkdir(exist_ok=True,parents=True)
    with zipfile.ZipFile(path,"w") as z:
        for name,body in sorted(source.items()):
            info=zipfile.ZipInfo(name,(2026,1,1,0,0,0))
            info.compress_type=zipfile.ZIP_DEFLATED
            info.external_attr=0o100644<<16
            z.writestr(info,body)
    return verify(path)

if __name__=="__main__":
    mode=sys.argv[1] if len(sys.argv)>1 else "pack"
    if mode not in ("pack","verify"):
        raise SystemExit("Usage: python3 scripts/build_fabric_jar_bridge.py pack|verify")
    sha=pack() if mode=="pack" else verify()
    sha_file=OUT.with_name(OUT.name+".sha256")
    if mode=="pack":
        sha_file.write_text(sha+"  "+OUT.name+"\n")
    elif not sha_file.exists() or sha_file.read_text().split()[0]!=sha:
        raise SystemExit("Checksum differs")
    print(OUT,sha)
