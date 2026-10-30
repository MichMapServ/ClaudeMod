"""Bounded JVM .class constant-pool inspection; never executes bytecode.

Only extracts symbolic references for a compatibility diagnostic.
This is not a Java verifier, classloader or proof that a code path is executed.
"""
from __future__ import annotations

import re
import struct


class BadClassfile(ValueError):
    pass


# Exact classes provided by the experimental Java shim, plus nested event types.
SUPPORTED = frozenset({
    "net/fabricmc/api/ModInitializer",
    "net/fabricmc/api/ClientModInitializer",
    "net/minecraft/client/MinecraftClient",
    "net/minecraft/client/network/ClientPlayerEntity",
    "net/minecraft/util/math/BlockPos",
    "net/fabricmc/fabric/api/client/event/lifecycle/v1/ClientTickEvents",
    "net/fabricmc/fabric/api/client/event/lifecycle/v1/ClientTickEvents$EndTick",
    "net/fabricmc/fabric/api/client/event/lifecycle/v1/ClientTickEvents$EndTickEvent",
    "dev/horizon/fabricbridge/HorizonBridge",
})
TARGET_PREFIXES = ("net/minecraft/", "net/fabricmc/fabric/", "net/fabricmc/loader/", "org/spongepowered/")
DESCRIPTOR_TYPE = re.compile(r"L((?:net/minecraft|net/fabricmc|org/spongepowered)/[A-Za-z0-9_/$]+);")


def references(data: bytes) -> tuple[int, set[str]]:
    """Read referenced types from a bounded .class constant pool.

    Raises BadClassfile for invalid/truncated/unsupported structures.
    """
    if len(data) < 10 or data[:4] != b"\xca\xfe\xba\xbe":
        raise BadClassfile("signature .class invalide")
    minor, major, count = struct.unpack_from(">HHH", data, 4)
    if count == 0:
        raise BadClassfile("constant pool absent")
    pos = 10
    utf8 = {}
    class_names = []
    index = 1

    def require(n: int):
        if n < 0 or pos + n > len(data):
            raise BadClassfile("constant pool tronquee")

    while index < count:
        require(1)
        tag = data[pos]
        pos += 1
        if tag == 1:  # CONSTANT_Utf8
            require(2)
            size = struct.unpack_from(">H", data, pos)[0]
            pos += 2
            require(size)
            utf8[index] = data[pos:pos+size].decode("utf-8", "replace")
            pos += size
        elif tag in (3, 4):  # int / float
            require(4)
            pos += 4
        elif tag in (5, 6):  # long / double (two pool slots)
            require(8)
            pos += 8
            index += 1
        elif tag == 7:  # CONSTANT_Class
            require(2)
            class_names.append(struct.unpack_from(">H", data, pos)[0])
            pos += 2
        elif tag in (8, 16, 19, 20):  # String / MethodType / Module / Package
            require(2)
            pos += 2
        elif tag in (9, 10, 11, 12, 17, 18):  # refs / NameAndType / dynamic
            require(4)
            pos += 4
        elif tag == 15:  # MethodHandle
            require(3)
            pos += 3
        else:
            raise BadClassfile(f"constant pool tag non reconnu: {tag}")
        index += 1

    found = set()
    for name_index in class_names:
        name = utf8.get(name_index)
        if not name:
            continue
        if name.startswith("["):
            found.update(DESCRIPTOR_TYPE.findall(name))
        elif name.startswith(TARGET_PREFIXES):
            found.add(name)
    # Descriptors mention more classes than CONSTANT_Class entries (method args).
    for value in utf8.values():
        for name in DESCRIPTOR_TYPE.findall(value):
            found.add(name)
    return major, found


def unsupported_java_api(refs: set[str]) -> set[str]:
    """Unknown classes in Minecraft/Fabric/Mixin namespaces (not proof of use)."""
    return {r for r in refs if r.startswith(TARGET_PREFIXES) and r not in SUPPORTED}
