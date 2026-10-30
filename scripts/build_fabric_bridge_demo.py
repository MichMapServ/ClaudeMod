#!/usr/bin/env python3
"""Build an honest Java bytecode proof JAR for the experimental Fabric bridge.

Requires local javac, does not fetch dependencies, packages only proof/JavaProof.class.
"""
from pathlib import Path
from tempfile import TemporaryDirectory
from subprocess import run
import json
import zipfile

ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/"native/FabricJarBridge/examples/JavaProof.java"
OUT=ROOT/"dist/FabricBridgeJavaProof.jar"
MOD='package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }\n'
API=r'''package dev.horizon.fabricbridge;
public final class HorizonBridge {
    public static void registerCommand(String name, String reply) {}
}
'''
def build(path=OUT):
    with TemporaryDirectory() as td:
        temp=Path(td)
        stubs=[
            ("net/fabricmc/api/ModInitializer.java",MOD),
            ("dev/horizon/fabricbridge/HorizonBridge.java",API),
        ]
        files=[SRC]
        for name,source in stubs:
            p=temp/name
            p.parent.mkdir(parents=True,exist_ok=True)
            p.write_text(source,encoding="utf-8")
            files.append(p)
        classes=temp/"classes"
        classes.mkdir()
        run(["javac","-encoding","UTF-8","-d",str(classes),*[str(p) for p in files]],check=True)
        java=classes/"proof/JavaProof.class"
        assert java.read_bytes()[:4]==b"\xca\xfe\xba\xbe"
        manifest={"schemaVersion":1,"id":"fabricbridge_demo","version":"1.0.0",
                  "name":"Fabric Bridge Java proof",
                  "entrypoints":{"main":["proof.JavaProof"]}}
        path.parent.mkdir(parents=True,exist_ok=True)
        with zipfile.ZipFile(path,"w",compression=zipfile.ZIP_DEFLATED) as zip:
            zip.writestr("fabric.mod.json",json.dumps(manifest,sort_keys=True)+"\n")
            zip.write(java,"proof/JavaProof.class")
    return path
if __name__=="__main__":
    print(build())
