"""Declared language parser timings through pinned Divan 0.1.21; never CPU/RSS estimates."""
from __future__ import annotations

import dataclasses
import hashlib
import json
import math
import re
import subprocess
import tomllib
from pathlib import Path

from ci.perf.use_case import Refused

SCHEMA = "divan-timing/v1"
DIVAN = "0.1.21"
FIELDS = {"schema", "workloads", "metrics"}
TIME = re.compile(r"([0-9]+(?:\.[0-9]+)?) (ps|ns|µs|ms|s)")
SCALE = {"ps": 0.001, "ns": 1.0, "µs": 1000.0, "ms": 1_000_000.0, "s": 1_000_000_000.0}
BRANCH = re.compile(r"(?P<prefix>(?:│  |   )*)(?:├─ |╰─ )(?P<name>[A-Za-z_][A-Za-z_0-9]*|[0-9]+)")


@dataclasses.dataclass(frozen=True)
class LanguageBench:
    """One existing Cargo target and its explicitly declared workload identities."""
    name: str
    package: str
    directory: Path
    target: str
    source_path: str
    source: bytes
    workloads: dict[str, list[int]]
    legacy: bool

    def keys(self) -> set[str]:
        """Required leaves are declared before any benchmark runs."""
        return {f"{self.name}/{workload}/{argument}/median-ns"
                for workload, arguments in self.workloads.items() for argument in arguments}


def digest(value: bytes) -> str:
    """Unframed content SHA for the recorded input provenance."""
    return hashlib.sha256(value).hexdigest()


def registry(tree: Path, fallback: dict[str, LanguageBench] | None = None) -> list[LanguageBench]:
    """Discover the existing workspace registry; missing language manifests/settings never disappear."""
    workspace = tree / "crates/ryo"
    try:
        manifest = tomllib.loads((workspace / "Cargo.toml").read_text())
        members = {path.resolve() for pattern in manifest["workspace"]["members"]
                   for path in workspace.glob(pattern) if path.is_dir()}
        languages = sorted(path for path in (workspace / "languages").iterdir() if path.is_dir())
        locked = tomllib.loads((workspace / "Cargo.lock").read_text())
    except (OSError, ValueError, KeyError) as error:
        raise Refused(f"language registry cannot be read: {error}") from error
    pins = [item["version"] for item in locked.get("package", []) if item.get("name") == "divan"]
    if pins != [DIVAN]:
        raise Refused(f"language timing decoder requires pinned Divan {DIVAN}, found {pins}")
    if not languages:
        raise Refused("language registry is empty")
    found = []
    for directory in languages:
        if directory.resolve() not in members:
            raise Refused(f"language {directory.name} is absent from the Cargo workspace")
        try:
            package = tomllib.loads((directory / "Cargo.toml").read_text())
            all_targets = package.get("bench", [])
            targets = [target for target in all_targets if target.get("name") in {"provider", "performance"}]
            current = [{"name": "provider", "path": "benchmarks/provider/bench.rs", "harness": False}]
            historical = [{"name": "performance", "path": "benches/performance.rs", "harness": False}]
            if targets == current:
                target, source_path = "provider", "benchmarks/provider/bench.rs"
            elif fallback is not None and targets == historical:
                target, source_path = "performance", "benches/performance.rs"
            else:
                raise Refused(f"{directory.name}: one explicit provider target is required")
            other_targets = [item for item in all_targets if item not in targets]
            for item in other_targets:
                parts = Path(item.get("path", "")).parts
                if (len(parts) != 4 or parts[:2] != ("benchmarks", "linters")
                        or parts[3] != "bench.rs" or item.get("name") != parts[2]
                        or item.get("harness") is not False):
                    raise Refused(f"{directory.name}: misplaced or ambiguous benchmark target")
            if (directory / "benches/performance.rs").exists() and target != "performance":
                raise Refused(f"{directory.name}: legacy provider bench remains beside the provider target")
            source = (directory / source_path).read_bytes()
            declared = package["package"].get("metadata", {}).get("ryo", {}).get("performance")
            name = package["package"]["name"]
        except (OSError, ValueError, KeyError) as error:
            raise Refused(f"{directory.name}: incomplete language benchmark: {error}") from error
        expected_package = f"idiomatic-sh-ryo-{directory.name}"
        if name != expected_package:
            raise Refused(f"{directory.name}: package identity differs from its language")
        legacy = declared is None
        if legacy:
            previous = (fallback or {}).get(directory.name)
            if previous is None or previous.source != source or previous.package != name:
                raise Refused(f"{directory.name}: baseline-required; no declared timing identity or identical established workload")
            workloads = previous.workloads
        else:
            if not isinstance(declared, dict) or set(declared) != FIELDS or declared["schema"] != SCHEMA or declared["metrics"] != ["median-ns"]:
                raise Refused(f"{directory.name}: unsupported language timing contract")
            workloads = declared["workloads"]
            if not isinstance(workloads, dict) or not workloads:
                raise Refused(f"{directory.name}: empty workload registry")
            for workload, arguments in workloads.items():
                if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", workload) or not isinstance(arguments, list) or not arguments:
                    raise Refused(f"{directory.name}: malformed workload declaration")
                if any(type(value) is not int or value <= 0 for value in arguments) or len(set(arguments)) != len(arguments):
                    raise Refused(f"{directory.name}: arguments must be distinct positive integers")
        found.append(LanguageBench(directory.name, name, directory, target, source_path, source, workloads, legacy))
    if fallback is not None:
        if {entry.name for entry in found} != set(fallback):
            raise Refused("baseline-required; base and head language inventories differ")
        for entry in found:
            head = fallback[entry.name]
            if entry.workloads != head.workloads or entry.source != head.source:
                raise Refused(f"{entry.name}: baseline-required; workload identity/source changed")
    return found


def declared_metrics(tree: Path) -> set[str]:
    """Use explicit Cargo/TM declarations, not observed table keys."""
    return {key for language in registry(tree) for key in language.keys()}


def duration(value: str) -> float:
    """Convert only the pinned native time units; no synthesized resource observations."""
    matched = TIME.fullmatch(value.strip())
    if matched is None:
        raise Refused(f"invalid Divan timing {value!r}")
    amount, unit = matched.groups()
    result = float(amount) * SCALE[unit]
    if not math.isfinite(result) or result <= 0:
        raise Refused("Divan timing must be finite and positive")
    return result


def decode(document: str, benchmark: LanguageBench) -> dict[str, float]:
    """Strict pinned two-level workload/argument table; every declared leaf must occur exactly once."""
    result: dict[str, float] = {}
    heading = False
    workload = None
    parents: set[str] = set()
    for raw in document.splitlines():
        if not raw.strip():
            continue
        parts = raw.split(" │")
        columns = [parts[0].rstrip(), *(part.strip() for part in parts[1:])]
        if len(columns) != 6:
            raise Refused(f"{benchmark.name}: malformed Divan table row {raw!r}")
        first = columns[0]
        if first.endswith("fastest"):
            if heading or first.removesuffix("fastest").strip() != benchmark.target or columns[1:] != ["slowest", "median", "mean", "samples", "iters"]:
                raise Refused(f"{benchmark.name}: invalid or duplicate Divan heading")
            heading = True
            continue
        if not heading:
            raise Refused("Divan data precedes its pinned heading")
        if not any(columns[1:]):
            branch = BRANCH.fullmatch(first)
            if branch is None or branch["prefix"] or branch["name"] not in benchmark.workloads:
                raise Refused(f"{benchmark.name}: unexpected Divan workload {first!r}")
            workload = branch["name"]
            if workload in parents:
                raise Refused(f"{benchmark.name}: duplicate Divan workload {workload}")
            parents.add(workload)
            continue
        leaf = re.fullmatch(r"(.*?)\s+([0-9]+(?:\.[0-9]+)? (?:ps|ns|µs|ms|s))", first)
        branch = BRANCH.fullmatch(leaf[1]) if leaf is not None else None
        if branch is None or len(branch["prefix"]) != 3 or workload is None:
            raise Refused(f"{benchmark.name}: malformed Divan leaf {first!r}")
        argument = branch["name"]
        if not argument.isdecimal() or int(argument) not in benchmark.workloads[workload]:
            raise Refused(f"{benchmark.name}: undeclared Divan argument {argument!r}")
        for value in [leaf[2], *columns[1:4]]:
            duration(value)
        if any(not value.isdecimal() or int(value) <= 0 for value in columns[4:]):
            raise Refused(f"{benchmark.name}: empty/invalid native samples or iterations")
        key = f"{benchmark.name}/{workload}/{argument}/median-ns"
        if key in result:
            raise Refused(f"duplicate Divan measurement {key}")
        result[key] = duration(columns[2])
    if result.keys() != benchmark.keys():
        raise Refused(f"{benchmark.name}: missing or extra declared Divan leaves")
    return result


def measure_languages(head: Path, tree: Path, output: Path | None, label: str, channel: str) -> dict[str, float]:
    """Retain raw table/command/config provenance, refusing absent or changed baseline workloads."""
    import os
    required = {entry.name: entry for entry in registry(head)}
    selected = registry(tree, required) if tree.resolve() != head.resolve() else list(required.values())
    output = output or head / "target/benchmarks/speed"
    output.mkdir(parents=True, exist_ok=True)
    measured = {}
    lock_path = tree / "crates/ryo/Cargo.lock"
    workspace_path = tree / "crates/ryo/Cargo.toml"
    lock_bytes, workspace_bytes = lock_path.read_bytes(), workspace_path.read_bytes()
    for entry in selected:
        command = ["cargo", "bench", "--locked", "--package", entry.package, "--bench", entry.target, "--", "--color", "never"]
        manifest_bytes = (entry.directory / "Cargo.toml").read_bytes()
        result = subprocess.run(command, cwd=tree / "crates/ryo", env={**os.environ, "RUSTUP_TOOLCHAIN": channel},
                                check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        stem = f"{label}-ryo-languages-{entry.name}"
        (output / f"{stem}.stdout.txt").write_text(result.stdout)
        (output / f"{stem}.stderr.txt").write_text(result.stderr)
        evidence = {"schema": SCHEMA, "divan": DIVAN, "command": command, "status": result.returncode,
                    "language": entry.name, "package": entry.package, "target": entry.target,
                    "source_path": entry.source_path, "required_metrics": sorted(entry.keys()),
                    "workload_source_sha256": digest(entry.source),
                    "cargo_manifest_sha256": digest(manifest_bytes),
                    "cargo_lock_sha256": digest(lock_bytes),
                    "cargo_workspace_sha256": digest(workspace_bytes),
                    "baseline_registry": "head-required/identical-established-source" if entry.legacy else "explicit-cargo-metadata",
                    "measurement": "Divan per-operation timing; no CPU/RSS observation"}
        (output / f"{stem}.command.json").write_text(json.dumps(evidence, indent=2) + "\n")
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode, command, output=result.stdout, stderr=result.stderr)
        if lock_path.read_bytes() != lock_bytes or workspace_path.read_bytes() != workspace_bytes:
            raise Refused(f"{entry.name}: benchmark workspace or lock changed during measurement")
        if (entry.directory / "Cargo.toml").read_bytes() != manifest_bytes:
            raise Refused(f"{entry.name}: benchmark registry changed during measurement")
        if (entry.directory / entry.source_path).read_bytes() != entry.source:
            raise Refused(f"{entry.name}: benchmark source changed during measurement")
        measured.update(decode(result.stdout, entry))
    if measured.keys() != {key for entry in required.values() for key in entry.keys()}:
        raise Refused("language benchmark result inventory is incomplete")
    (output / f"{label}-ryo-languages.metrics.json").write_text(json.dumps(measured, indent=2) + "\n")
    return measured
