#!/usr/bin/env python3
"""Deterministic hnmod packager: native sources only."""
import json, tomllib, hashlib, zipfile, re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
base=root/"native"/"HorizonUtilities"
src=base/"mixins"/"horizon_utilities"
def entries():
    compat=json.loads((base/"compatibility.json").read_text())
    mixin=tomllib.loads((src/"horizon-mixin.toml").read_text())
    assert mixin["mod"]["id"]=="horizon_utilities"
    assert mixin["compatibility"]["horizon_revision"]==compat["horizon_revision"]
    assert all(re.fullmatch("[0-9a-f]{40}",compat[k]) for k in ("horizon_revision","horizon_mixins_revision"))
    for hook in mixin["injection"]:
        assert (src/hook["helper"]).is_file()
    data={"schema":1,"id":mixin["mod"]["id"],"version":mixin["mod"]["version"],"game":compat["game"],"horizon":compat["horizon_revision"],"mixins":compat["horizon_mixins_revision"]}
    files={"horizon-native.json":(json.dumps(data,indent=2,sort_keys=True)+"\n").encode()}
    for p in sorted(src.iterdir()):
        assert p.is_file() and not p.is_symlink() and p.suffix in (".rs",".toml") and p.stat().st_size<=1048576
        files["mixins/"+data["id"]+"/"+p.name]=p.read_bytes()
    return data,files
def check(path):
    _,files=entries()
    with zipfile.ZipFile(path) as z:
        names=z.namelist()
        assert len(names)==len(set(names)) and sorted(names)==sorted(files)
        for info in z.infolist():
            assert info.file_size<=1048576 and info.external_attr>>16 & 0o170000 == 0o100000
            assert z.read(info)==files[info.filename]
    return hashlib.sha256(path.read_bytes()).hexdigest()
def pack(path):
    _,files=entries()
    path.parent.mkdir(exist_ok=True,parents=True)
    with zipfile.ZipFile(path,"w") as z:
        for name,content in sorted(files.items()):
            info=zipfile.ZipInfo(name,(2026,1,1,0,0,0))
            info.compress_type=zipfile.ZIP_DEFLATED
            info.external_attr=0o100644<<16
            z.writestr(info,content)
    return check(path)
if __name__=="__main__":
    import sys
    mode=sys.argv[1] if len(sys.argv)>1 else "pack"
    info,_=entries()
    path=root/"dist"/("HorizonUtilitiesNative-"+info["version"]+".hnmod")
    digest=pack(path) if mode=="pack" else check(path)
    (path.parent/(path.name+".sha256")).write_text(digest+"  "+path.name+"\n")
    print(path, digest)
