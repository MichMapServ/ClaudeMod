#!/usr/bin/env python3
"""Build an honest Java bytecode proof JAR for the experimental Fabric bridge.

Requires local javac, does not fetch dependencies, packages only proof/JavaProof.class and proof/ClientTickProof.class.
"""
from pathlib import Path
from tempfile import TemporaryDirectory
from subprocess import run
import json
import zipfile

ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/"native/FabricJarBridge/examples/JavaProof.java"
TICK_SRC=ROOT/"native/FabricJarBridge/examples/ClientTickProof.java"
OUT=ROOT/"dist/FabricBridgeJavaProof.jar"
MOD='package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }\n'
API=r'''package dev.horizon.fabricbridge;
public final class HorizonBridge {
    public static void registerCommand(String name, String reply) {}
}
'''

CLIENT_MOD='package net.fabricmc.api; public interface ClientModInitializer { void onInitializeClient(); }\n'
BLOCK_POS='''package net.minecraft.util.math;
public class BlockPos {
    public int getX(){return 0;}
    public int getY(){return 0;}
    public int getZ(){return 0;}
}
'''
PLAYER='''package net.minecraft.client.network;
import net.minecraft.util.math.BlockPos;
public class ClientPlayerEntity {
    public BlockPos getBlockPos(){return null;}
}
'''
MC='''package net.minecraft.client;
import net.minecraft.client.network.ClientPlayerEntity;
public class MinecraftClient {
    public ClientPlayerEntity player;
}
'''
TICKS='''package net.fabricmc.fabric.api.client.event.lifecycle.v1;
import net.minecraft.client.MinecraftClient;
public final class ClientTickEvents {
    @FunctionalInterface public interface EndTick {void onEndTick(MinecraftClient client);}
    public interface EndTickEvent {void register(EndTick callback);}
    public static final EndTickEvent END_CLIENT_TICK=callback->{};
}
'''
def build(path=OUT):
    with TemporaryDirectory() as td:
        temp=Path(td)
        stubs=[
            ("net/fabricmc/api/ModInitializer.java",MOD),
            ("net/fabricmc/api/ClientModInitializer.java",CLIENT_MOD),
            ("net/minecraft/util/math/BlockPos.java",BLOCK_POS),
            ("net/minecraft/client/network/ClientPlayerEntity.java",PLAYER),
            ("net/minecraft/client/MinecraftClient.java",MC),
            ("net/fabricmc/fabric/api/client/event/lifecycle/v1/ClientTickEvents.java",TICKS),
            ("dev/horizon/fabricbridge/HorizonBridge.java",API),
        ]
        files=[SRC,TICK_SRC]
        for name,source in stubs:
            p=temp/name
            p.parent.mkdir(parents=True,exist_ok=True)
            p.write_text(source,encoding="utf-8")
            files.append(p)
        classes=temp/"classes"
        classes.mkdir()
        run(["javac","-encoding","UTF-8","-d",str(classes),*[str(p) for p in files]],check=True)
        java=classes/"proof/JavaProof.class"
        tick=classes/"proof/ClientTickProof.class"
        for file in (java,tick):
            assert file.read_bytes()[:4]==b"\xca\xfe\xba\xbe"
        manifest={"schemaVersion":1,"id":"fabricbridge_demo","version":"1.0.0",
                  "name":"Fabric Bridge Java proof",
                  "entrypoints":{"main":["proof.JavaProof"],"client":["proof.ClientTickProof"]}}
        path.parent.mkdir(parents=True,exist_ok=True)
        with zipfile.ZipFile(path,"w",compression=zipfile.ZIP_DEFLATED) as zip:
            zip.writestr("fabric.mod.json",json.dumps(manifest,sort_keys=True)+"\n")
            zip.write(java,"proof/JavaProof.class")
            zip.write(tick,"proof/ClientTickProof.class")
    return path
if __name__=="__main__":
    print(build())
