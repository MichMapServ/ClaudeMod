#!/usr/bin/env python3
"""Offline Fabric JAR auditor for Horizon Fabric Jar Bridge (no Java execution).

Usage:
    python scripts/audit_fabric_jars.py path/to/mod.jar [other.jar ...] --json

This is deliberately a diagnostic, not a promise of mod compatibility.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path, PurePosixPath
from zipfile import ZipFile, BadZipFile
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from fabric_bytecode_inspector import references, unsupported_java_api, BadClassfile

MAX_JAR = 128 * 1024 * 1024
MAX_ENTRIES = 4096
MAX_META = 65536
MAX_RESOURCE = 8 * 1024 * 1024
MAX_RESOURCE_TOTAL = 64 * 1024 * 1024
MAX_CLASS_ANALYZE = 512 * 1024
MAX_BYTECODE_ANALYZE = 8 * 1024 * 1024
MAX_CLASS_FILES = 256
FABRIC_ID = re.compile(r"^[a-z][a-z0-9_-]{0,63}$")
HORIZON_ID = re.compile(r"^[a-z0-9_]{1,64}$")
ALLOWED_TOP = ("assets", "data")
EXEC_EXT = (".class", ".jar", ".dll", ".exe", ".so", ".dylib", ".bat", ".ps1", ".sh", ".wasm")
BASE_DEPENDENCIES = frozenset({"minecraft", "fabricloader", "java"})
RESERVED = {"fabricbridge", "help", "list", "trust"}


def issue(issues: list[dict], severity: str, code: str, message: str) -> None:
    issues.append({"severity": severity, "code": code, "message": message})


def path_safe(name: str) -> bool:
    if not name or len(name) > 512 or "\\" in name or name.startswith("/") or ":" in name or "\x00" in name:
        return False
    path = PurePosixPath(name)
    return all(part not in ("", ".", "..") for part in name.rstrip("/").split("/")) and ".." not in path.parts


def valid_entrypoints(entrypoints: object, issues: list[dict]) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    if not isinstance(entrypoints, dict):
        issue(issues, "blocker", "invalid_entrypoints", "entrypoints doit etre un objet")
        return found
    for kind, elements in entrypoints.items():
        if kind not in ("main", "client") and elements:
            issue(issues, "blocker", "unsupported_entrypoint_kind", f"Entrypoint non pris en charge: {kind}")
        if not isinstance(elements, list):
            issue(issues, "blocker", "invalid_entrypoints", f"Entrypoints {kind}: liste attendue")
            continue
        if len(elements) > 100:
            issue(issues, "blocker", "entrypoint_limit", f"Entrypoints {kind}: plus de 100 entrees")
        for value in elements[:100]:
            adapter = "default"
            if isinstance(value, dict):
                adapter, value = value.get("adapter", "default"), value.get("value")
            if not isinstance(value, str):
                issue(issues, "blocker", "invalid_entrypoint", f"Entrypoint {kind} invalide")
                continue
            found.append((kind, value))
            if adapter != "default":
                issue(issues, "blocker", "language_adapter", f"Adapter {adapter} necessaire")
            if "::" in value or not re.fullmatch(r"[A-Za-z0-9_.$]{1,180}", value):
                issue(issues, "blocker", "method_entrypoint", f"Syntaxe non prise en charge: {value[:120]}")
    return found


def audit(path: Path) -> dict:
    result = {
        "file": str(path), "format": "unknown", "id": None, "version": None,
        "sha256": None, "entrypoints": [], "java_classes": 0,
        "resource_candidates": 0, "classfile_major_versions": [],
        "dependencies": {}, "mixins": [], "unsupported_java_api": [], "issues": [],
        "execution": "not_evaluated", "resources": "not_evaluated",
    }
    issues = result["issues"]
    try:
        size = path.stat().st_size
        if size > MAX_JAR:
            issue(issues, "blocker", "jar_size_limit", f"JAR depasse {MAX_JAR} octets")
            return result
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for block in iter(lambda: stream.read(65536), b""):
                digest.update(block)
        result["sha256"] = digest.hexdigest()

        with ZipFile(path) as z:
            names = z.namelist()
            if len(names) > MAX_ENTRIES:
                issue(issues, "blocker", "entry_limit", "Trop d'entrees ZIP")
                return result
            if len(names) != len(set(names)):
                issue(issues, "blocker", "duplicate_entries", "Entrees ZIP dupliquees")
            info = z.infolist()
            total_unpacked = 0
            for entry in info:
                if not path_safe(entry.filename):
                    issue(issues, "blocker", "unsafe_zip_path", f"Chemin archive non sur: {entry.filename[:120]}")
                if (entry.external_attr >> 16) & 0o170000 == 0o120000:
                    issue(issues, "blocker", "symlink", f"Lien symbolique refuse: {entry.filename[:120]}")
                total_unpacked += entry.file_size
                if entry.file_size > 64 * 1024 * 1024 or total_unpacked > 256 * 1024 * 1024:
                    issue(issues, "blocker", "inflated_size_limit", "Taille ZIP decompressee excessive")
                    break
                if entry.file_size and (not entry.compress_size or entry.file_size // entry.compress_size > 500):
                    issue(issues, "blocker", "compression_ratio", f"Ratio ZIP suspect: {entry.filename[:120]}")
            if "fabric.mod.json" not in names:
                result["format"] = "other_jar"
                issue(issues, "info", "not_fabric", "Aucun fabric.mod.json")
                return result
            result["format"] = "fabric"
            meta_entry = z.getinfo("fabric.mod.json")
            if meta_entry.file_size > MAX_META:
                issue(issues, "blocker", "manifest_size", "Manifest trop volumineux")
                return result
            with z.open(meta_entry) as stream:
                try:
                    meta = json.loads(stream.read(MAX_META + 1).decode("utf-8"))
                except (UnicodeError, ValueError) as error:
                    issue(issues, "blocker", "manifest_json", f"Manifest JSON invalide: {error}")
                    return result
            if not isinstance(meta, dict):
                issue(issues, "blocker", "manifest_shape", "Manifest JSON doit etre un objet")
                return result
            mod_id = meta.get("id")
            result["id"] = mod_id if isinstance(mod_id, str) else None
            result["version"] = str(meta.get("version", "?"))[:80]
            if not isinstance(mod_id, str) or not FABRIC_ID.fullmatch(mod_id):
                issue(issues, "blocker", "invalid_mod_id", "Mod ID Fabric invalide")
            elif not HORIZON_ID.fullmatch(mod_id):
                issue(issues, "warning", "horizon_id_rewrite", "ID Fabric avec tiret: ressources non importables par le bridge actuel")
            env = meta.get("environment", "*")
            if env == "server":
                issue(issues, "blocker", "server_only", "Mod reserve au serveur Java")
            elif env not in ("client", "*"):
                issue(issues, "blocker", "environment", f"Environment non prise en charge: {str(env)[:80]}")
            eps = valid_entrypoints(meta.get("entrypoints", {}), issues)
            result["entrypoints"] = [{"kind": kind, "class": name} for kind, name in eps[:100]]
            for kind, class_name in eps:
                if kind not in ("main", "client"):
                    continue
                # Fabric permits class-only entrypoints; method entrypoints are
                # already rejected. The current JVM runner requires a class in
                # the top-level JAR, not a nested library or classpath provider.
                if re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_.$]{0,179}", class_name):
                    class_path = class_name.replace(".", "/") + ".class"
                    if class_path not in names:
                        issue(issues, "blocker", "missing_entrypoint_class",
                              f"Classe initialisatrice absente: {class_name[:120]}")
            if len([k for k, _ in eps if k in ("main", "client")]) > 8:
                issue(issues, "blocker", "too_many_entrypoints", "Plus de 8 entrypoints")
            mixes = meta.get("mixins", [])
            if isinstance(mixes, (str, dict)):
                mixes = [mixes]
            if isinstance(mixes, list):
                result["mixins"] = [str(x)[:150] for x in mixes[:100]]
                if mixes:
                    issue(issues, "blocker", "mixin_required", "Mixins Java non transformes dans Horizon Rust")
            else:
                issue(issues, "blocker", "invalid_mixins", "Definition mixins mal formee")
            if "accessWidener" in meta:
                issue(issues, "blocker", "access_widener", "AccessWidener Java non applique")
            if "jars" in meta and meta["jars"] != []:
                issue(issues, "blocker", "nested_jars", "JARs embarques non resolus")
            dependencies = meta.get("depends", {})
            if isinstance(dependencies, dict):
                result["dependencies"] = {str(k)[:64]: v for k, v in list(dependencies.items())[:100]}
                for dep, constraint in dependencies.items():
                    if dep not in BASE_DEPENDENCIES:
                        issue(issues, "blocker", "external_dependency", f"Dependance manquante/non resolue: {dep}")
                    elif constraint != "*":
                        issue(issues, "warning", "unchecked_version_constraint",
                              f"Version de {dep} demandee ({str(constraint)[:80]}); non verifiee par le bridge")
            else:
                issue(issues, "blocker", "invalid_dependencies", "Champ depends invalide")
            if any(name.startswith(("net/minecraft/", "net/fabricmc/fabric/", "org/spongepowered/")) for name in names if name.endswith(".class")):
                issue(issues, "warning", "bundled_java_api", "Classes Minecraft/Fabric/Mixin embarquees: risque de conflit de classpath")
            if any(name.endswith(".mixins.json") for name in names):
                issue(issues, "warning", "mixin_config_present", "Configuration Mixin presente dans le JAR")
            total = 0
            versions = set()
            refs = set()
            bytecode_analyzed = 0
            classes_analyzed = 0
            for entry in info:
                name = entry.filename
                if name.endswith(".class"):
                    result["java_classes"] += 1
                    if entry.file_size >= 8:
                        with z.open(entry) as stream:
                            header = stream.read(8)
                        if header[:4] == b"\xca\xfe\xba\xbe":
                            versions.add(int.from_bytes(header[6:8], "big"))
                    if (classes_analyzed < MAX_CLASS_FILES
                            and entry.file_size <= MAX_CLASS_ANALYZE
                            and bytecode_analyzed + entry.file_size <= MAX_BYTECODE_ANALYZE):
                        with z.open(entry) as stream:
                            content = stream.read(MAX_CLASS_ANALYZE + 1)
                        bytecode_analyzed += len(content)
                        classes_analyzed += 1
                        try:
                            _, class_references = references(content)
                            refs.update(class_references)
                        except BadClassfile:
                            pass
                    continue
                if not (name.startswith("assets/") or name.startswith("data/")) or entry.is_dir():
                    continue
                if not isinstance(mod_id, str) or not HORIZON_ID.fullmatch(mod_id):
                    continue
                valid_own = name.startswith((f"assets/{mod_id}/", f"data/{mod_id}/"))
                valid_shared = name.startswith(("data/minecraft/tags/", "data/c/tags/"))
                if not (valid_own or valid_shared) or name.lower().endswith(EXEC_EXT):
                    continue
                if entry.file_size > MAX_RESOURCE:
                    issue(issues, "blocker", "resource_limit", f"Ressource trop volumineuse: {name[:100]}")
                    continue
                total += entry.file_size
                result["resource_candidates"] += 1
            result["classfile_major_versions"] = sorted(versions)
            missing = unsupported_java_api(refs)
            result["unsupported_java_api"] = sorted(missing)[:60]
            if missing:
                issue(issues, "warning", "missing_java_api", f"{len(missing)} reference(s) a des classes Minecraft/Fabric/Mixin non implementees; le lancement peut echouer")
            if classes_analyzed < result["java_classes"]:
                issue(issues, "info", "partial_bytecode_scan",
                      f"Analyse bornee: {classes_analyzed}/{result['java_classes']} classes inspectees")
            if total > MAX_RESOURCE_TOTAL:
                issue(issues, "blocker", "resource_total", "Total de ressources trop volumineux")
            if not eps:
                issue(issues, "info", "no_java_entrypoint", "Aucun main/client Java a initialiser")
            if result["java_classes"] and not eps:
                issue(issues, "info", "java_classes_no_entrypoint", "Classes Java presentes sans entrypoints standards")
    except (OSError, BadZipFile, RuntimeError, ValueError, EOFError) as e:
        issue(issues, "blocker", "unreadable_archive", f"Impossible de lire archive: {e}")
    blockers = [i for i in issues if i["severity"] == "blocker"]
    result["execution"] = ("blocked" if blockers else
                           "candidate_only" if result["entrypoints"] else "no_entrypoint")
    result["resources"] = ("candidate_only" if result["resource_candidates"] and
                           not any(i["code"] in {"unsafe_zip_path", "duplicate_entries", "symlink",
                              "inflated_size_limit", "compression_ratio",
                              "resource_total", "resource_limit"} for i in blockers)
                           else "none_or_blocked")
    return result


def _numeric_version(value: str) -> tuple[int, int, int]:
    """Common numeric Fabric version subset; non-numeric versions are unknown."""
    if not isinstance(value, str) or "-" in value:
        raise ValueError("prerelease/non-numeric version")
    parts = value.split("+", 1)[0].split(".")
    if not 1 <= len(parts) <= 3 or any(not x.isdigit() for x in parts):
        raise ValueError("unsupported version syntax")
    return tuple(map(int, parts + ["0"] * (3 - len(parts))))


def _predicate(actual: str, required: str) -> bool:
    if required == "*":
        return True
    if not isinstance(required, str) or len(required) > 160:
        raise ValueError("invalid dependency predicate")
    clauses = required.split()
    if len(clauses) > 1:
        return all(_predicate(actual, clause) for clause in clauses)
    if "||" in required or "*" in required:
        raise ValueError("unsupported wildcard/alternative syntax")
    op = next((candidate for candidate in ("<=", ">=", "==", "=", "<", ">", "^", "~")
               if required.startswith(candidate)), "=")
    expression = required[len(op):] if required.startswith(op) else required
    current, requested = _numeric_version(actual), _numeric_version(expression)
    if op in ("=", "=="):
        return current == requested
    if op == "<":
        return current < requested
    if op == ">":
        return current > requested
    if op == "<=":
        return current <= requested
    if op == ">=":
        return current >= requested
    if op in ("^", "~"):
        upper = list(requested)
        idx = (0 if requested[0] else 1 if requested[1] else 2) if op == "^" else (
            1 if "." in expression else 0)
        upper[idx] += 1
        for j in range(idx + 1, 3):
            upper[j] = 0
        return requested <= current < tuple(upper)
    raise ValueError("unsupported version operator")


def audit_set(paths: list[Path]) -> list[dict]:
    """Offline batch report with locally available dependency diagnostics.

    This does not execute mods or prove that the Horizon JVM can initialize them.
    """
    reports = [audit(path) for path in paths]
    available: dict[str, dict] = {}
    for report in reports:
        mod_id = report["id"]
        if report["format"] != "fabric" or not isinstance(mod_id, str):
            continue
        if mod_id in available:
            for item in (available[mod_id], report):
                issue(item["issues"], "blocker", "duplicate_mod_id",
                      f"Identifiant Fabric duplique dans le lot: {mod_id}")
        else:
            available[mod_id] = report

    for report in reports:
        if report["format"] != "fabric":
            continue
        dependencies = report["dependencies"]
        kept = []
        for entry in report["issues"]:
            if entry["code"] != "external_dependency":
                kept.append(entry)
                continue
            name = entry["message"].rsplit(": ", 1)[-1]
            dep = available.get(name)
            if dep is None:
                kept.append(entry)
                continue
            predicate = dependencies.get(name, "*")
            if isinstance(predicate, str):
                predicates = [predicate]
            elif isinstance(predicate, list) and predicate and all(isinstance(x, str) for x in predicate):
                predicates = predicate
            else:
                kept.append(entry)
                continue
            try:
                compatible = any(_predicate(dep["version"], p) for p in predicates)
            except (ValueError, TypeError):
                kept.append(entry)
                issue(kept, "blocker", "unsupported_dependency_version",
                      f"Contrainte de version non prise en charge: {name}")
                continue
            if compatible:
                issue(kept, "info", "local_dependency_found",
                      f"Archive locale trouvee pour {name} (autorisation et API JVM non verifiees)")
            else:
                kept.append(entry)
                issue(kept, "blocker", "dependency_version_mismatch",
                      f"Version incompatible pour {name}: {dep['version']}")
        report["issues"] = kept

    # A local dependency is only a candidate if all its prerequisites are
    # candidates. Bound the fixed point to the supplied file count.
    for _ in reports:
        changed = False
        for report in reports:
            if report["format"] != "fabric":
                continue
            if any(i["code"] == "blocked_dependency" for i in report["issues"]):
                continue
            for name in report["dependencies"]:
                if name in BASE_DEPENDENCIES:
                    continue
                dep = available.get(name)
                if dep and any(i["severity"] == "blocker" for i in dep["issues"]):
                    issue(report["issues"], "blocker", "blocked_dependency",
                          f"Dependance locale elle-meme bloquee: {name}")
                    changed = True
                    break
        if not changed:
            break

    # Eliminate nodes in topological order. Remaining candidates form
    # dependency cycles (or chains depending on those cycles).
    pending = {
        mod_id for mod_id, report in available.items()
        if not any(i["severity"] == "blocker" for i in report["issues"])
    }
    for _ in range(len(pending)):
        ready = {
            mod_id for mod_id in pending
            if not any(dep in pending for dep in available[mod_id]["dependencies"]
                       if dep not in BASE_DEPENDENCIES)
        }
        if not ready:
            break
        pending -= ready
    for mod_id in pending:
        issue(available[mod_id]["issues"], "blocker", "dependency_cycle",
              f"Cycle ou chaine de dependances cyclique: {mod_id}")

    for report in reports:
        if report["format"] == "fabric":
            blocked = any(i["severity"] == "blocker" for i in report["issues"])
            report["execution"] = ("blocked" if blocked else
                                   "candidate_only" if report["entrypoints"] else "no_entrypoint")
    return reports


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("jars", type=Path, nargs="+")
    parser.add_argument("--json", action="store_true", help="JSON machine-readable report")
    args = parser.parse_args()
    reports = audit_set(args.jars)
    if args.json:
        print(json.dumps(reports, ensure_ascii=False, indent=2))
    else:
        for report in reports:
            print(f"{report['file']} : {report['format']} id={report['id']} "
                  f"execution={report['execution']} resources={report['resources']}")
            for item in report["issues"]:
                print(f"  [{item['severity']}] {item['code']}: {item['message']}")
    return 1 if any(r["format"] == "unknown" for r in reports) else 0


if __name__ == "__main__":
    raise SystemExit(main())
