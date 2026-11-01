"""Materialize and verify prepared Ruff candidate profiles."""

from __future__ import annotations

import argparse
import collections
import contextlib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

import tomllib

RUFF_DIR = Path(__file__).resolve().parent
REPOSITORY = RUFF_DIR.parents[6]
PROFILE_DIR = RUFF_DIR / "profiles"
MANIFEST = Path(os.environ.get("RYO_RUFF_PREPARED_MANIFEST", PROFILE_DIR / "prepared.json"))
SHAPE_PROFILE = "shape-110"
RULE_PHASES_FILE = PROFILE_DIR / "pinned-rule-phases.json"
PINNED_RUFF_VERSION = "0.16.5"
PINNED_RUFF_SOURCE_COMMIT = "9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a"
RUFF_PHASES = {
    "ast",
    "filesystem",
    "io",
    "imports",
    "logical-lines",
    "noqa",
    "physical-lines",
    "toml",
    "tokens",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def atomic_json_write(path: Path, value: object) -> None:
    """Commit a machine-readable receipt by rename after its bytes reach disk."""
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    with temporary.open("w", encoding="utf-8") as output:
        output.write(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
        output.flush()
        os.fsync(output.fileno())
    temporary.replace(path)


def source_hash(files: dict[str, str]) -> str:
    encoded = json.dumps(files, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def diagnostic_multiset(findings: list[dict[str, object]]) -> collections.Counter[tuple[object, ...]]:
    """Compare exact reference findings while retaining duplicate diagnostics."""
    return collections.Counter(
        (finding["rule"], finding["file"], finding["start"], finding["end"])
        for finding in findings
    )


def toml_value(value: object) -> str:
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        entries = ", ".join(
            f"{json.dumps(str(key), ensure_ascii=False)} = {toml_value(item)}"
            for key, item in value.items()
        )
        return "{ " + entries + " }"
    raise TypeError(f"unsupported benchmark TOML value: {type(value).__name__}")


def write_cases(path: Path, cases: list[dict[str, object]]) -> None:
    lines = []
    for case in cases:
        lines.extend(
            [
                "[[cases]]",
                f"name = {toml_value(case['name'])}",
                f"comparison = {toml_value(case.get('comparison', 'rule'))}",
                f"files = {toml_value(case['files'])}",
                f"expected = {toml_value(case['expected'])}",
                "",
            ]
        )
    path.write_text("\n".join(lines), encoding="utf-8")


def manifest_paths(manifest: dict[str, object], helper_path: str) -> set[str]:
    profiles = manifest["profiles"]
    base_profile = RUFF_DIR / "benchmark.toml"
    paths = {helper_path, base_profile.relative_to(REPOSITORY).as_posix()}
    paths.add(RULE_PHASES_FILE.relative_to(REPOSITORY).as_posix())
    base_data = tomllib.loads(base_profile.read_text(encoding="utf-8"))
    paths.update(module_path(path) for path in profile_modules(base_data).values())
    for profile_name, recorded in profiles.items():
        rule = recorded.get("rule_id", profile_name)
        profile = prepared_profile_path(profile_name, recorded)
        case_file = prepared_case_path(rule, recorded)
        candidate_file = prepared_candidate_path(rule, recorded)
        paths.update({
                profile.relative_to(REPOSITORY).as_posix(),
                candidate_file.relative_to(REPOSITORY).as_posix(),
                case_file.relative_to(REPOSITORY).as_posix(),
            })
        profile_data = tomllib.loads(profile.read_text(encoding="utf-8"))
        paths.update(module_path(path) for path in profile_modules(profile_data).values())
        if "oracle_file" in recorded:
            paths.add(recorded["oracle_file"])
        if "reference_audit_file" in recorded:
            paths.add(recorded["reference_audit_file"])
    for descriptor in manifest.get("combined_profiles", {}).values():
        paths.update(descriptor["input_files"])
        combined_profile = REPOSITORY / descriptor["profile_file"]
        combined_data = tomllib.loads(combined_profile.read_text(encoding="utf-8"))
        paths.update(module_path(path) for path in profile_modules(combined_data).values())
    return paths


def prepared_profile_path(profile_name: str, recorded: dict[str, object]) -> Path:
    """Resolve a sealed profile TOML, allowing target-specific profile aliases."""
    value = recorded.get("profile_file", f"profiles/{profile_name}/benchmark.toml")
    if not isinstance(value, str):
        raise ValueError(f"prepared profile path must be a string for {profile_name}")
    path = Path(value)
    if path.is_absolute() or not path.parts or any(part in (".", "..") for part in path.parts):
        raise ValueError(f"unsafe prepared profile path for {profile_name}: {value}")
    source = RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f"prepared profile path traverses a symlink for {profile_name}: {value}")
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(RUFF_DIR.resolve()):
        raise ValueError(f"prepared profile path is outside Ruff profile for {profile_name}: {value}")
    return resolved


def module_path(value: object) -> str:
    if not isinstance(value, str):
        raise ValueError("query module paths must be strings")
    path = Path(value)
    if path.is_absolute() or not path.parts or any(part in (".", "..") for part in path.parts):
        raise ValueError(f"unsafe query module path: {value}")
    source = RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f"query module path traverses a symlink: {value}")
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(RUFF_DIR.resolve()):
        raise ValueError(f"query module is outside the Ruff profile: {value}")
    return resolved.relative_to(REPOSITORY).as_posix()


def prepared_case_path(rule: str, recorded: dict[str, object]) -> Path:
    """Resolve a profile-specific frozen case snapshot, or its canonical rule case file."""
    value = recorded.get("cases_file", f"rules/{rule}/cases.toml")
    if not isinstance(value, str):
        raise ValueError(f"prepared case file path must be a string for {rule}")
    path = Path(value)
    if path.is_absolute() or not path.parts or any(part in (".", "..") for part in path.parts):
        raise ValueError(f"unsafe prepared case path for {rule}: {value}")
    source = RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f"prepared case path traverses a symlink for {rule}: {value}")
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(RUFF_DIR.resolve()):
        raise ValueError(f"prepared case path is outside the Ruff profile for {rule}: {value}")
    return resolved


def prepared_candidate_path(rule: str, recorded: dict[str, object]) -> Path:
    """Resolve a profile-local frozen query snapshot or the canonical candidate."""
    value = recorded.get("candidate_file", f"rules/{rule}/candidate.ryoql")
    if not isinstance(value, str):
        raise ValueError(f"prepared candidate file path must be a string for {rule}")
    path = Path(value)
    if path.is_absolute() or not path.parts or any(part in (".", "..") for part in path.parts):
        raise ValueError(f"unsafe prepared candidate path for {rule}: {value}")
    source = RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f"prepared candidate path traverses a symlink for {rule}: {value}")
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(RUFF_DIR.resolve()):
        raise ValueError(f"prepared candidate path is outside the Ruff profile for {rule}: {value}")
    return resolved


def profile_modules(profile_data: dict[str, object]) -> dict[str, str]:
    registry = profile_data.get("query-modules", {})
    if not isinstance(registry, dict):
        raise ValueError("query-modules must be a table")
    result = {}
    for logical_id, path in registry.items():
        if not isinstance(logical_id, str) or not logical_id.strip():
            raise ValueError("query module IDs must be nonblank")
        module_path(path)
        result[logical_id] = path
    return result


def write_staged_profile(
    source: Path,
    destination: Path,
    registry: dict[str, str],
    parser_configuration: str = "python-target=3.10",
    rule_applicability: dict[str, str] | None = None,
) -> None:
    """Serialize the supported profile fields, replacing any existing module table."""
    parsed = tomllib.loads(source.read_text(encoding="utf-8"))
    current = profile_modules(parsed)
    registry = profile_modules({"query-modules": registry})
    if any(key in registry and registry[key] != value for key, value in current.items()):
        raise ValueError("staged profile conflicts with its query module registry")
    if parser_configuration not in {"python-target=3.10", "python-target=3.11"}:
        raise ValueError(f"unsupported staged parser configuration: {parser_configuration}")
    merged = {**current, **registry}
    applicability = rule_applicability or {}
    if set(applicability.values()) - {"always", "accepted-syntax"}:
        raise ValueError("staged rule applicability contains an unsupported value")
    parsed.pop("query-modules", None)
    parsed.pop("rule-applicability", None)
    parsed["parser-configuration"] = parser_configuration
    if merged:
        parsed["query-modules"] = merged
    if applicability:
        parsed["rule-applicability"] = applicability
    rendered = "".join(f"{key} = {toml_value(value)}\n" for key, value in parsed.items())
    destination.write_text(rendered, encoding="utf-8")


def parser_configuration(target_version: object) -> str:
    """Resolve only target versions whose Ruff and Python parser behavior is supported."""
    configurations = {
        "py310": "python-target=3.10",
        "py311": "python-target=3.11",
    }
    if not isinstance(target_version, str) or target_version not in configurations:
        raise ValueError(f"unsupported prepared Python target version: {target_version}")
    return configurations[target_version]


def copy_modules(registry: dict[str, str], destination: Path) -> None:
    for source_path in sorted(set(registry.values())):
        source = RUFF_DIR / source_path
        target = destination / source_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)


def combined_modules(
    profile_data: dict[str, object], selected_rules: list[str], manifest: dict[str, object]
) -> dict[str, str]:
    registry = profile_modules(profile_data)
    sources = [
        tomllib.loads((RUFF_DIR / "benchmark.toml").read_text(encoding="utf-8")),
    ]
    sources.extend(
        tomllib.loads((PROFILE_DIR / rule / "benchmark.toml").read_text(encoding="utf-8"))
        for rule in selected_rules
        if rule in manifest["profiles"]
    )
    for source_data in sources:
        for logical_id, source_path in profile_modules(source_data).items():
            if logical_id in registry and registry[logical_id] != source_path:
                raise ValueError(f"combined profile conflicts with module source for {logical_id}")
            registry[logical_id] = source_path
    return registry


def verify_files(manifest: dict[str, object], helper_path: str) -> None:
    files = manifest["files"]
    if manifest.get("target_version") != "py310":
        raise ValueError("prepared Ruff target version must be py310")
    verify_rule_phase_registry(manifest)
    allowed_targets = {"py310", "py311"}
    if any(
        profile.get("target_version", "py310") not in allowed_targets
        for profile in manifest["profiles"].values()
    ):
        raise ValueError("prepared single-rule profile has an unsupported target version")
    if any(
        profile.get("target_version", "py310") not in allowed_targets
        for profile in manifest.get("combined_profiles", {}).values()
    ):
        raise ValueError("prepared combined profile has an unsupported target version")
    if set(files) != manifest_paths(manifest, helper_path):
        raise ValueError("prepared input paths do not match the candidate acceptance cohorts")
    defaults = set(tomllib.loads((RUFF_DIR / "benchmark.toml").read_text(encoding="utf-8"))["defaults"])
    profile_rules = {
        profile.get("rule_id", profile_name)
        for profile_name, profile in manifest["profiles"].items()
    }
    if profile_rules - defaults:
        raise ValueError("prepared profile contains a non-default rule")
    for descriptor in manifest.get("combined_profiles", {}).values():
        selected = descriptor["selected_rules"]
        if len(selected) != len(set(selected)) or set(selected) - defaults:
            raise ValueError("combined profile contains a non-default rule")
        if descriptor.get("validation_status") not in {"active-current", "archived-historical"}:
            raise ValueError("combined profile must declare active-current or archived-historical status")
    if files.get(helper_path) != sha256(Path(__file__)):
        raise ValueError("profile helper hash does not match prepared.json")
    for relative, expected in files.items():
        path = REPOSITORY / relative
        actual = sha256(path) if path.is_file() else None
        if actual != expected:
            raise ValueError(
                f"prepared input hash mismatch: {relative} expected={expected} actual={actual} manifest={MANIFEST}"
            )


def verify_rule_phase_registry(
    manifest: dict[str, object], registry: dict[str, object] | None = None
) -> dict[str, object]:
    """Require a pinned, positive-dispatch phase for every current prepared rule."""
    if manifest.get("reference_version") != PINNED_RUFF_VERSION:
        raise ValueError("rule phase registry is only valid for pinned Ruff 0.16.5")
    if registry is None:
        registry = json.loads(RULE_PHASES_FILE.read_text(encoding="utf-8"))
    if (
        registry.get("schema_version") != 1
        or registry.get("ruff_version") != PINNED_RUFF_VERSION
        or registry.get("ruff_source_commit") != PINNED_RUFF_SOURCE_COMMIT
        or not isinstance(registry.get("code_to_variant_registry"), dict)
        or not isinstance(registry.get("phase_registry"), dict)
        or manifest.get("rule_phases_file")
        != RULE_PHASES_FILE.relative_to(REPOSITORY).as_posix()
    ):
        raise ValueError("pinned Ruff phase registry has an unsupported identity")
    expected_rules = {
        profile.get("rule_id", profile_name)
        for profile_name, profile in manifest["profiles"].items()
    }
    expected_rules.update(
        rule
        for descriptor in manifest.get("combined_profiles", {}).values()
        if descriptor.get("validation_status") == "active-current"
        for rule in descriptor["selected_rules"]
    )
    rules = registry.get("rules")
    shipped_defaults = registry.get("shipped_default_rules")
    if (
        not isinstance(shipped_defaults, list)
        or len(shipped_defaults) != len(set(shipped_defaults))
        or not isinstance(rules, dict)
        or set(rules) != set(shipped_defaults)
        or not expected_rules <= set(rules)
    ):
        missing = expected_rules - set(rules or {})
        extra = set(rules or {}) - set(shipped_defaults or [])
        raise ValueError(
            f"pinned Ruff phase coverage mismatch: missing={sorted(missing)} extra={sorted(extra)}"
        )
    for rule, record in rules.items():
        if not isinstance(record, dict):
            raise ValueError(f"invalid phase classification for {rule}")
        phases = record.get("phases")
        if (
            not isinstance(phases, list)
            or not phases
            or any(not isinstance(phase, str) for phase in phases)
            or len(phases) != len(set(phases))
            or set(phases) - RUFF_PHASES
        ):
            raise ValueError(f"unknown or empty Ruff phase classification for {rule}")
        evidence = record.get("evidence")
        if (
            not isinstance(evidence, list)
            or len(evidence) != 2
            or any(
                not isinstance(item, dict)
                or item.get("kind") not in {"code-to-variant", "lint-source"}
                or not isinstance(item.get("line"), int)
                or item["line"] <= 0
                or not isinstance(item.get("sha256"), str)
                or len(item["sha256"]) != 64
                or any(char not in "0123456789abcdef" for char in item["sha256"])
                or item.get("path")
                != (
                    registry.get("code_to_variant_registry", {}).get("path")
                    if item.get("kind") == "code-to-variant"
                    else registry.get("phase_registry", {}).get("path")
                )
                for item in evidence
            )
            or not isinstance(record.get("registry_variant"), str)
            or not isinstance(record.get("registry_line"), int)
            or record["registry_line"] <= 0
        ):
            raise ValueError(f"phase classification for {rule} lacks pinned dispatch evidence")
        evidence_by_kind = {item["kind"]: item for item in evidence}
        code_registry = registry["code_to_variant_registry"]
        phase_registry = registry["phase_registry"]
        if (
            set(evidence_by_kind) != {"code-to-variant", "lint-source"}
            or evidence_by_kind["code-to-variant"]["sha256"] != code_registry.get("sha256")
            or evidence_by_kind["code-to-variant"]["line"] != record["registry_line"]
            or evidence_by_kind["lint-source"]["sha256"] != phase_registry.get("sha256")
            or not isinstance(phase_registry.get("ast_fallback_line"), int)
        ):
            raise ValueError(f"phase evidence does not match the pinned registry for {rule}")
        phase_evidence = evidence_by_kind["lint-source"]
        resolution = phase_evidence.get("resolution")
        lint_sources = registry.get("lint_sources", {})
        if phases == ["ast"]:
            if (
                resolution != "Ast wildcard fallback"
                or phase_evidence["line"] != phase_registry["ast_fallback_line"]
            ):
                raise ValueError(f"AST classification lacks the positive pinned fallback for {rule}")
        elif (
            len(phases) != 1
            or lint_sources.get(resolution) != phases[0]
            or phase_evidence["line"] == phase_registry["ast_fallback_line"]
        ):
            if not record.get("mixed_policy"):
                raise ValueError(f"non-AST or mixed phase lacks explicit registry evidence for {rule}")
        applicability = record.get("applicability")
        if set(phases) <= {"ast", "imports"}:
            expected_applicability = "accepted-syntax"
        elif set(phases) <= RUFF_PHASES - {"ast", "imports"}:
            expected_applicability = "always"
        else:
            # A rule dispatched in both parser-dependent and independent lanes needs a
            # reviewed explicit policy; this version of the inventory has no such rules.
            if applicability not in {"always", "accepted-syntax"} or not record.get("mixed_policy"):
                raise ValueError(f"mixed Ruff phase policy is not explicitly reviewed for {rule}")
            expected_applicability = applicability
        if applicability != expected_applicability:
            raise ValueError(
                f"Ruff phase/applicability mismatch for {rule}: {phases} -> {applicability}"
            )
    return registry


def verify_active_combined_cohort(manifest: dict[str, object]) -> None:
    """Require the default acceptance run to include one current full-report cohort."""
    active = [
        (name, descriptor)
        for name, descriptor in manifest.get("combined_profiles", {}).items()
        if descriptor.get("validation_status") == "active-current"
    ]
    if len(active) != 1:
        raise ValueError(
            f"default acceptance requires exactly one active combined cohort; found {len(active)}"
        )
    name, descriptor = active[0]
    if descriptor.get("expectations_mode") != "complete-full-report":
        raise ValueError(f"active combined cohort {name} is not a complete full-report profile")


def selected_rule_applicability(
    manifest: dict[str, object], selected_rules: list[str]
) -> dict[str, str]:
    """Project the pinned phase registry onto this exact selected rule inventory."""
    registry = verify_rule_phase_registry(manifest)
    return {
        rule: registry["rules"][rule]["applicability"]
        for rule in selected_rules
    }


def check_staged_applicability(profile: Path, selected_rules: list[str], manifest: dict[str, object]) -> None:
    """Refuse a staged profile whose typed policy differs from its sealed selection."""
    actual = tomllib.loads(profile.read_text(encoding="utf-8")).get("rule-applicability")
    expected = selected_rule_applicability(manifest, selected_rules)
    if actual != expected:
        raise ValueError("staged rule applicability differs from the pinned phase registry")


def profile_tool_matches(profile_data: dict[str, object], manifest: dict[str, object]) -> bool:
    return (
        profile_data.get("tool") == "ruff"
        and profile_data.get("version") == manifest["reference_version"]
        and profile_data.get("sha256") == manifest["reference_sha256"]
    )


def command_for(
    worker: Path,
    ruff: Path,
    profile_root: Path,
    reference_only: bool = False,
    shard: tuple[int, int] | None = None,
    case_selection: Path | None = None,
) -> list[str]:
    """Invoke the one immutable benchmark executable built for this profile batch."""
    command = [
        str(worker),
        "verify-reference-cases" if reference_only else "verify-cases",
        str(ruff),
        str(profile_root),
    ]
    if shard is not None:
        command.append(f"--shard={shard[0]}/{shard[1]}")
    if case_selection is not None:
        command.append(f"--case-selection={case_selection}")
    return command


def profile_case_inventory(profile_root: Path) -> list[dict[str, object]]:
    """Derive canonical case identities after the staged profile is sealed."""
    inventory = []
    for rule_root in sorted((profile_root / "rules").iterdir(), key=lambda path: path.name):
        cases_path = rule_root / "cases.toml"
        if not cases_path.is_file():
            continue
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        for case in cases:
            identity = {"rule": rule_root.name, "name": case["name"]}
            encoded = json.dumps(case, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
            source_digest = hashlib.sha256()
            for file, source in sorted(case["files"].items()):
                source_digest.update(file.encode("utf-8"))
                source_digest.update(b"\0")
                source_digest.update(source.encode("utf-8"))
            expected = sorted(
                (
                    {
                        "rule": finding.get("rule", rule_root.name),
                        "file": finding["file"],
                        "start": finding["start"],
                        "end": finding["end"],
                    }
                    for finding in case.get("expected", [])
                ),
                key=lambda row: (row["rule"], row["file"], row["start"], row["end"]),
            )
            inventory.append({
                **identity,
                "case_sha256": hashlib.sha256(encoded.encode()).hexdigest(),
                "source_sha256": source_digest.hexdigest(),
                "files": sorted(case["files"]),
                "comparison": case.get("comparison", "rule"),
                "expected": expected,
            })
    inventory.sort(key=lambda case: (case["rule"], case["name"]))
    ids = [(case["rule"], case["name"]) for case in inventory]
    if not ids or len(ids) != len(set(ids)):
        raise ValueError("staged case inventory is empty or has duplicate (rule, name) identities")
    return inventory


def complete_case_ids(
    selected_cases: list[dict[str, object]], records: list[dict[str, object]]
) -> set[str]:
    """Return cases with one valid, durable full report from each contestant."""
    expected_cases = {f"{case['rule']}/{case['name']}": case for case in selected_cases}
    grouped: dict[str, dict[str, dict[str, object]]] = {}
    duplicates = set()
    for row in records:
        case_id = f"{row.get('rule')}/{row.get('name')}"
        side = row.get("side")
        if case_id not in expected_cases or side not in {"reference", "candidate"}:
            continue
        bucket = grouped.setdefault(case_id, {})
        if side in bucket:
            duplicates.add(case_id)
            continue
        bucket[side] = row
    completed = set()
    for case_id, sides in grouped.items():
        expected_case = expected_cases[case_id]
        if case_id in duplicates or set(sides) != {"reference", "candidate"}:
            continue
        if all(
            row.get("complete") is True
            and row.get("error") is None
            and row.get("source_sha256") == expected_case["source_sha256"]
            and row.get("files") == expected_case["files"]
            and row.get("expected") == expected_case["expected"]
            and isinstance(row.get("case_passed"), bool)
            for row in sides.values()
        ):
            completed.add(case_id)
    return completed


def partition_case_inventory(
    inventory: list[dict[str, object]], shard_count: int
) -> list[list[dict[str, object]]]:
    """Assign each sorted global case exactly once by index modulo shard count."""
    if shard_count < 1 or shard_count > len(inventory):
        raise ValueError("invalid shard count for case inventory")
    return [
        [case for index, case in enumerate(inventory) if index % shard_count == shard]
        for shard in range(shard_count)
    ]


def valid_shard_receipts(
    inventory: list[dict[str, object]],
    receipts: list[dict[str, object]],
    shard_count: int,
    global_identity: dict[str, object],
) -> bool:
    """Reject gaps, overlaps, failed cases, and cross-run identity mixing."""
    if len(receipts) != shard_count:
        return False
    partitions = partition_case_inventory(inventory, shard_count)
    by_index = {receipt.get("shard_index"): receipt for receipt in receipts}
    if set(by_index) != set(range(shard_count)):
        return False
    all_ids = []
    for index, expected_cases in enumerate(partitions):
        receipt = by_index[index]
        expected_ids = [f"{case['rule']}/{case['name']}" for case in expected_cases]
        process_proven = (
            receipt.get("stdout_summary_valid") is True
            and receipt.get("last_process_status") == 0
            and receipt.get("reconstructed_from_case_records") is not True
        )
        reconstructed = (
            receipt.get("reconstructed_from_case_records") is True
            and receipt.get("stdout_summary_valid") is None
            and receipt.get("last_process_status") is None
            and isinstance(receipt.get("reconstruction"), dict)
            and receipt["reconstruction"].get("method") == "complete-case-records-no-process-summary"
            and receipt["reconstruction"].get("records_sha256") == receipt.get("case_results_sha256")
        )
        if (
            receipt.get("case_ids") != expected_ids
            or receipt.get("global_identity") != global_identity
            or receipt.get("shard_count") != shard_count
            or receipt.get("status") != 0
            or receipt.get("case_result_records_valid") is not True
            or not (process_proven or reconstructed)
        ):
            return False
        all_ids.extend(receipt["case_ids"])
    expected_global = [f"{case['rule']}/{case['name']}" for case in inventory]
    return len(all_ids) == len(set(all_ids)) and sorted(all_ids) == sorted(expected_global)


def case_observation_map(paths: list[Path]) -> dict[tuple[str, str, str], dict[str, object]]:
    """Load complete per-case contestant records and reject duplicate observations."""
    observations = {}
    for path in paths:
        for line in path.read_text(encoding="utf-8").splitlines():
            row = json.loads(line)
            key = (row["rule"], row["name"], row["side"])
            if key in observations:
                raise ValueError(f"duplicate case observation: {key}")
            if row.get("complete") is not True:
                raise ValueError(f"incomplete case observation: {key}")
            observations[key] = row
    return observations


def case_observations_match(monolithic: Path, shards: list[Path]) -> bool:
    """Require exact full-report observations for each side and case across modes."""
    try:
        return case_observation_map([monolithic]) == case_observation_map(shards)
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return False


def candidate_result_valid(
    profile_root: Path,
    stdout: bytes,
    case_results_path: Path,
) -> tuple[bool, str | None]:
    """Require the real worker summary and exactly two complete contestant records per case."""
    try:
        inventory = profile_case_inventory(profile_root)
        records = [
            json.loads(line)
            for line in case_results_path.read_text(encoding="utf-8").splitlines()
        ]
        valid_ids = complete_case_ids(inventory, records)
        expected_ids = {f"{case['rule']}/{case['name']}" for case in inventory}
        side_counts = collections.Counter(
            (row.get("rule"), row.get("name"), row.get("side")) for row in records
        )
        output = stdout.decode("utf-8")
    except (OSError, UnicodeDecodeError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        return False, f"candidate proof records could not be read: {error}"
    expected_summary = f"{len(inventory)} cases passed against both contestants\n"
    if output != expected_summary:
        return False, f"worker summary mismatch: expected {expected_summary!r}, got {output!r}"
    if valid_ids != expected_ids or len(records) != 2 * len(inventory):
        return False, (
            f"worker case receipts incomplete: expected {len(expected_ids)} exact case IDs, "
            f"observed {len(valid_ids)}"
        )
    if any(
        side_counts[(case["rule"], case["name"], side)] != 1
        for case in inventory
        for side in ("reference", "candidate")
    ):
        return False, "worker case receipts contain missing or duplicate contestant records"
    if any(row.get("case_passed") is not True for row in records):
        return False, "worker case receipts include a failed contestant comparison"
    return True, None


def audit_worker_protocol(worker: Path, *, runner=subprocess.run) -> tuple[bool, str]:
    """Confirm the executable is the benchmark CLI, not a same-named Rust test harness."""
    command = [str(worker), "audit"]
    try:
        result = runner(command, cwd=REPOSITORY / "crates" / "ryo", capture_output=True, check=False)
    except Exception as error:
        return False, f"worker protocol smoke could not start: {type(error).__name__}: {error}"
    if result.returncode != 0:
        return False, f"worker protocol smoke exited {result.returncode}"
    expected = re.fullmatch(
        rb"ruff 0\.16\.5: [0-9]+ queries authored / 413 shipped defaults "
        rb"\(not a parity certificate\)\n",
        result.stdout,
    )
    if expected is None:
        return False, f"worker protocol smoke returned an unexpected response: {result.stdout!r}"
    return True, result.stdout.decode("utf-8").strip()


def shard_worker_limit() -> int:
    """Bound verifier concurrency using available memory and logical CPU count."""
    cpu_count = os.cpu_count() or 1
    available_bytes = None
    try:
        pages = os.sysconf("SC_AVPHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        available_bytes = pages * page_size
    except (OSError, ValueError):
        pass
    if available_bytes is None:
        return max(1, cpu_count // 2)
    memory_slots = max(1, (available_bytes - 4 * 1024**3) // (3 * 1024**3))
    return max(1, min(max(1, cpu_count // 2), int(memory_slots)))


def verify_profile_sharded(
    name: str,
    profile_root: Path,
    worker: Path,
    ruff: Path,
    output_directory: Path,
    identities: dict[str, object],
    shard_count: int,
    *,
    resume: bool = False,
    runner=subprocess.run,
) -> int:
    """Run each audited canonical case once and persist an aggregatable receipt."""
    inventory = profile_case_inventory(profile_root)
    if not inventory:
        raise ValueError("cannot shard an empty canonical case inventory")
    effective_shard_count = min(shard_count, len(inventory))
    inventory_bytes = json.dumps(inventory, sort_keys=True, separators=(",", ":")).encode()
    inventory_sha = hashlib.sha256(inventory_bytes).hexdigest()
    profile_output = output_directory / "profiles" / name
    shard_root = profile_output / "shards"
    shard_root.mkdir(parents=True, exist_ok=resume)
    global_identity = {
        "inventory_sha256": inventory_sha,
        "selection": identities,
        "worker_sha256": sha256(worker),
        "ruff_sha256": sha256(ruff),
        "rule_phase_registry_sha256": identities.get(
            "rule_phase_registry_sha256", sha256(RULE_PHASES_FILE)
        ),
        "selected_rule_applicability_sha256": source_hash(
            identities.get("selected_rule_applicability", {})
        ),
        "target": tomllib.loads((profile_root / "benchmark.toml").read_text(encoding="utf-8")).get("parser-configuration"),
        "requested_shard_count": shard_count,
        "effective_shard_count": effective_shard_count,
    }
    shards = list(enumerate(partition_case_inventory(inventory, effective_shard_count)))

    def run_one(item: tuple[int, list[dict[str, object]], list[dict[str, object]], bool]) -> dict[str, object]:
        shard_index, selected, to_run, earlier_case_failed = item
        shard_name = f"{shard_index:03d}"
        result_path = shard_root / f"{shard_name}.cases.jsonl"
        selection_path = None
        if len(to_run) != len(selected):
            selection_path = shard_root / f"{shard_name}.selection.json"
            atomic_json_write(selection_path, [f"{case['rule']}/{case['name']}" for case in to_run])
        command = command_for(
            worker,
            ruff,
            profile_root,
            shard=(shard_index, effective_shard_count),
            case_selection=selection_path,
        )
        environment = os.environ.copy()
        environment["RYO_VERIFY_CASE_RESULTS"] = str(result_path)
        try:
            result = runner(
                command,
                cwd=REPOSITORY / "crates" / "ryo",
                env=environment,
                capture_output=True,
                text=False,
                check=False,
            )
            error = None
            status = result.returncode
            stdout = result.stdout
            stderr = result.stderr
        except Exception as exception:
            result = None
            error = f"{type(exception).__name__}: {exception}"
            status = 1
            stdout = b""
            stderr = error.encode()
        stdout_path = shard_root / f"{shard_name}.stdout.bin"
        stderr_path = shard_root / f"{shard_name}.stderr.bin"
        stdout_path.write_bytes(stdout)
        stderr_path.write_bytes(stderr)
        expected_ids = [f"{case['rule']}/{case['name']}" for case in selected]
        observed = []
        if result_path.is_file():
            try:
                for line in result_path.read_text(encoding="utf-8").splitlines():
                    observed.append(json.loads(line))
            except json.JSONDecodeError:
                observed = []
        selected_ids = {f"{case['rule']}/{case['name']}" for case in selected}
        observed_ids = complete_case_ids(selected, observed)
        side_counts = collections.Counter((row.get("rule"), row.get("name"), row.get("side")) for row in observed)
        valid_records = (
            observed_ids == selected_ids
            and all(side_counts[(case["rule"], case["name"], side)] == 1 for case in selected for side in ("reference", "candidate"))
            and len(observed) == 2 * len(selected)
        )
        completed_failed = any(not row.get("case_passed", False) for row in observed)
        # A resumed shard may execute only the unfinished subset while its
        # receipt still proves the complete original shard inventory.
        expected_summary = f"{len(to_run)} cases passed against both contestants\n".encode()
        summary_valid = stdout == expected_summary
        final_status = (
            0
            if status == 0
            and not earlier_case_failed
            and not completed_failed
            and valid_records
            and summary_valid
            else 1
        )
        receipt = {
            "profile": name,
            "shard_index": shard_index,
            "shard_count": effective_shard_count,
            "case_ids": expected_ids,
            "status": final_status,
            "last_process_status": status,
            "error": error,
            "global_identity": global_identity,
            "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
            "case_results_sha256": sha256(result_path) if result_path.is_file() else None,
            "case_result_records_valid": valid_records,
            "stdout_summary_valid": summary_valid,
            "executed_case_ids": [f"{case['rule']}/{case['name']}" for case in to_run],
            "case_selection_file": (
                selection_path.name if selection_path is not None else None
            ),
            "case_selection_sha256": (
                sha256(selection_path) if selection_path is not None else None
            ),
        }
        atomic_json_write(shard_root / f"{shard_name}.receipt.json", receipt)
        return receipt

    prior_aggregate_path = profile_output / "aggregate.json"
    if resume:
        plan_path = profile_output / "shard-plan.json"
        if not plan_path.is_file():
            raise ValueError("cannot resume: prior shard plan is missing")
        prior_plan = json.loads(plan_path.read_text(encoding="utf-8"))
        if prior_plan.get("global_identity") != global_identity:
            raise ValueError("cannot resume with changed global inventory/worker/reference/target identity")
        if prior_plan.get("staged_inputs") != staged_hashes(profile_root):
            raise ValueError("cannot resume with changed staged profile inputs")
    else:
        atomic_json_write(
            profile_output / "shard-plan.json",
            {
                "profile": name,
                "global_identity": global_identity,
                "staged_inputs": staged_hashes(profile_root),
                "case_ids": [f"{case['rule']}/{case['name']}" for case in inventory],
                "shard_count": effective_shard_count,
                "requested_shard_count": shard_count,
            },
        )
    receipts = []
    pending_shards = []
    for shard_index, selected in shards:
        shard_name = f"{shard_index:03d}"
        receipt_path = shard_root / f"{shard_name}.receipt.json"
        records_path = shard_root / f"{shard_name}.cases.jsonl"
        expected_ids = [f"{case['rule']}/{case['name']}" for case in selected]
        reused = False
        if resume and receipt_path.is_file() and records_path.is_file():
            try:
                receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                records = [json.loads(line) for line in records_path.read_text(encoding="utf-8").splitlines()]
                counts = collections.Counter((row.get("rule"), row.get("name"), row.get("side")) for row in records)
                records_valid = (
                    complete_case_ids(selected, records)
                    == {f"{case['rule']}/{case['name']}" for case in selected}
                    and all(counts[(case["rule"], case["name"], side)] == 1 for case in selected for side in ("reference", "candidate"))
                    and len(records) == 2 * len(selected)
                )
                reused = (
                    receipt.get("global_identity") == global_identity
                    and receipt.get("case_ids") == expected_ids
                    and receipt.get("case_result_records_valid") is True
                    and receipt.get("case_results_sha256") == sha256(records_path)
                    and records_valid
                )
                if reused:
                    receipts.append(receipt)
            except (OSError, ValueError, json.JSONDecodeError):
                reused = False
        if not reused:
            if not resume and (receipt_path.exists() or records_path.exists()):
                raise ValueError("shard output already exists; use --resume with matching identities")
            prior_records = []
            malformed = False
            if resume and records_path.is_file():
                try:
                    prior_records = [json.loads(line) for line in records_path.read_text(encoding="utf-8").splitlines()]
                except (OSError, json.JSONDecodeError):
                    malformed = True
            completed = set() if malformed else complete_case_ids(selected, prior_records)
            completed_ids = {f"{case['rule']}/{case['name']}" for case in selected}
            keep = [row for row in prior_records if f"{row.get('rule')}/{row.get('name')}" in completed]
            if malformed:
                keep = []
            temporary_records = records_path.with_name(f".{records_path.name}.tmp")
            with temporary_records.open("w", encoding="utf-8") as output_records:
                for row in keep:
                    output_records.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
                output_records.flush()
                os.fsync(output_records.fileno())
            temporary_records.replace(records_path)
            to_run = [case for case in selected if f"{case['rule']}/{case['name']}" not in completed]
            if not to_run:
                failed = any(not row.get("case_passed", False) for row in keep)
                records_valid = (
                    len(keep) == 2 * len(selected)
                    and completed == completed_ids
                    and all(row.get("complete") is True and row.get("error") is None for row in keep)
                )
                receipt = {
                    "profile": name,
                    "shard_index": shard_index,
                    "shard_count": effective_shard_count,
                    "case_ids": expected_ids,
                    "executed_case_ids": [],
                    "status": int(failed or not records_valid),
                    "last_process_status": None,
                    "error": None,
                    "global_identity": global_identity,
                    "stdout_sha256": None,
                    "stderr_sha256": None,
                    "case_results_sha256": sha256(records_path),
                    "case_result_records_valid": records_valid,
                    "stdout_summary_valid": None,
                    "reconstructed_from_case_records": True,
                    "reconstruction": {
                        "method": "complete-case-records-no-process-summary",
                        "records_sha256": sha256(records_path),
                        "process_exit_observed": False,
                        "stdout_summary_observed": False,
                    },
                }
                atomic_json_write(receipt_path, receipt)
                receipts.append(receipt)
                continue
            for path in (receipt_path, shard_root / f"{shard_name}.stdout.bin", shard_root / f"{shard_name}.stderr.bin"):
                path.unlink(missing_ok=True)
            earlier_failed = any(not row.get("case_passed", False) for row in keep)
            pending_shards.append((shard_index, selected, to_run, earlier_failed))
    with ThreadPoolExecutor(max_workers=min(effective_shard_count, shard_worker_limit())) as pool:
        futures = [pool.submit(run_one, shard) for shard in pending_shards]
        for future in as_completed(futures):
            receipts.append(future.result())
    receipts.sort(key=lambda receipt: receipt["shard_index"])
    flattened = [case_id for receipt in receipts for case_id in receipt["case_ids"]]
    receipts_valid = valid_shard_receipts(inventory, receipts, effective_shard_count, global_identity)
    aggregate = {
        "profile": name,
        "global_identity": global_identity,
        "staged_inputs": staged_hashes(profile_root),
        "case_count": len(inventory),
        "shard_count": effective_shard_count,
        "requested_shard_count": shard_count,
        "receipts": [f"shards/{index:03d}.receipt.json" for index in range(effective_shard_count)],
        "all_shards_passed": receipts_valid,
        "case_ids_exact_once": len(flattened) == len(set(flattened))
        and sorted(flattened) == sorted(f"{case['rule']}/{case['name']}" for case in inventory),
    }
    aggregate["status"] = 0 if aggregate["all_shards_passed"] and aggregate["case_ids_exact_once"] else 1
    atomic_json_write(profile_output / "aggregate.json", aggregate)
    print(
        f"{name}: {aggregate['case_count']} cases across {effective_shard_count} of {shard_count} requested shards; status {aggregate['status']}",
        file=sys.stderr,
    )
    return int(aggregate["status"])


def cargo_worker_build_command() -> list[str]:
    """Build the benchmark once and request machine-readable artifact paths."""
    return [
        "cargo",
        "bench",
        "--no-run",
        "--locked",
        "-p",
        "idiomatic-sh-ryo-python",
        "--bench",
        "ruff",
        "--message-format=json",
    ]


def parse_worker_artifact(stdout: bytes) -> Path:
    """Read Cargo JSON output and return the ruff bench executable."""
    matches = []
    for line in stdout.splitlines():
        try:
            message = json.loads(line)
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        target = message.get("target", {})
        source_path = target.get("src_path", "")
        normalized_source = source_path.replace("\\", "/") if isinstance(source_path, str) else ""
        if (
            message.get("reason") == "compiler-artifact"
            and target.get("name") == "ruff"
            and "bench" in target.get("kind", [])
            and target.get("test") is False
            and normalized_source.endswith("/languages/python/benchmarks/linters/ruff/bench.rs")
            and isinstance(message.get("executable"), str)
        ):
            matches.append(Path(message["executable"]))
    if len(matches) != 1:
        raise ValueError(f"Cargo JSON reported {len(matches)} ruff benchmark executables")
    return matches[0].resolve(strict=True)


def worker_source_identity() -> dict[str, object]:
    """Capture the Ryo source identity needed to detect edits during the build."""
    try:
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPOSITORY, capture_output=True, check=True
        ).stdout.decode().strip()
        diff = subprocess.run(
            ["git", "diff", "--binary", "HEAD", "--", "crates/ryo"],
            cwd=REPOSITORY,
            capture_output=True,
            check=True,
        ).stdout
        status = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all", "--", "crates/ryo"],
            cwd=REPOSITORY,
            capture_output=True,
            check=True,
        ).stdout.decode().splitlines()
        source_paths = subprocess.run(
            [
                "git",
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                "--",
                "crates/ryo",
            ],
            cwd=REPOSITORY,
            capture_output=True,
            check=True,
        ).stdout.split(b"\0")
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError(f"could not identify Ryo source before worker build: {error}") from error
    source_digest = hashlib.sha256()
    for raw_path in sorted(path for path in source_paths if path):
        relative = Path(os.fsdecode(raw_path))
        path = REPOSITORY / relative
        if path.is_symlink():
            content = os.fsencode(os.readlink(path))
        elif path.is_file():
            content = path.read_bytes()
        else:
            continue
        source_digest.update(len(raw_path).to_bytes(8, "big"))
        source_digest.update(raw_path)
        source_digest.update(len(content).to_bytes(8, "big"))
        source_digest.update(content)
    return {
        "repository_revision": revision,
        "working_tree_status": status,
        "working_tree_diff_sha256": hashlib.sha256(diff).hexdigest(),
        "source_tree_sha256": source_digest.hexdigest(),
    }


def build_worker_once(
    output_directory: Path | None,
    stack: contextlib.ExitStack,
    *,
    runner=subprocess.run,
) -> tuple[Path, dict[str, object]]:
    """Build, identify and freeze one executable before any profile is run."""
    command = cargo_worker_build_command()
    before = worker_source_identity()
    environment = {**os.environ, "CARGO_BUILD_JOBS": "6"}
    error = None
    result = None
    try:
        result = runner(
            command,
            cwd=REPOSITORY / "crates" / "ryo",
            env=environment,
            capture_output=True,
            text=False,
            check=False,
        )
        if result.returncode != 0:
            raise ValueError(f"Cargo benchmark build exited with status {result.returncode}")
        artifact = parse_worker_artifact(result.stdout)
        after = worker_source_identity()
        if before != after:
            raise ValueError("Ryo sources changed while building the benchmark executable")
        if output_directory is None:
            worker_directory = Path(
                stack.enter_context(tempfile.TemporaryDirectory(prefix="ryo-ruff-worker-"))
            )
        else:
            worker_directory = output_directory / "worker"
            worker_directory.mkdir(parents=True, exist_ok=False)
        worker_copy = worker_directory / f"ruff-{sha256(artifact)}"
        shutil.copy2(artifact, worker_copy)
        identities = {
            "command": command,
            "status": result.returncode,
            "source_before_build": before,
            "source_after_build": after,
            "source_stable_during_build": True,
            "cargo_artifact": str(artifact),
            "frozen_executable": {"path": str(worker_copy.resolve()), "sha256": sha256(worker_copy)},
        }
    except Exception as caught:
        error = f"{type(caught).__name__}: {caught}"
        after = worker_source_identity()
        identities = {
            "command": command,
            "status": result.returncode if result is not None else 1,
            "source_before_build": before,
            "source_after_build": after,
            "source_stable_during_build": before == after,
            "error": error,
        }
    if output_directory is not None:
        build_directory = output_directory / "build"
        build_directory.mkdir(exist_ok=True)
        (build_directory / "stdout.bin").write_bytes(result.stdout if result is not None else b"")
        (build_directory / "stderr.bin").write_bytes(result.stderr if result is not None else b"")
        (build_directory / "result.json").write_text(
            json.dumps(identities, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    if error is not None:
        raise ValueError(error)
    return worker_copy, identities


def reuse_frozen_worker(output_directory: Path) -> tuple[Path, dict[str, object]]:
    """Resume with the executable whose stable build provenance was saved."""
    record_path = output_directory / "build" / "result.json"
    record = json.loads(record_path.read_text(encoding="utf-8"))
    frozen = record.get("frozen_executable")
    if (
        record.get("status") != 0
        or record.get("source_stable_during_build") is not True
        or record.get("source_before_build") != record.get("source_after_build")
        or not isinstance(frozen, dict)
        or not isinstance(frozen.get("path"), str)
        or not isinstance(frozen.get("sha256"), str)
    ):
        raise ValueError("cannot resume: frozen worker build provenance is invalid")
    executable = Path(frozen["path"])
    if not executable.is_file() or sha256(executable) != frozen["sha256"]:
        raise ValueError("cannot resume: frozen worker executable is missing or changed")
    build_stdout = output_directory / "build" / "stdout.bin"
    try:
        artifact = parse_worker_artifact(build_stdout.read_bytes())
    except (OSError, ValueError) as error:
        raise ValueError(f"cannot resume: saved Cargo artifact record is invalid: {error}") from error
    if (
        record.get("cargo_artifact") != str(artifact)
        or record.get("command") != cargo_worker_build_command()
    ):
        raise ValueError("cannot resume: saved Cargo artifact is not the Ruff benchmark target")
    return executable, record


def resume_manifest(output_directory: Path, profile_name: str) -> dict[str, object]:
    """Load the immutable manifest snapshot bound to a prior shard plan."""
    prepared_path = output_directory / "prepared.json"
    plan_path = output_directory / "profiles" / profile_name / "shard-plan.json"
    try:
        manifest_bytes = prepared_path.read_bytes()
        manifest = json.loads(manifest_bytes)
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot resume: saved manifest or shard plan is unreadable: {error}") from error
    selection = plan.get("global_identity", {}).get("selection", {})
    if (
        plan.get("profile") != profile_name
        or selection.get("manifest_sha256") != hashlib.sha256(manifest_bytes).hexdigest()
        or profile_name not in manifest.get("combined_profiles", {})
        or manifest["combined_profiles"][profile_name].get("validation_status") != "active-current"
        or selection.get("sealed_source_hashes") != manifest.get("files")
    ):
        raise ValueError("cannot resume: saved manifest does not match the sealed active profile plan")
    repro = output_directory / "profiles" / profile_name / "repro"
    if not repro.is_dir() or plan.get("staged_inputs") != staged_hashes(repro):
        raise ValueError("cannot resume: staged profile inputs differ from the saved shard plan")
    profile_data = tomllib.loads((repro / "benchmark.toml").read_text(encoding="utf-8"))
    if profile_data.get("defaults") != manifest["combined_profiles"][profile_name].get("selected_rules"):
        raise ValueError("cannot resume: profile rule selection differs from the saved manifest")
    if selection.get("target_version") != profile_data.get("parser-configuration", "python-target=3.10"):
        raise ValueError("cannot resume: parser target differs from the saved run identity")
    if selection.get("selected_rule_applicability") != profile_data.get("rule-applicability", {}):
        raise ValueError("cannot resume: rule applicability differs from the saved run identity")
    return manifest


def validate_output_directory(output_directory: Path, *, resume: bool) -> None:
    """Keep new verifier receipts outside the Rust source identity being built."""
    rust_root = (REPOSITORY / "crates" / "ryo").resolve()
    resolved = output_directory.resolve()
    if not resume and resolved.is_relative_to(rust_root):
        raise ValueError("new candidate output directories must be outside crates/ryo")


def stage_directory(
    name: str, stack: contextlib.ExitStack, output_directory: Path | None
) -> Path:
    """Use a durable repro directory when requested, otherwise a temporary one."""
    if output_directory is not None:
        destination = output_directory / "profiles" / name / "repro"
        destination.mkdir(parents=True, exist_ok=False)
        return destination
    return Path(stack.enter_context(tempfile.TemporaryDirectory(prefix=f"ryo-ruff-{name}-")))


def staged_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): sha256(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def persist_run_record(
    output_directory: Path | None,
    name: str,
    command: list[str],
    profile_root: Path,
    result: subprocess.CompletedProcess[bytes] | None,
    *,
    error: str | None = None,
    identities: dict[str, object] | None = None,
    case_results_path: Path | None = None,
    status_override: int | None = None,
) -> int:
    """Persist raw command output and exact identities before temporary inputs can be removed."""
    process_status = result.returncode if result is not None else 1
    status = status_override if status_override is not None else process_status
    if output_directory is None:
        if result is not None:
            sys.stderr.buffer.write(result.stderr)
            sys.stdout.buffer.write(result.stdout)
        if error is not None:
            print(f"{name}: could not start verifier: {error}", file=sys.stderr)
        return status

    record_directory = output_directory / "profiles" / name
    record_directory.mkdir(parents=True, exist_ok=True)
    (record_directory / "stdout.bin").write_bytes(result.stdout if result is not None else b"")
    (record_directory / "stderr.bin").write_bytes(result.stderr if result is not None else b"")
    record = {
        "profile": name,
        "command": command,
        "cwd": str(REPOSITORY / "crates" / "ryo"),
        "status": status,
        "process_status": process_status,
        "error": error,
        "identities": identities or {},
        "staged_inputs": staged_hashes(profile_root) if profile_root.is_dir() else {},
        "stdout_file": f"profiles/{name}/stdout.bin",
        "stderr_file": f"profiles/{name}/stderr.bin",
        "reproduction_directory": f"profiles/{name}/repro",
        "case_results_file": (
            f"profiles/{name}/cases.jsonl" if case_results_path is not None and case_results_path.is_file() else None
        ),
        "case_results_sha256": sha256(case_results_path) if case_results_path is not None and case_results_path.is_file() else None,
    }
    (record_directory / "result.json").write_text(
        json.dumps(record, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return status


def verify_profile(
    name: str,
    profile_root: Path,
    command: list[str],
    output_directory: Path | None,
    identities: dict[str, object],
    *,
    runner=subprocess.run,
) -> int:
    case_results_path = None
    environment = os.environ.copy()
    if output_directory is not None and command[1] != "verify-reference-cases":
        case_results_path = output_directory / "profiles" / name / "cases.jsonl"
        environment["RYO_VERIFY_CASE_RESULTS"] = str(case_results_path)
    try:
        result = runner(
            command,
            cwd=REPOSITORY / "crates" / "ryo",
            capture_output=True,
            text=False,
            check=False,
            env=environment,
        )
    except Exception as error:
        return persist_run_record(
            output_directory,
            name,
            command,
            profile_root,
            None,
            error=f"{type(error).__name__}: {error}",
            identities=identities,
            case_results_path=case_results_path,
        )
    validation_error = None
    status_override = None
    if command[1] == "verify-cases":
        if output_directory is None or case_results_path is None:
            validation_error = "candidate verification requires durable output and exact case receipts"
            status_override = 1
        elif result.returncode == 0:
            valid, validation_error = candidate_result_valid(
                profile_root, result.stdout, case_results_path
            )
            if not valid:
                status_override = 1
    return persist_run_record(
        output_directory,
        name,
        command,
        profile_root,
        result,
        identities=identities,
        case_results_path=case_results_path,
        error=validation_error,
        status_override=status_override,
    )


def ruff_location_to_byte(source: str, location: dict[str, object]) -> int:
    """Convert Ruff's one-based Unicode row/column to the wire's UTF-8 byte offset."""
    row = location.get("row")
    column = location.get("column")
    if not isinstance(row, int) or not isinstance(column, int) or row < 1 or column < 1:
        raise ValueError("Ruff returned an invalid source location")
    separators = list(re.finditer(r"\r\n|\r|\n", source))
    starts = [0, *(match.end() for match in separators)]
    if row > len(starts):
        if row == len(starts) + 1 and column == 1:
            return len(source.encode("utf-8"))
        raise ValueError("Ruff returned a row beyond the staged source")
    start = starts[row - 1]
    end = separators[row - 1].start() if row <= len(separators) else len(source)
    line = source[start:end]
    prefix = line[: column - 1]
    if len(prefix) != column - 1:
        raise ValueError("Ruff returned a column beyond the staged source")
    return len(source[:start].encode("utf-8")) + len(prefix.encode("utf-8"))


def verify_reference_profile(
    name: str,
    profile_root: Path,
    ruff: Path,
    output_directory: Path | None,
    identities: dict[str, object],
    *,
    runner=subprocess.run,
) -> int:
    """Check pinned full-report fixtures directly with Ruff, without building Ryo."""
    profile = tomllib.loads((profile_root / "benchmark.toml").read_text(encoding="utf-8"))
    target_config = profile.get("parser-configuration", "python-target=3.10")
    match = re.fullmatch(r"python-target=(3)\.(10|11)", target_config)
    if match is None:
        raise ValueError(f"unsupported reference-only parser configuration: {target_config}")
    target = f"py{match.group(1)}{match.group(2)}"
    rules = profile.get("defaults")
    if not isinstance(rules, list) or not rules or any(not isinstance(rule, str) for rule in rules):
        raise ValueError("reference-only profile must have a nonempty defaults list")
    cases_path = profile_root / "rules" / rules[0] / "cases.toml"
    cases = tomllib.loads(cases_path.read_text(encoding="utf-8")).get("cases")
    if not isinstance(cases, list) or not cases:
        raise ValueError("reference-only profile has no canonical cases")
    rows = []
    failures = []
    command_log = []
    for case in cases:
        actual = []
        process_records = []
        for file, source in sorted(case["files"].items()):
            command = [
                str(ruff), "check", "--isolated", "--no-cache", "--output-format", "json",
                "--select", ",".join(rules), "--target-version", target,
                "--stdin-filename", file, "-",
            ]
            command_log.append(command)
            try:
                result = runner(
                    command,
                    cwd=REPOSITORY / "crates" / "ryo",
                    input=source.encode("utf-8"),
                    capture_output=True,
                    check=False,
                )
                if result.returncode not in {0, 1}:
                    raise ValueError(f"Ruff exited {result.returncode}: {result.stderr!r}")
                findings = json.loads(result.stdout)
                if not isinstance(findings, list):
                    raise ValueError("Ruff JSON response was not an array")
                for finding in findings:
                    start = ruff_location_to_byte(source, finding["location"])
                    end_location = finding.get("end_location")
                    if not isinstance(end_location, dict):
                        raise ValueError("Ruff finding is missing end_location")
                    end = ruff_location_to_byte(source, end_location)
                    if start > end:
                        raise ValueError("Ruff returned a reversed source span")
                    actual.append({"rule": finding["code"], "file": file, "start": start, "end": end})
                process_records.append({
                    "file": file,
                    "status": result.returncode,
                    "stdout": result.stdout.decode("utf-8"),
                    "stderr": result.stderr.decode("utf-8"),
                })
            except (OSError, UnicodeDecodeError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
                failures.append(f"{case['name']}/{file}: {error}")
                process_records.append({"file": file, "error": f"{type(error).__name__}: {error}"})
        actual.sort(key=lambda row: (row["rule"], row["file"], row["start"], row["end"]))
        expected = sorted(
            ({"rule": item.get("rule", rules[0]), "file": item["file"], "start": item["start"], "end": item["end"]}
             for item in case.get("expected", [])),
            key=lambda row: (row["rule"], row["file"], row["start"], row["end"]),
        )
        case_passed = actual == expected
        if not case_passed:
            failures.append(f"{case['name']}: expected {expected!r}; observed {actual!r}")
        rows.append({
            "rule": rules[0], "name": case["name"], "side": "reference",
            "complete": not any(record.get("error") for record in process_records),
            "case_passed": case_passed, "expected": expected, "actual": actual,
            "source_sha256": source_hash(case["files"]),
            "files": sorted(case["files"]), "commands": process_records,
        })
    summary = f"{len(cases)} cases matched the reference; no candidate was executed\n"
    status = 1 if failures else 0
    result = subprocess.CompletedProcess(
        ["ruff-reference-only", str(ruff), str(profile_root)],
        status,
        summary.encode("utf-8"),
        ("\n".join(failures) + ("\n" if failures else "")).encode("utf-8"),
    )
    case_results_path = None
    if output_directory is not None:
        case_results_path = output_directory / "profiles" / name / "reference-cases.jsonl"
        case_results_path.write_text("\n".join(json.dumps(row, ensure_ascii=False) for row in rows) + "\n", encoding="utf-8")
    return persist_run_record(
        output_directory, name, command_log, profile_root, result,
        identities={**identities, "reference_only": True, "candidate_worker_built": False},
        case_results_path=case_results_path,
    )


def reference_tool_identity(ruff: Path, manifest: dict[str, object]) -> dict[str, object]:
    """Require the executable itself to match a pinned Ruff wheel and version."""
    executable = ruff.resolve()
    if sha256(executable) not in manifest.get("reference_sha256", []):
        raise ValueError("reference-only executable hash is not one of the pinned Ruff wheels")
    version = subprocess.run([str(executable), "--version"], capture_output=True, check=False)
    if version.returncode != 0 or version.stderr:
        raise ValueError("reference-only executable did not return a clean version response")
    expected = f"ruff {manifest['reference_version']}\n".encode()
    if version.stdout != expected:
        raise ValueError(f"reference-only executable version mismatch: {version.stdout!r}")
    return {"path": str(executable), "sha256": sha256(executable), "version": manifest["reference_version"]}


def worker_identity(identities: dict[str, object], executable: Path) -> None:
    """Record and verify the immutable executable used for this profile."""
    worker = identities.setdefault("worker", {})
    if not isinstance(worker, dict):
        return
    worker["executable"] = {"path": str(executable.resolve()), "sha256": sha256(executable)}


def run_identity(
    manifest: dict[str, object],
    ruff: Path,
    profile_root: Path,
    worker_executable: Path,
    build_identity: dict[str, object],
) -> dict[str, object]:
    """Describe the sealed inputs, candidate worker source, and pinned reference binary."""
    try:
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPOSITORY, capture_output=True, check=True
        ).stdout.decode().strip()
    except (OSError, subprocess.CalledProcessError):
        revision = None
    try:
        worker_diff = subprocess.run(
            ["git", "diff", "--binary", "HEAD", "--", "crates/ryo"],
            cwd=REPOSITORY,
            capture_output=True,
            check=True,
        ).stdout
        worker_status = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all", "--", "crates/ryo"],
            cwd=REPOSITORY,
            capture_output=True,
            check=True,
        ).stdout.decode().splitlines()
    except (OSError, subprocess.CalledProcessError):
        worker_diff = b""
        worker_status = None
    current_source = worker_source_identity()
    lock = REPOSITORY / "crates" / "ryo" / "Cargo.lock"
    resolved_ruff = ruff.resolve()
    cargo = shutil.which("cargo")
    try:
        cargo_version = subprocess.run(
            [cargo, "--version"], capture_output=True, check=True
        ).stdout.decode().strip() if cargo else None
        rustc_version = subprocess.run(
            ["rustc", "--version", "--verbose"], capture_output=True, check=True
        ).stdout.decode().strip()
    except (OSError, subprocess.CalledProcessError):
        cargo_version = None
        rustc_version = None
    return {
        "manifest_path": MANIFEST.relative_to(REPOSITORY).as_posix(),
        "manifest_sha256": sha256(MANIFEST),
        "manifest_status": manifest.get("status"),
        "rule_phase_registry_sha256": sha256(RULE_PHASES_FILE),
        "selected_rule_applicability": tomllib.loads(
            (profile_root / "benchmark.toml").read_text(encoding="utf-8")
        ).get("rule-applicability", {}),
        "target_version": tomllib.loads(
            (profile_root / "benchmark.toml").read_text(encoding="utf-8")
        ).get("parser-configuration", "python-target=3.10"),
        "manifest_source_commits": manifest.get("source_commits"),
        "sealed_source_hashes": manifest["files"],
        "worker": {
            "repository_revision": revision,
            "working_tree_status": worker_status,
            "working_tree_diff_sha256": hashlib.sha256(worker_diff).hexdigest(),
            "cargo_lock_sha256": sha256(lock) if lock.is_file() else None,
            "cargo_bench_package": "idiomatic-sh-ryo-python",
            "cargo_path": cargo,
            "cargo_version": cargo_version,
            "rustc_version": rustc_version,
            "build": build_identity,
            "source_status_changed_after_build": (
                not isinstance(build_identity.get("source_after_build"), dict)
                or any(
                    build_identity["source_after_build"].get(key) != current
                    for key, current in (
                        ("repository_revision", revision),
                        ("working_tree_status", worker_status),
                        ("working_tree_diff_sha256", hashlib.sha256(worker_diff).hexdigest()),
                        (
                            "source_tree_sha256",
                            current_source.get("source_tree_sha256"),
                        ),
                    )
                )
            ),
            "staged_profile_sha256": staged_hashes(profile_root),
        },
        "ruff": {
            "path": str(resolved_ruff),
            "sha256": sha256(resolved_ruff),
            "version": manifest.get("reference_version"),
            "sealed_sha256": manifest.get("reference_sha256"),
        },
    }


def stage_binding(
    manifest: dict[str, object],
    selected: list[str],
    stack: contextlib.ExitStack,
    output_directory: Path | None = None,
) -> list[tuple[str, Path]]:
    staged = []
    profiles = manifest["profiles"]
    # Validate and materialize every selected profile before any command can run.
    for profile_name in selected:
        recorded = profiles[profile_name]
        rule = recorded.get("rule_id", profile_name)
        profile = prepared_profile_path(profile_name, recorded)
        profile_data = tomllib.loads(profile.read_text(encoding="utf-8"))
        if profile_data.get("defaults") != [rule]:
            raise ValueError(f"profile must select only {rule}")
        if not profile_tool_matches(profile_data, manifest):
            raise ValueError(f"tool identity mismatch for {rule}")
        candidate_path = prepared_candidate_path(rule, recorded)
        cases_path = prepared_case_path(rule, recorded)
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        names = [case["name"] for case in cases]
        if (
            recorded.get("selected_rules") != [rule]
            or names != recorded["case_names"]
            or len(cases) != recorded["case_count"]
        ):
            raise ValueError(f"case inventory mismatch for {rule}")
        if "oracle_file" in recorded:
            oracle = REPOSITORY / recorded["oracle_file"]
            if not oracle.is_file() or sha256(oracle) != recorded["oracle_sha256"]:
                raise ValueError(f"oracle evidence mismatch for {rule}")
        if "reference_audit_file" in recorded:
            audit = REPOSITORY / recorded["reference_audit_file"]
            if not audit.is_file() or sha256(audit) != recorded["reference_audit_sha256"]:
                raise ValueError(f"reference case audit mismatch for {rule}")
        destination = stage_directory(profile_name, stack, output_directory)
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        registry = combined_modules(profile_data, [rule], manifest)
        copy_modules(registry, destination)
        target_version = recorded.get("target_version", "py310")
        write_staged_profile(
            profile,
            destination / "benchmark.toml",
            registry,
            parser_configuration(target_version),
            selected_rule_applicability(manifest, [rule]),
        )
        check_staged_applicability(destination / "benchmark.toml", [rule], manifest)
        shutil.copyfile(candidate_path, rule_dir / "query.ryoql")
        shutil.copyfile(cases_path, rule_dir / "cases.toml")
        staged.append((profile_name, destination))
    return staged


def stage_combined(
    profile_name: str,
    descriptor: dict[str, object],
    manifest: dict[str, object],
    stack: contextlib.ExitStack,
    output_directory: Path | None = None,
) -> tuple[str, Path]:
    profile_path = REPOSITORY / descriptor["profile_file"]
    profile_data = tomllib.loads(profile_path.read_text(encoding="utf-8"))
    selected_rules = descriptor["selected_rules"]
    if profile_data.get("defaults") != selected_rules or len(selected_rules) != len(set(selected_rules)):
        raise ValueError(f"{profile_name} selection does not match its prepared descriptor")
    if not profile_tool_matches(profile_data, manifest):
        raise ValueError(f"tool identity mismatch for {profile_name}")
    if sha256(profile_path) != descriptor["benchmark_sha256"]:
        raise ValueError(f"{profile_name} benchmark hash mismatch")

    overlays_path = REPOSITORY / descriptor["expectations_file"]
    if sha256(overlays_path) != descriptor["expectations_sha256"]:
        raise ValueError(f"{profile_name} expectation overlay hash mismatch")
    overlay = json.loads(overlays_path.read_text(encoding="utf-8"))
    oracle_path = REPOSITORY / descriptor["oracle_file"]
    oracle = json.loads(oracle_path.read_text(encoding="utf-8"))
    full_report = descriptor.get("expectations_mode") == "complete-full-report"
    if (
        overlay.get("profile") != profile_name
        or overlay.get("mode", "shape-overlay")
        != descriptor.get("expectations_mode", "shape-overlay")
        or overlay.get("reference_sha256") not in manifest["reference_sha256"]
        or overlay.get("selection_count") != len(selected_rules)
        or overlay.get("selected_rules_sha256")
        != hashlib.sha256(json.dumps(sorted(selected_rules), separators=(",", ":")).encode("utf-8")).hexdigest()
        or overlay.get("oracle_sha256") != descriptor["oracle_sha256"]
        or overlay.get("oracle_file") != descriptor.get("oracle_file")
    ):
        raise ValueError(f"{profile_name} expectation overlay identity mismatch")

    overlays = {}
    added_findings = 0
    expected_findings = 0
    for item in overlay["cases"]:
        key = (item["rule"], item["case"])
        if key in overlays or item["rule"] not in selected_rules:
            raise ValueError(f"duplicate or unselected {profile_name} overlay case: {key}")
        cases_path = combined_case_path(item["rule"], descriptor)
        canonical_case_label = (RUFF_DIR / "rules" / item["rule"] / "cases.toml").relative_to(REPOSITORY).as_posix()
        if item["cases_file"] != canonical_case_label:
            raise ValueError(f"{profile_name} overlay names a noncanonical case identity: {key}")
        if sha256(cases_path) != item["canonical_cases_sha256"]:
            raise ValueError(f"{profile_name} canonical case hash mismatch: {item['cases_file']}")
        source_cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        matching = [case for case in source_cases if case["name"] == item["case"]]
        if len(matching) != 1:
            raise ValueError(f"{profile_name} overlay case is absent or duplicated: {key}")
        case = matching[0]
        if not full_report and case.get("comparison", "rule") != "full":
            raise ValueError(f"{profile_name} overlay requires a full-report case: {key}")
        if source_hash(case["files"]) != item["case_source_sha256"]:
            raise ValueError(f"{profile_name} case source hash mismatch: {key}")
        findings = item["expected"]
        for finding in findings:
            if (
                not isinstance(finding.get("rule"), str)
                or not isinstance(finding.get("file"), str)
                or not isinstance(finding.get("start"), int)
                or not isinstance(finding.get("end"), int)
                or finding["start"] < 0
                or finding["end"] < finding["start"]
                or finding["file"] not in case["files"]
                or (finding["rule"] not in selected_rules and finding["rule"] != "invalid-syntax")
            ):
                raise ValueError(f"invalid {profile_name} expected finding: {key}")
        if case.get("comparison", "rule") == "full" and any("rule" not in entry for entry in case["expected"]):
            raise ValueError(f"{profile_name} canonical full-report finding lacks an explicit rule: {key}")
        original = diagnostic_multiset(
            [
                {**entry, "rule": entry.get("rule", item["rule"])}
                for entry in case["expected"]
            ]
        )
        replacement = diagnostic_multiset(findings)
        if original - replacement:
            raise ValueError(f"{profile_name} overlay drops canonical findings: {key}")
        if not full_report:
            added_findings += sum((replacement - original).values())
        expected_findings += len(findings)
        overlays[key] = (item, case)
    if full_report:
        if len(overlays) != descriptor["case_count"] or len(overlays) != overlay["case_count"]:
            raise ValueError(f"{profile_name} complete expected inventory count mismatch")
    elif len(overlays) != overlay["changed_case_count"] or added_findings != overlay["additional_diagnostic_count"]:
        raise ValueError(f"{profile_name} overlay diagnostic totals mismatch")
    if expected_findings != overlay["expected_diagnostic_count"]:
        raise ValueError(f"{profile_name} overlay diagnostic totals mismatch")

    oracle_cases: dict[tuple[str, str], dict[str, object]] = {}
    for oracle_case in oracle["cases"]:
        key = (oracle_case["rule"], oracle_case.get("case", oracle_case.get("name")))
        if key in oracle_cases:
            raise ValueError(f"{profile_name} oracle has duplicate case identity: {key}")
        oracle_cases[key] = oracle_case
    if full_report and len(oracle_cases) != descriptor["case_count"]:
        raise ValueError(f"{profile_name} oracle case count mismatch")
    for key, (item, _case) in overlays.items():
        oracle_case = oracle_cases.get(key)
        if (
            oracle_case is None
            or oracle_case["case_source_sha256"] != item["case_source_sha256"]
            or diagnostic_multiset(item["expected"]) != diagnostic_multiset(oracle_case["expected"])
        ):
            raise ValueError(f"{profile_name} overlay diagnostics differ from the sealed oracle: {key}")

    inventory = []
    staged_cases: dict[str, list[dict[str, object]]] = {}
    for rule in selected_rules:
        query = combined_query_path(rule, descriptor)
        cases_path = combined_case_path(rule, descriptor)
        if not query.is_file() or not cases_path.is_file():
            raise ValueError(f"{profile_name} source is missing for {rule}")
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        staged_cases[rule] = cases
        for case in cases:
            comparison = "full" if full_report else case.get("comparison", "rule")
            inventory.append({"rule": rule, "name": case["name"], "comparison": comparison})
    counts = (
        collections.Counter({"full": len(inventory)})
        if full_report
        else collections.Counter(
            item.get("comparison", "rule")
            for cases in staged_cases.values()
            for item in cases
        )
    )
    if len(inventory) != descriptor["case_count"] or counts != descriptor["comparison_counts"]:
        raise ValueError(f"{profile_name} comparison inventory mismatch")
    if full_report and set(overlays) != {(item["rule"], item["name"]) for item in inventory}:
        raise ValueError(f"{profile_name} complete expected inventory does not match selected cases")
    inventory_hash = hashlib.sha256(
        json.dumps(inventory, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if inventory_hash != descriptor["inventory_sha256"]:
        raise ValueError(f"{profile_name} inventory hash mismatch")

    destination = stage_directory(profile_name, stack, output_directory)
    registry = combined_modules(profile_data, selected_rules, manifest)
    copy_modules(registry, destination)
    target_version = descriptor.get("target_version", "py310")
    write_staged_profile(
        profile_path,
        destination / "benchmark.toml",
        registry,
        parser_configuration(target_version),
        selected_rule_applicability(manifest, selected_rules),
    )
    check_staged_applicability(destination / "benchmark.toml", selected_rules, manifest)
    for rule in selected_rules:
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        query_path = combined_query_path(rule, descriptor)
        shutil.copyfile(query_path, rule_dir / "query.ryoql")
        cases = staged_cases[rule]
        for case in cases:
            overlay_entry = overlays.get((rule, case["name"]))
            if overlay_entry is not None:
                case["expected"] = overlay_entry[0]["expected"]
            elif full_report:
                raise ValueError(f"{profile_name} is missing complete expected case: {rule}/{case['name']}")
            if full_report:
                case["comparison"] = "full"
        write_cases(rule_dir / "cases.toml", cases)
    return profile_name, destination


def combined_input_path(value: str, descriptor: dict[str, object]) -> Path:
    """Resolve a combined input only when its exact path is sealed and contained."""
    if value not in descriptor["input_files"]:
        raise ValueError(f"combined input is not sealed: {value}")
    relative = Path(value)
    if relative.is_absolute() or any(part in (".", "..") for part in relative.parts):
        raise ValueError(f"unsafe combined input path: {value}")
    resolved = (REPOSITORY / relative).resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(RUFF_DIR.resolve()):
        raise ValueError(f"combined input is outside Ruff profile: {value}")
    cursor = RUFF_DIR
    for part in relative.relative_to(RUFF_DIR.relative_to(REPOSITORY)).parts:
        cursor = cursor / part
        if cursor.is_symlink():
            raise ValueError(f"combined input traverses a symlink: {value}")
    return resolved


def combined_query_filename(rule: str, descriptor: dict[str, object]) -> str:
    """Return the sealed source basename for canonical combined queries."""
    return combined_filename(rule, descriptor, "candidate.ryoql", "query.ryoql")


def combined_query_path(rule: str, descriptor: dict[str, object]) -> Path:
    paths = descriptor.get("query_files", {})
    if rule in paths:
        return combined_input_path(paths[rule], descriptor)
    source_dir = RUFF_DIR / "rules" / rule
    return source_dir / combined_filename(rule, descriptor, "candidate.ryoql", "query.ryoql")


def combined_case_path(rule: str, descriptor: dict[str, object]) -> Path:
    paths = descriptor.get("case_files", {})
    if rule in paths:
        return combined_input_path(paths[rule], descriptor)
    source_dir = RUFF_DIR / "rules" / rule
    return source_dir / combined_filename(rule, descriptor, "cases.toml")


def combined_filename(rule: str, descriptor: dict[str, object], *names: str) -> str:
    """Select one per-rule artifact filename from a combined cohort's sealed inputs."""
    inputs = set(descriptor["input_files"])
    source_dir = f"crates/ryo/languages/python/benchmarks/linters/ruff/rules/{rule}"
    candidates = [name for name in names if f"{source_dir}/{name}" in inputs]
    if len(candidates) != 1:
        raise ValueError(f"combined profile must seal exactly one {names} source for {rule}")
    return candidates[0]


def self_test() -> int:
    """Exercise fail-closed profile-selection and hash-mismatch guards without invoking Cargo."""
    global MANIFEST
    if (
        ruff_location_to_byte("é\fvalue\r\nx", {"row": 1, "column": 8}) != 8
        or ruff_location_to_byte("é\fvalue\r\nx", {"row": 2, "column": 2}) != 11
    ):
        print("Ruff coordinate conversion must count physical CR/LF lines and UTF-8 bytes", file=sys.stderr)
        return 1
    sample_inventory = [
        {"rule": "A", "name": "one", "case_sha256": "a"},
        {"rule": "A", "name": "two", "case_sha256": "b"},
        {"rule": "B", "name": "one", "case_sha256": "c"},
        {"rule": "C", "name": "one", "case_sha256": "d"},
        {"rule": "C", "name": "two", "case_sha256": "e"},
    ]
    prepared = json.loads(MANIFEST.read_text(encoding="utf-8"))
    try:
        no_active_cohort = json.loads(json.dumps(prepared))
        for descriptor in no_active_cohort.get("combined_profiles", {}).values():
            descriptor["validation_status"] = "archived-historical"
        try:
            verify_active_combined_cohort(no_active_cohort)
        except ValueError as error:
            if "exactly one active combined cohort" not in str(error):
                raise
        else:
            raise ValueError("default acceptance allowed a missing active combined cohort")
        ambiguous_cohorts = json.loads(json.dumps(prepared))
        for descriptor in ambiguous_cohorts.get("combined_profiles", {}).values():
            descriptor["validation_status"] = "active-current"
        try:
            verify_active_combined_cohort(ambiguous_cohorts)
        except ValueError as error:
            if "exactly one active combined cohort" not in str(error):
                raise
        else:
            raise ValueError("default acceptance allowed multiple active combined cohorts")
        phase_registry = verify_rule_phase_registry(prepared)
        if selected_rule_applicability(
            prepared, prepared["combined_profiles"]["current-canonical"]["selected_rules"]
        ) != {
            rule: phase_registry["rules"][rule]["applicability"]
            for rule in prepared["combined_profiles"]["current-canonical"]["selected_rules"]
        }:
            raise ValueError("applicability projection changed the selected rule inventory")
        missing_phase = json.loads(json.dumps(phase_registry))
        missing_phase["rules"].pop(next(iter(missing_phase["rules"])))
        unknown_phase = json.loads(json.dumps(phase_registry))
        unknown_rule = next(iter(unknown_phase["rules"].values()))
        unknown_rule["phases"] = ["future-checker"]
        mixed_phase = json.loads(json.dumps(phase_registry))
        mixed_rule = next(iter(mixed_phase["rules"].values()))
        mixed_rule["phases"] = ["ast", "tokens"]
        mixed_rule["evidence"].append(
            {"phase": "tokens", "path": "checkers/tokens.rs", "sha256": "0" * 64}
        )
        for invalid in (missing_phase, unknown_phase, mixed_phase):
            try:
                verify_rule_phase_registry(prepared, invalid)
            except ValueError:
                continue
            raise ValueError("phase validation accepted missing, unknown, or unreviewed mixed data")
        if phase_registry["rules"]["EXE005"]["applicability"] != "always":
            raise ValueError("token-only Ruff rule lost independent applicability")
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"pinned Ruff phase registry validation failed: {error}", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-resume-manifest-") as temporary:
        resume_root = Path(temporary)
        profile_name = "sealed-active"
        resume_manifest_data = {
            "files": {},
            "combined_profiles": {
                profile_name: {
                    "validation_status": "active-current",
                    "expectations_mode": "complete-full-report",
                    "selected_rules": ["A"],
                }
            },
        }
        prepared_path = resume_root / "prepared.json"
        manifest_bytes = json.dumps(resume_manifest_data, separators=(",", ":")).encode()
        prepared_path.write_bytes(manifest_bytes)
        repro = resume_root / "profiles" / profile_name / "repro"
        repro.mkdir(parents=True)
        (repro / "benchmark.toml").write_text(
            'defaults = ["A"]\nparser-configuration = "python-target=3.10"\n'
            'rule-applicability = {}\n',
            encoding="utf-8",
        )
        staged = staged_hashes(repro)
        selection = {
            "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "sealed_source_hashes": {},
            "staged_profile_sha256": staged,
            "target_version": "python-target=3.10",
            "selected_rule_applicability": {},
        }
        plan_root = resume_root / "profiles" / profile_name
        plan_root.mkdir(exist_ok=True)
        (plan_root / "shard-plan.json").write_text(
            json.dumps({
                "profile": profile_name,
                "staged_inputs": staged,
                "global_identity": {"selection": selection},
            }),
            encoding="utf-8",
        )
        try:
            if resume_manifest(resume_root, profile_name) != resume_manifest_data:
                raise ValueError("resume did not use its saved manifest snapshot")
            # Replacing the mutable current manifest must not affect this run's snapshot.
            original_manifest_path = MANIFEST
            MANIFEST = resume_root / "mutable-current.json"
            MANIFEST.write_text('{"different":true}', encoding="utf-8")
            try:
                if resume_manifest(resume_root, profile_name) != resume_manifest_data:
                    raise ValueError("resume followed the mutable current manifest")
            finally:
                MANIFEST = original_manifest_path
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f"sealed resume manifest check failed: {error}", file=sys.stderr)
            return 1
    partitions = partition_case_inventory(sample_inventory, 3)
    identity = {"inventory_sha256": "sealed"}
    receipts = [
        {
            "shard_index": index,
            "shard_count": 3,
            "case_ids": [f"{case['rule']}/{case['name']}" for case in partition],
            "status": 0,
            "last_process_status": 0,
            "case_result_records_valid": True,
            "stdout_summary_valid": True,
            "global_identity": identity,
        }
        for index, partition in enumerate(partitions)
    ]
    if not valid_shard_receipts(sample_inventory, receipts, 3, identity):
        print("valid deterministic shard receipts were rejected", file=sys.stderr)
        return 1
    overlapping = json.loads(json.dumps(receipts))
    overlapping[1]["case_ids"].append(overlapping[0]["case_ids"][0])
    incomplete = json.loads(json.dumps(receipts))
    incomplete.pop()
    mixed_worker = json.loads(json.dumps(receipts))
    mixed_worker[2]["global_identity"]["worker_sha256"] = "different"
    if any(
        valid_shard_receipts(sample_inventory, altered, 3, identity)
        for altered in (overlapping, incomplete, mixed_worker)
    ):
        print("shard aggregation accepted overlap, missing shard, or mixed worker", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-shard-test-") as temporary:
        root = Path(temporary)
        profile_root = root / "repro"
        rules_root = profile_root / "rules"
        for rule, names in (("A", ("one", "two")), ("B", ("one",))):
            rule_root = rules_root / rule
            rule_root.mkdir(parents=True)
            cases = [{"name": name, "files": {"case.py": f"value = {index}\n"}, "expected": []} for index, name in enumerate(names)]
            (rule_root / "cases.toml").write_text(
                "\n".join(f'[[cases]]\nname = "{case["name"]}"\nfiles = {{ "case.py" = {json.dumps(case["files"]["case.py"])} }}\nexpected = []\n' for case in cases),
                encoding="utf-8",
            )
        (profile_root / "benchmark.toml").write_text('parser-configuration = "python-target=3.10"\n', encoding="utf-8")
        worker = root / "worker"
        ruff = root / "ruff"
        worker.write_bytes(b"worker")
        ruff.write_bytes(b"ruff")
        output = root / "output"
        output.mkdir()
        inv = profile_case_inventory(profile_root)
        case_results = root / "candidate-cases.jsonl"
        if candidate_result_valid(
            profile_root,
            f"{len(inv)} cases passed against both contestants\n".encode(),
            case_results,
        )[0]:
            raise ValueError("zero-exit worker without case records was accepted")
        rows = []
        for case in inv:
            for side in ("reference", "candidate"):
                rows.append({
                    "rule": case["rule"],
                    "name": case["name"],
                    "side": side,
                    "complete": True,
                    "error": None,
                    "case_passed": True,
                    "source_sha256": case["source_sha256"],
                    "files": case["files"],
                    "expected": case["expected"],
                })
        case_results.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
        valid_receipt, _reason = candidate_result_valid(
            profile_root,
            f"{len(inv)} cases passed against both contestants\n".encode(),
            case_results,
        )
        if not valid_receipt:
            raise ValueError("exact worker case receipts were rejected")
        false_audit = lambda command, **_kwargs: subprocess.CompletedProcess(command, 0, b"running 0 tests\n", b"")
        valid_audit = lambda command, **_kwargs: subprocess.CompletedProcess(
            command,
            0,
            b"ruff 0.16.5: 104 queries authored / 413 shipped defaults (not a parity certificate)\n",
            b"",
        )
        if audit_worker_protocol(worker, runner=false_audit)[0]:
            raise ValueError("same-named zero-test harness passed the CLI protocol smoke")
        if not audit_worker_protocol(worker, runner=valid_audit)[0]:
            raise ValueError("valid benchmark CLI audit response was rejected")
        calls = []

        def fake_runner(command, *, env, **_kwargs):
            shard_arg = next(value for value in command if value.startswith("--shard="))
            shard_index, shard_count = (int(value) for value in shard_arg.removeprefix("--shard=").split("/"))
            selected = partition_case_inventory(inv, shard_count)[shard_index]
            selection_arg = next((value for value in command if value.startswith("--case-selection=")), None)
            if selection_arg is not None:
                selected_ids = set(json.loads(Path(selection_arg.split("=", 1)[1]).read_text(encoding="utf-8")))
                selected = [case for case in selected if f"{case['rule']}/{case['name']}" in selected_ids]
            calls.append(shard_index)
            records_path = Path(env["RYO_VERIFY_CASE_RESULTS"])
            with records_path.open("a" if records_path.exists() else "w", encoding="utf-8") as records:
                for case in selected:
                    for side in ("reference", "candidate"):
                        records.write(json.dumps({
                            "rule": case["rule"], "name": case["name"], "side": side,
                            "complete": True, "error": None, "case_passed": True,
                            "diagnostics": [], "expected": case["expected"],
                            "source_sha256": case["source_sha256"], "files": case["files"],
                        }) + "\n")
            summary = f"{len(selected)} cases passed against both contestants\n".encode()
            return subprocess.CompletedProcess(command, 0, summary, b"")

        identities = {"prepared_sha256": "sealed"}
        first = verify_profile_sharded("test", profile_root, worker, ruff, output, identities, 2, runner=fake_runner)
        shards_dir = output / "profiles" / "test" / "shards"
        monolithic = output / "profiles" / "test" / "monolithic.jsonl"
        monolithic.write_bytes(b"".join((shards_dir / f"{index:03d}.cases.jsonl").read_bytes() for index in range(2)))
        if not case_observations_match(monolithic, sorted(shards_dir.glob("*.cases.jsonl"))):
            print("monolithic/sharded full-report comparison failed", file=sys.stderr)
            return 1
        # Model a crash after every case row is atomically appended but before
        # the shard receipt rename. Recovery may prove comparisons from those
        # complete rows, but must not invent a process exit or stdout summary.
        lost_receipt = shards_dir / "000.receipt.json"
        lost_receipt.unlink()
        recovered = verify_profile_sharded(
            "test", profile_root, worker, ruff, output, identities, 2, resume=True,
            runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(
                AssertionError("complete case rows should be reconstructed without rerunning")
            ),
        )
        recovered_receipt = json.loads(lost_receipt.read_text(encoding="utf-8"))
        if (
            recovered != 0
            or recovered_receipt.get("reconstructed_from_case_records") is not True
            or recovered_receipt.get("stdout_summary_valid") is not None
            or recovered_receipt.get("last_process_status") is not None
            or recovered_receipt.get("reconstruction", {}).get("process_exit_observed") is not False
        ):
            print("full-record/missing-receipt recovery fabricated process proof", file=sys.stderr)
            return 1
        second = verify_profile_sharded(
            "test", profile_root, worker, ruff, output, identities, 2, resume=True,
            runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("complete shard was rerun")),
        )
        interrupted_index = 0
        interrupted_name = f"{interrupted_index:03d}"
        interrupted_path = shards_dir / f"{interrupted_name}.cases.jsonl"
        interrupted_rows = [json.loads(line) for line in interrupted_path.read_text(encoding="utf-8").splitlines()]
        one_case = f"{partitions[interrupted_index][0]['rule']}/{partitions[interrupted_index][0]['name']}"
        interrupted_path.write_text(
            "\n".join(json.dumps(row) for row in interrupted_rows if f"{row['rule']}/{row['name']}" == one_case) + "\n",
            encoding="utf-8",
        )
        (shards_dir / f"{interrupted_name}.receipt.json").unlink()
        third = verify_profile_sharded(
            "test", profile_root, worker, ruff, output, identities, 2, resume=True, runner=fake_runner
        )
        torn = shards_dir / "000.cases.jsonl"
        with torn.open("a", encoding="utf-8") as output_records:
            output_records.write("{incomplete json")
        fourth = verify_profile_sharded(
            "test", profile_root, worker, ruff, output, identities, 2, resume=True, runner=fake_runner
        )
        if first != 0 or recovered != 0 or second != 0 or third != 0 or fourth != 0 or calls != [0, 1, 0, 0]:
            print("shard execution or identity-checked resume self-test failed", file=sys.stderr)
            print(f"statuses={first, recovered, second, third, fourth}; worker-calls={calls}", file=sys.stderr)
            return 1
        cases_path = rules_root / "A" / "cases.toml"
        original_cases = cases_path.read_bytes()
        cases_path.write_bytes(original_cases + b"# staged-input mutation\n")
        try:
            verify_profile_sharded(
                "test", profile_root, worker, ruff, output, identities, 2, resume=True,
                runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(
                    AssertionError("changed staged cases should fail before worker execution")
                ),
            )
            print("changed staged case inputs were accepted on resume", file=sys.stderr)
            return 1
        except ValueError as error:
            if "staged profile inputs" not in str(error):
                raise
        finally:
            cases_path.write_bytes(original_cases)
        more_shards = verify_profile_sharded(
            "bounded-shards", profile_root, worker, ruff, output, identities, 5, runner=fake_runner
        )
        bounded_plan = json.loads(
            (output / "profiles" / "bounded-shards" / "shard-plan.json").read_text(encoding="utf-8")
        )
        if (
            more_shards != 0
            or bounded_plan.get("requested_shard_count") != 5
            or bounded_plan.get("shard_count") != len(inv)
            or calls[-len(inv):] != list(range(len(inv)))
        ):
            print("requested shards were not bounded to the nonempty case inventory", file=sys.stderr)
            return 1
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-profile-guards-") as temporary:
        root = Path(temporary)
        single_profile = PROFILE_DIR / "UP006" / "benchmark.toml"
        single_data = tomllib.loads(single_profile.read_text(encoding="utf-8"))
        registry = combined_modules(single_data, ["UP006"], manifest)
        module_id = "python/ruff/suppress"
        module_source = "fragments/suppress.ryoql"
        staged_profile = root / "single" / "benchmark.toml"
        staged_profile.parent.mkdir(parents=True)
        write_staged_profile(
            single_profile,
            staged_profile,
            registry,
            rule_applicability=selected_rule_applicability(manifest, ["UP006"]),
        )
        copy_modules(registry, staged_profile.parent)
        if (
            registry.get(module_id) != module_source
            or tomllib.loads(staged_profile.read_text(encoding="utf-8")).get("query-modules")
            != {module_id: module_source}
            or tomllib.loads(staged_profile.read_text(encoding="utf-8")).get("rule-applicability")
            != {"UP006": "accepted-syntax"}
            or not (staged_profile.parent / module_source).is_file()
        ):
            print("single-rule profile did not inherit the shared query module", file=sys.stderr)
            return 1
        py311_profile = root / "py311" / "benchmark.toml"
        py311_profile.parent.mkdir(parents=True)
        write_staged_profile(
            single_profile,
            py311_profile,
            registry,
            parser_configuration("py311"),
            selected_rule_applicability(manifest, ["UP006"]),
        )
        if tomllib.loads(py311_profile.read_text(encoding="utf-8")).get(
            "parser-configuration"
        ) != "python-target=3.11":
            print("target-specific parser configuration was not staged", file=sys.stderr)
            return 1
        try:
            parser_configuration("py312")
        except ValueError:
            pass
        else:
            print("unsupported Python target version was accepted", file=sys.stderr)
            return 1
        durable = root / "durable-run"
        durable.mkdir()
        repro = durable / "profiles" / "mock" / "repro"
        repro.mkdir(parents=True)
        (repro / "benchmark.toml").write_text('defaults = ["F401"]\n', encoding="utf-8")
        fake_output = subprocess.CompletedProcess(
            ["fake-worker"], 7, stdout=b"candidate stdout\n", stderr=b"candidate stderr\n"
        )
        calls = []

        def fake_runner(command, **options):
            calls.append((command, options))
            return fake_output

        command = ["fake-worker", "--case", "exact"]
        mock_executable = root / "mock-executable"
        mock_executable.write_bytes(b"fixed mock executable")
        mock_identity = {
            "identity": "sealed",
            "worker": {
                "executable": {
                    "path": str(mock_executable.resolve()),
                    "sha256": sha256(mock_executable),
                }
            },
        }
        if verify_profile(
            "mock", repro, command, durable, mock_identity, runner=fake_runner
        ) != 7:
            print("mock verifier status was not preserved", file=sys.stderr)
            return 1
        result_path = durable / "profiles" / "mock" / "result.json"
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if (
            len(calls) != 1
            or calls[0][0] != command
            or not calls[0][1]["capture_output"]
            or calls[0][1]["text"]
            or (durable / "profiles" / "mock" / "stdout.bin").read_bytes() != fake_output.stdout
            or (durable / "profiles" / "mock" / "stderr.bin").read_bytes() != fake_output.stderr
            or result["status"] != 7
            or result["identities"]["identity"] != "sealed"
            or result["identities"]["worker"]["executable"]["sha256"] != sha256(mock_executable)
            or result["staged_inputs"] != {"benchmark.toml": sha256(repro / "benchmark.toml")}
        ):
            print("durable verifier output or identities were not preserved", file=sys.stderr)
            return 1

        fake_worker = root / "ruff-fake-worker"
        fake_worker.write_text(
            "#!/usr/bin/env python3\n"
            "import os, pathlib, subprocess, sys\n"
            "label = 'child' if len(sys.argv) > 1 and sys.argv[1] == 'run-ryo' else 'parent'\n"
            "pathlib.Path(os.environ['WORKER_PATHS']).open('a').write(label + '|' + sys.argv[0] + '\\n')\n"
            "if label == 'parent': subprocess.run([sys.argv[0], 'run-ryo'], check=True)\n",
            encoding="utf-8",
        )
        fake_worker.chmod(0o755)
        worker_record = {"worker": {}}
        worker_identity(worker_record, fake_worker)
        if worker_record["worker"]["executable"] != {
            "path": str(fake_worker.resolve()),
            "sha256": sha256(fake_worker),
        }:
            print("frozen worker identity was not captured", file=sys.stderr)
            return 1

        build_root = root / "worker-build"
        build_root.mkdir()
        artifact_line = json.dumps(
            {
                "reason": "compiler-artifact",
                "target": {
                    "name": "ruff",
                    "kind": ["bench"],
                    "src_path": "/repo/languages/python/benchmarks/linters/ruff/bench.rs",
                    "test": False,
                },
                "executable": str(fake_worker),
            }
        ).encode()
        build_calls = []

        def successful_build_runner(build_command, **options):
            build_calls.append((build_command, options))
            return subprocess.CompletedProcess(
                build_command, 0, stdout=artifact_line + b"\n", stderr=b"build output\n"
            )

        source_identity = globals()["worker_source_identity"]
        stable_source_identity = {
            "repository_revision": "self-test-revision",
            "working_tree_status": [],
            "working_tree_diff_sha256": "0" * 64,
            "source_tree_sha256": "1" * 64,
        }
        globals()["worker_source_identity"] = lambda: dict(stable_source_identity)
        try:
            frozen_worker, build_identity = build_worker_once(
                build_root, contextlib.ExitStack(), runner=successful_build_runner
            )
        finally:
            globals()["worker_source_identity"] = source_identity
        globals()["worker_source_identity"] = lambda: (_ for _ in ()).throw(
            AssertionError("resuming a frozen worker must not re-identify current sources")
        )
        try:
            reused_worker, reused_identity = reuse_frozen_worker(build_root)
        finally:
            globals()["worker_source_identity"] = source_identity
        if reused_worker != frozen_worker or reused_identity["source_before_build"] != reused_identity["source_after_build"]:
            print("source edits after a stable build invalidated the frozen worker", file=sys.stderr)
            return 1
        original_worker_bytes = frozen_worker.read_bytes()
        frozen_worker.write_bytes(b"changed worker")
        try:
            reuse_frozen_worker(build_root)
            print("changed frozen worker bytes were accepted", file=sys.stderr)
            return 1
        except ValueError as error:
            if "missing or changed" not in str(error):
                raise
        finally:
            frozen_worker.write_bytes(original_worker_bytes)
        try:
            validate_output_directory(REPOSITORY / "crates" / "ryo" / "unsafe-output", resume=False)
            print("new output under the Rust source tree was accepted", file=sys.stderr)
            return 1
        except ValueError as error:
            if "outside crates/ryo" not in str(error):
                raise
        profile_commands = [
            command_for(frozen_worker, Path("ruff"), root / "profile-a"),
            command_for(frozen_worker, Path("ruff"), root / "profile-b"),
        ]
        worker_paths = root / "worker-paths.log"
        env = {**os.environ, "WORKER_PATHS": str(worker_paths)}
        runs = [subprocess.run(command, env=env, capture_output=True, check=False) for command in profile_commands]
        observed_paths = worker_paths.read_text(encoding="utf-8").splitlines()
        if (
            len(build_calls) != 1
            or len(runs) != 2
            or any(run.returncode != 0 for run in runs)
            or profile_commands[0][0] != profile_commands[1][0]
            or observed_paths
            != [
                f"parent|{frozen_worker}",
                f"child|{frozen_worker}",
                f"parent|{frozen_worker}",
                f"child|{frozen_worker}",
            ]
            or any("--bench" in command for command in profile_commands)
            or build_identity["frozen_executable"]["sha256"] != sha256(frozen_worker)
            or not build_identity["source_stable_during_build"]
        ):
            print("profile batch did not use one stable frozen worker", file=sys.stderr)
            return 1

        failed_build_root = root / "failed-worker-build"
        failed_build_root.mkdir()

        def failed_build_runner(build_command, **_options):
            return subprocess.CompletedProcess(
                build_command, 101, stdout=b"cargo stdout\n", stderr=b"cargo compiler diagnostic\n"
            )

        try:
            build_worker_once(failed_build_root, contextlib.ExitStack(), runner=failed_build_runner)
            print("failed worker build unexpectedly succeeded", file=sys.stderr)
            return 1
        except ValueError:
            failed_build = json.loads((failed_build_root / "build/result.json").read_text())
            if (
                failed_build["status"] != 101
                or (failed_build_root / "build/stdout.bin").read_bytes() != b"cargo stdout\n"
                or (failed_build_root / "build/stderr.bin").read_bytes()
                != b"cargo compiler diagnostic\n"
            ):
                print("failed worker build diagnostics were not persisted", file=sys.stderr)
                return 1

        source_identity = globals()["worker_source_identity"]
        identities = iter(
            [
                {"revision": "before", "source_tree_sha256": "before"},
                {"revision": "after", "source_tree_sha256": "after"},
                {"revision": "after", "source_tree_sha256": "after"},
            ]
        )
        globals()["worker_source_identity"] = lambda: next(identities)
        changed_build_root = root / "changed-worker-build"
        changed_build_root.mkdir()
        try:
            build_worker_once(
                changed_build_root,
                contextlib.ExitStack(),
                runner=successful_build_runner,
            )
            print("worker source edit during build unexpectedly succeeded", file=sys.stderr)
            return 1
        except ValueError:
            changed_build = json.loads((changed_build_root / "build/result.json").read_text())
            if changed_build["source_stable_during_build"] or "sources changed" not in changed_build["error"]:
                print("worker source edit during build was not recorded", file=sys.stderr)
                return 1
        finally:
            globals()["worker_source_identity"] = source_identity

        def failing_runner(_command, **_options):
            raise OSError("mock worker could not start")

        error_repro = durable / "profiles" / "mock-error" / "repro"
        error_repro.mkdir(parents=True)
        (error_repro / "cases.toml").write_text("[[cases]]\n", encoding="utf-8")
        if verify_profile(
            "mock-error", error_repro, command, durable, {}, runner=failing_runner
        ) != 1:
            print("mock verifier startup failure did not return failure", file=sys.stderr)
            return 1
        error_record = json.loads(
            (durable / "profiles" / "mock-error" / "result.json").read_text(encoding="utf-8")
        )
        if (
            error_record["status"] != 1
            or "mock worker could not start" not in error_record["error"]
            or not (durable / error_record["reproduction_directory"] / "cases.toml").is_file()
        ):
            print("verifier startup exception was not preserved", file=sys.stderr)
            return 1
        profile_source = root / "profile-source.toml"
        profile_source.write_text(
            'tool = "ruff"\nversion = "0"\nsha256 = ["pin"]\ndefaults = ["F401"]\n'
            '[query-modules] # existing but empty\n',
            encoding="utf-8",
        )
        module_registry = {"python/ruff/suppress": "rules/F401/candidate.ryoql"}
        copy_modules(module_registry, root)
        staged_profile = root / "module-profile.toml"
        write_staged_profile(profile_source, staged_profile, module_registry)
        parsed_profile = tomllib.loads(staged_profile.read_text(encoding="utf-8"))
        if parsed_profile.get("query-modules") != module_registry:
            print("staged query module registry did not remain valid TOML", file=sys.stderr)
            return 1
        staged_module = root / "rules/F401/candidate.ryoql"
        if not staged_module.is_file() or staged_module.read_bytes() != (RUFF_DIR / module_registry["python/ruff/suppress"]).read_bytes():
            print("staged query module bytes changed", file=sys.stderr)
            return 1
        shape_descriptor = manifest["combined_profiles"][SHAPE_PROFILE]
        expanded_descriptor = manifest["combined_profiles"]["expanded-defaults"]
        if combined_query_filename("LOG015", expanded_descriptor) != "query.ryoql":
            print("expanded cohort did not retain its sealed LOG015 query", file=sys.stderr)
            return 1
        for rule in ("LOG015", "ASYNC115"):
            profile_case = prepared_case_path(rule, manifest["profiles"][rule])
            canonical_case = RUFF_DIR / "rules" / rule / "cases.toml"
            if (
                profile_case == canonical_case
                or profile_case.read_bytes() == canonical_case.read_bytes()
                or combined_filename(rule, expanded_descriptor, "cases.toml") != "cases.toml"
                or combined_query_filename(rule, expanded_descriptor) != "query.ryoql"
            ):
                print(f"{rule} candidate snapshot changed the frozen expanded cohort", file=sys.stderr)
                return 1
        f821_record = manifest["profiles"]["F821"]
        f821_candidate = prepared_candidate_path("F821", f821_record)
        f821_cases = prepared_case_path("F821", f821_record)
        historical_candidate = combined_query_path("F821", expanded_descriptor)
        historical_cases = combined_case_path("F821", expanded_descriptor)
        if (
            f821_candidate != RUFF_DIR / "rules/F821/candidate.ryoql"
            or f821_cases != RUFF_DIR / "rules/F821/cases.toml"
            or historical_candidate == f821_candidate
            or historical_cases == f821_cases
            or historical_candidate.read_bytes() == f821_candidate.read_bytes()
            or historical_cases.read_bytes() == f821_cases.read_bytes()
        ):
            print("F821 profile snapshots changed the frozen expanded cohort", file=sys.stderr)
            return 1
        shape_profile = tomllib.loads(
            (REPOSITORY / shape_descriptor["profile_file"]).read_text(encoding="utf-8")
        )
        shape_registry = combined_modules(
            shape_profile, shape_descriptor["selected_rules"], manifest
        )
        shape_stage = root / "shape-110-profile"
        shape_stage.mkdir()
        copy_modules(shape_registry, shape_stage)
        write_staged_profile(
            REPOSITORY / shape_descriptor["profile_file"],
            shape_stage / "benchmark.toml",
            shape_registry,
        )
        if (
            shape_registry.get("python/ruff/suppress") != "fragments/suppress.ryoql"
            or not (shape_stage / "fragments/suppress.ryoql").is_file()
            or tomllib.loads((shape_stage / "benchmark.toml").read_text(encoding="utf-8"))
            .get("query-modules", {})
            .get("python/ruff/suppress")
            != "fragments/suppress.ryoql"
        ):
            print("combined profile omitted an active rule's query module", file=sys.stderr)
            return 1
        for relative in manifest["files"]:
            source = REPOSITORY / relative
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)

        helper_relative = Path(__file__).relative_to(REPOSITORY).as_posix()
        manifest["files"][helper_relative] = sha256(Path(__file__))
        temporary_manifest = root / MANIFEST.relative_to(REPOSITORY)
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        helper = root / helper_relative
        ruff = str(Path(sys.executable).resolve())
        pristine_manifest = json.loads(json.dumps(manifest))
        invalid_selection = subprocess.run(
            [sys.executable, str(helper), ruff, "--output-dir", str(root / "invalid-output"), "INVALID-RULE"],
            capture_output=True,
            text=True,
            check=False,
        )
        if invalid_selection.returncode != 2 or "unknown or duplicate profile rule" not in invalid_selection.stderr:
            print(f"invalid profile selection did not fail closed: status={invalid_selection.returncode}; stderr={invalid_selection.stderr!r}", file=sys.stderr)
            return 1
        duplicate_selection = subprocess.run(
            [sys.executable, str(helper), ruff, "--output-dir", str(root / "duplicate-output"), "F401", "F401"],
            capture_output=True,
            text=True,
            check=False,
        )
        if duplicate_selection.returncode != 2 or "unknown or duplicate profile rule" not in duplicate_selection.stderr:
            print("duplicate profile selection did not fail closed", file=sys.stderr)
            return 1

        candidate = root / next(
            path
            for path in manifest["files"]
            if path.endswith("/rules/F401/candidate.ryoql")
        )
        original_candidate = candidate.read_bytes()
        candidate.write_bytes(original_candidate + b"\n")
        changed_input = subprocess.run(
            [sys.executable, str(helper), ruff, "--output-dir", str(root / "changed-output"), "F401"],
            capture_output=True,
            text=True,
            check=False,
        )
        if changed_input.returncode != 2 or "prepared input hash mismatch" not in changed_input.stderr:
            print("prepared input hash mismatch did not fail closed", file=sys.stderr)
            return 1
        candidate.write_bytes(original_candidate)
        manifest = json.loads(temporary_manifest.read_text(encoding="utf-8"))
        last_profile = max(manifest["profiles"])
        manifest["profiles"][last_profile]["case_count"] += 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        fake_bin = root / "fake-bin"
        fake_bin.mkdir()
        marker = root / "worker-was-started"
        fake_cargo = fake_bin / "cargo"
        fake_git = fake_bin / "git"
        fake_git.write_text(
            "#!/bin/sh\n"
            "case \"$1\" in\n"
            "  rev-parse) echo self-test-revision ;;\n"
            "  diff|status|ls-files) : ;;\n"
            "  *) exit 2 ;;\n"
            "esac\n",
            encoding="utf-8",
        )
        fake_git.chmod(0o755)
        cargo_build_marker = root / "cargo-build-count"
        reference_worker = root / "reference-worker"
        reference_worker.write_text(
            "#!/bin/sh\n"
            "if [ \"$1\" = audit ]; then echo 'ruff 0.16.5: 104 queries authored / 413 shipped defaults (not a parity certificate)'; exit 0; fi\n"
            f"printf '%s\\n' \"$*\" >> {marker}\nexit 0\n",
            encoding="utf-8",
        )
        reference_worker.chmod(0o755)
        artifact_json = json.dumps(
            {
                "reason": "compiler-artifact",
                "target": {
                    "name": "ruff",
                    "kind": ["bench"],
                    "src_path": "/repo/languages/python/benchmarks/linters/ruff/bench.rs",
                    "test": False,
                },
                "executable": str(reference_worker),
            }
        )
        fake_cargo.write_text(
            "#!/bin/sh\n"
            "if [ \"$1\" = \"--version\" ]; then echo 'cargo test'; exit 0; fi\n"
            f"printf '%s\\n' \"$*\" >> {cargo_build_marker}\n"
            f"printf '%s\\n' '{artifact_json}'\n",
            encoding="utf-8",
        )
        fake_cargo.chmod(0o755)
        pinned_ruff = shutil.which("ruff")
        if pinned_ruff is None:
            print("reference-only self-test requires the pinned Ruff executable on PATH", file=sys.stderr)
            return 1
        reference_output = root / "reference-only-output"
        reference_only = subprocess.run(
            [sys.executable, str(helper), pinned_ruff, "--output-dir", str(reference_output), "--reference-only", "F401"],
            capture_output=True,
            text=True,
            check=False,
            env={
                **os.environ,
                "PATH": f"{fake_bin}:{os.environ['PATH']}",
                "RYO_RUFF_PREPARED_MANIFEST": str(temporary_manifest),
            },
        )
        builds = cargo_build_marker.read_text().splitlines() if cargo_build_marker.exists() else []
        reference_records = reference_output / "profiles" / "F401" / "reference-cases.jsonl"
        if (
            reference_only.returncode != 0
            or builds
            or not reference_records.is_file()
            or len(reference_records.read_text(encoding="utf-8").splitlines()) != 22
        ):
            print(
                "reference-only mode did not verify all cases without building a candidate: "
                f"status={reference_only.returncode}; stderr={reference_only.stderr!r}; "
                f"builds={builds!r}; records={reference_records.is_file()}",
                file=sys.stderr,
            )
            return 1
        cargo_build_marker.unlink(missing_ok=True)
        invalid_rule_manifest = json.loads(json.dumps(pristine_manifest))
        invalid_rule_manifest["combined_profiles"]["expanded-defaults"]["selected_rules"].append(
            "NOT-SHIPPED"
        )
        temporary_manifest.write_text(
            json.dumps(invalid_rule_manifest, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        invalid_rule = subprocess.run(
            [sys.executable, str(helper), ruff, "--output-dir", str(root / "invalid-rule-output"), "--profile", "expanded-defaults"],
            capture_output=True,
            text=True,
            check=False,
            env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
        )
        if (
            invalid_rule.returncode != 2
            or "combined profile contains a non-default rule" not in invalid_rule.stderr
            or marker.exists()
        ):
            print("non-default combined rule did not fail before candidate work", file=sys.stderr)
            return 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

        def later_combined_preflight_failure() -> tuple[bool, str]:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            later_name = next(
                name
                for name, descriptor in sorted(trial_manifest["combined_profiles"].items())
                if descriptor.get("validation_status") == "active-current"
            )
            for item in trial_manifest["combined_profiles"].values():
                item["validation_status"] = "archived-historical"
            descriptor = trial_manifest["combined_profiles"][later_name]
            descriptor["validation_status"] = "active-current"
            descriptor["case_count"] += 1
            temporary_manifest.write_text(
                json.dumps(trial_manifest, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            marker.unlink(missing_ok=True)
            preflight_output = root / "later-preflight-output"
            check = subprocess.run(
                [sys.executable, str(helper), ruff, "--output-dir", str(preflight_output)],
                capture_output=True,
                text=True,
                check=False,
                env={
                    **os.environ,
                    "PATH": f"{fake_bin}:{os.environ['PATH']}",
                    "RYO_RUFF_PREPARED_MANIFEST": str(temporary_manifest),
                },
            )
            receipt_path = preflight_output / "preflight-error.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8")) if receipt_path.is_file() else {}
            valid = (
                check.returncode == 2
                and "complete expected inventory count mismatch" in check.stderr
                and not marker.exists()
                and receipt.get("manifest_path") == temporary_manifest.relative_to(root).as_posix()
                and receipt.get("manifest_sha256") == sha256(temporary_manifest)
            )
            return valid, check.stderr

        later_combined_refused, later_combined_output = later_combined_preflight_failure()
        if not later_combined_refused:
            print("later active combined profile did not fail before candidate work", file=sys.stderr)
            print(later_combined_output, file=sys.stderr)
            return 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        overlay_relative = pristine_manifest["combined_profiles"][SHAPE_PROFILE]["expectations_file"]
        baseline_overlay = (REPOSITORY / overlay_relative).read_bytes()
        (root / pristine_manifest["combined_profiles"]["expanded-defaults"]["expectations_file"]).write_bytes(
            (REPOSITORY / pristine_manifest["combined_profiles"]["expanded-defaults"]["expectations_file"]).read_bytes()
        )
        (root / pristine_manifest["combined_profiles"]["shape-110"]["expectations_file"]).write_bytes(
            baseline_overlay
        )
        preflight = subprocess.run(
            [sys.executable, str(helper), str(Path(sys.executable).resolve()), "--output-dir", str(root / "inventory-output")],
            capture_output=True,
            text=True,
            check=False,
            env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
        )
        if (
            preflight.returncode != 2
            or f"case inventory mismatch for {last_profile}" not in preflight.stderr
            or marker.exists()
        ):
            print(
                f"later-profile preflight guard failed: status={preflight.returncode}; "
                f"stderr={preflight.stderr.strip()!r}; marker={marker.exists()}",
                file=sys.stderr,
            )
            return 1

        def shape_preflight_failure(kind: str, expected_error: str) -> bool:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            descriptor = trial_manifest["combined_profiles"][SHAPE_PROFILE]
            overlay_path = root / descriptor["expectations_file"]
            overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
            if kind == "source-hash":
                overlay["cases"][0]["case_source_sha256"] = "0" * 64
            else:
                overlay["cases"][0]["case"] = "unknown-case-key"
            overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            overlay_hash = sha256(overlay_path)
            descriptor["expectations_sha256"] = overlay_hash
            trial_manifest["files"][descriptor["expectations_file"]] = overlay_hash
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            marker.unlink(missing_ok=True)
            check = subprocess.run(
                [sys.executable, str(helper), ruff, "--output-dir", str(root / f"shape-{kind}-output"), "--profile", SHAPE_PROFILE],
                capture_output=True,
                text=True,
                check=False,
                env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
            )
            return check.returncode == 2 and expected_error in check.stderr and not marker.exists()

        # Each case gets a fresh source snapshot because the helper itself refuses any stale seal.
        for kind, message in (("source-hash", "case source hash mismatch"), ("case-key", "overlay case is absent")):
            (root / overlay_relative).write_bytes(baseline_overlay)
            if not shape_preflight_failure(kind, message):
                print(f"shape {kind} refusal did not fail before candidate work", file=sys.stderr)
                return 1
        (root / overlay_relative).write_bytes(baseline_overlay)

        def expanded_preflight_failure(kind: str, expected_error: str) -> bool:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            active_name = next(
                name
                for name, item in trial_manifest["combined_profiles"].items()
                if item.get("validation_status") == "active-current"
            )
            descriptor = trial_manifest["combined_profiles"][active_name]
            overlay_path = root / descriptor["expectations_file"]
            overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
            if kind == "source-hash":
                overlay["cases"][0]["case_source_sha256"] = "0" * 64
            elif kind == "case-key":
                overlay["cases"][0]["case"] = "unknown-case-key"
            elif kind == "missing-case":
                overlay["cases"].pop()
            elif kind == "missing-rule":
                item = next(row for row in overlay["cases"] if row["expected"])
                item["expected"][0].pop("rule")
            elif kind == "duplicate":
                item = duplicate = None
                for row in overlay["cases"]:
                    cases_path = REPOSITORY / row["cases_file"]
                    source_cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
                    canonical = next(case for case in source_cases if case["name"] == row["case"])
                    original = collections.Counter(
                        (
                            finding.get("rule", row["rule"]),
                            finding["file"],
                            finding["start"],
                            finding["end"],
                        )
                        for finding in canonical["expected"]
                    )
                    current = collections.Counter(
                        (finding["rule"], finding["file"], finding["start"], finding["end"])
                        for finding in row["expected"]
                    )
                    extra_duplicate = next(
                        (key for key, count in current.items() if count > 1 and count > original[key]),
                        None,
                    )
                    if extra_duplicate is not None:
                        item, duplicate = row, extra_duplicate
                        break
                if item is None or duplicate is None:
                    raise ValueError("no externally added duplicate to exercise")
                item["expected"].remove(
                    next(
                        finding
                        for finding in item["expected"]
                        if (finding["rule"], finding["file"], finding["start"], finding["end"])
                        == duplicate
                    )
                )
                overlay["expected_diagnostic_count"] -= 1
            else:
                raise ValueError(f"unknown self-test: {kind}")
            overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            overlay_hash = sha256(overlay_path)
            descriptor["expectations_sha256"] = overlay_hash
            trial_manifest["files"][descriptor["expectations_file"]] = overlay_hash
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            marker.unlink(missing_ok=True)
            check = subprocess.run(
                [sys.executable, str(helper), ruff, "--output-dir", str(root / f"active-{kind}-output"), "--profile", active_name],
                capture_output=True,
                text=True,
                check=False,
                env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
            )
            success = check.returncode == 2 and expected_error in check.stderr and not marker.exists()
            if not success:
                print(
                    f"active {kind}: status={check.returncode}; stderr={check.stderr.strip()!r}; "
                    f"cargo-started={marker.exists()}",
                    file=sys.stderr,
                )
            return success

        active_name = next(
            name
            for name, item in pristine_manifest["combined_profiles"].items()
            if item.get("validation_status") == "active-current"
        )
        expanded_overlay_relative = pristine_manifest["combined_profiles"][active_name]["expectations_file"]
        expanded_overlay_bytes = (root / expanded_overlay_relative).read_bytes()
        for kind, message in (
            ("source-hash", "case source hash mismatch"),
            ("case-key", "overlay case is absent or duplicated"),
            ("missing-case", "complete expected inventory count mismatch"),
            ("missing-rule", f"invalid {active_name} expected finding"),
            ("duplicate", "overlay diagnostics differ from the sealed oracle"),
        ):
            (root / expanded_overlay_relative).write_bytes(expanded_overlay_bytes)
            if not expanded_preflight_failure(kind, message):
                print(f"active full-report {kind} refusal did not fail before candidate work", file=sys.stderr)
                return 1
    print("profile selection and hash-mismatch guards passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ruff", type=Path, nargs="?", help="pinned Ruff executable")
    parser.add_argument("rules", nargs="*", help="optional rule IDs; default: every prepared profile")
    parser.add_argument("--profile", help="run one named combined prepared profile")
    parser.add_argument("--shards", type=int, help="execute each case once across deterministic case shards")
    parser.add_argument("--resume", action="store_true", help="reuse only complete shard receipts with identical sealed identities")
    parser.add_argument("--parity-check", help="single-rule profile to compare monolithic and two-shard full reports")
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="persist staged repro inputs, identities, exit status, stdout and stderr",
    )
    parser.add_argument("--reference-only", action="store_true", help="check sealed expected findings with Ruff only")
    parser.add_argument("--self-test", action="store_true", help="exercise fail-closed profile guards without running Cargo")
    arguments = parser.parse_args()
    if arguments.self_test:
        return self_test()
    if arguments.ruff is None:
        parser.error("the pinned Ruff executable is required unless --self-test is selected")
    if arguments.shards is not None and (arguments.shards < 2 or arguments.reference_only):
        parser.error("--shards requires at least two shards and candidate verification")
    if arguments.resume and (arguments.shards is None or arguments.output_dir is None):
        parser.error("--resume requires --shards and --output-dir")
    if arguments.resume and arguments.profile is None:
        parser.error("--resume requires the exact prior --profile selection")
    if arguments.resume and arguments.rules:
        parser.error("--resume does not accept a changed rule selection")
    if not arguments.reference_only and arguments.output_dir is None:
        parser.error("candidate verification requires --output-dir for exact case receipts")

    output_directory = arguments.output_dir.resolve() if arguments.output_dir else None
    if arguments.parity_check is not None and (
        output_directory is None
        or arguments.shards is None
        or arguments.resume
        or arguments.parity_check not in arguments.rules
        or (arguments.profile is None and len(arguments.rules) != 1)
    ):
        parser.error("--parity-check requires output-dir, shards, and exactly one selected rule (plus a combined profile when specified)")
    if output_directory is not None:
        if arguments.resume:
            if not output_directory.is_dir():
                parser.error("resume output directory does not exist")
        else:
            try:
                validate_output_directory(output_directory, resume=False)
            except ValueError as error:
                parser.error(str(error))
            output_directory.mkdir(parents=True, exist_ok=False)
            shutil.copyfile(MANIFEST, output_directory / "prepared.json")

    try:
        manifest = (
            resume_manifest(output_directory, arguments.profile)
            if arguments.resume and output_directory is not None and arguments.profile is not None
            else json.loads(MANIFEST.read_text(encoding="utf-8"))
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    helper_path = Path(__file__).relative_to(REPOSITORY).as_posix()
    statuses = []
    try:
        if not arguments.resume:
            verify_files(manifest, helper_path)
        if arguments.profile is None and not arguments.rules and not arguments.resume:
            verify_active_combined_cohort(manifest)
        if arguments.profile is not None:
            if arguments.profile not in manifest.get("combined_profiles", {}):
                raise ValueError(f"unknown prepared combined profile: {arguments.profile}")
            if arguments.resume:
                verify_active_combined_cohort(manifest)
            defaults = set(
                tomllib.loads((RUFF_DIR / "benchmark.toml").read_text(encoding="utf-8"))["defaults"]
            )
            if len(set(arguments.rules)) != len(arguments.rules) or set(arguments.rules) - defaults:
                raise ValueError("unknown or duplicate profile rule")
            selections = [arguments.profile]
        else:
            profiles = manifest["profiles"]
            defaults = set(
                tomllib.loads((RUFF_DIR / "benchmark.toml").read_text(encoding="utf-8"))["defaults"]
            )
            if len(set(arguments.rules)) != len(arguments.rules) or set(arguments.rules) - defaults:
                raise ValueError("unknown or duplicate profile rule")
            selections = [
                profile_name
                for profile_name, profile in sorted(profiles.items())
                if not arguments.rules
                or profile.get("rule_id", profile_name) in arguments.rules
            ]
            if not selections:
                raise ValueError("selected rules have no prepared candidate profiles")
        ruff = arguments.ruff.resolve()
        with contextlib.ExitStack() as stack:
            staged = []
            if arguments.profile is not None:
                descriptor = manifest["combined_profiles"][arguments.profile]
                if arguments.resume:
                    staged.append((arguments.profile, output_directory / "profiles" / arguments.profile / "repro"))
                else:
                    if arguments.rules:
                        single_profiles = [
                            profile_name
                            for profile_name, profile in sorted(manifest["profiles"].items())
                            if profile.get("rule_id", profile_name) in arguments.rules
                        ]
                        staged.extend(stage_binding(manifest, single_profiles, stack, output_directory))
                    staged.append(
                        stage_combined(arguments.profile, descriptor, manifest, stack, output_directory)
                    )
            else:
                staged.extend(stage_binding(manifest, selections, stack, output_directory))
                if not arguments.rules:
                    for name, descriptor in sorted(manifest.get("combined_profiles", {}).items()):
                        if descriptor.get("validation_status") != "active-current":
                            continue
                        staged.append(
                            stage_combined(name, descriptor, manifest, stack, output_directory)
                        )
            if arguments.resume:
                if output_directory is None:
                    raise ValueError("resume requires an output directory")
                worker_executable, build_identity = reuse_frozen_worker(output_directory)
            elif arguments.reference_only:
                worker_executable = None
                build_identity = {
                    "status": "not-built",
                    "reason": "reference-only mode invokes the sealed Ruff executable directly",
                }
            else:
                worker_executable, build_identity = build_worker_once(output_directory, stack)
            if not arguments.reference_only:
                assert worker_executable is not None
                protocol_ok, protocol_message = audit_worker_protocol(worker_executable)
                protocol_record = {
                    "command": [str(worker_executable), "audit"],
                    "worker_sha256": sha256(worker_executable),
                    "status": 0 if protocol_ok else 1,
                    "message": protocol_message,
                }
                build_identity["protocol_smoke"] = protocol_record
                if not protocol_ok:
                    if output_directory is not None:
                        atomic_json_write(output_directory / "worker-protocol.json", protocol_record)
                    print(f"worker protocol smoke failed: {protocol_message}", file=sys.stderr)
                    return 1
            for name, profile_root in staged:
                if arguments.reference_only:
                    ref_identity = reference_tool_identity(ruff, manifest)
                    identities = {
                        "manifest_path": MANIFEST.relative_to(REPOSITORY).as_posix(),
                        "manifest_sha256": sha256(MANIFEST),
                        "target_version": tomllib.loads(
                            (profile_root / "benchmark.toml").read_text(encoding="utf-8")
                        ).get("parser-configuration", "python-target=3.10"),
                        "pinned_reference": ref_identity,
                        "staged_profile_sha256": staged_hashes(profile_root),
                        "selected_rule_applicability": tomllib.loads(
                            (profile_root / "benchmark.toml").read_text(encoding="utf-8")
                        ).get("rule-applicability", {}),
                    }
                    status = verify_reference_profile(
                        name, profile_root, ruff, output_directory, identities
                    )
                    print(f"{name}: verifier exit status {status}", file=sys.stderr)
                    statuses.append((name, status))
                    continue
                assert worker_executable is not None
                executable = build_identity["frozen_executable"]
                if not isinstance(executable, dict) or sha256(Path(executable["path"])) != executable["sha256"]:
                    raise ValueError("frozen worker executable hash changed before profile invocation")
                command = command_for(worker_executable, ruff, profile_root, False)
                if arguments.resume:
                    prior_plan_path = output_directory / "profiles" / name / "shard-plan.json"
                    prior_plan = json.loads(prior_plan_path.read_text(encoding="utf-8"))
                    identities = prior_plan["global_identity"]["selection"]
                    reference_identity = reference_tool_identity(ruff, manifest)
                    if (
                        identities.get("manifest_sha256") != sha256(output_directory / "prepared.json")
                        or identities.get("ruff", {}).get("sha256") != reference_identity["sha256"]
                    ):
                        raise ValueError("cannot resume with a changed manifest or pinned reference executable")
                else:
                    identities = run_identity(manifest, ruff, profile_root, worker_executable, build_identity)
                worker_identity(identities, worker_executable)
                if arguments.shards is not None and (
                    arguments.profile is not None and name == arguments.profile
                    or arguments.profile is None and arguments.parity_check is None
                ):
                    if output_directory is None:
                        raise ValueError("sharded verification requires --output-dir for durable receipts")
                    status = verify_profile_sharded(
                        name,
                        profile_root,
                        worker_executable,
                        ruff,
                        output_directory,
                        identities,
                        arguments.shards,
                        resume=arguments.resume,
                    )
                else:
                    status = verify_profile(name, profile_root, command, output_directory, identities)
                print(f"{name}: verifier exit status {status}", file=sys.stderr)
                statuses.append((name, status))
            if arguments.parity_check is not None:
                parity_name = arguments.parity_check
                parity_root = output_directory / "profiles" / parity_name / "repro"
                parity_identity = run_identity(
                    manifest, ruff, parity_root, worker_executable, build_identity
                )
                worker_identity(parity_identity, worker_executable)
                shard_status = verify_profile_sharded(
                    parity_name,
                    parity_root,
                    worker_executable,
                    ruff,
                    output_directory,
                    parity_identity,
                    2,
                )
                shard_paths = sorted(
                    (output_directory / "profiles" / parity_name / "shards").glob("*.cases.jsonl")
                )
                mono_path = output_directory / "profiles" / parity_name / "cases.jsonl"
                observations_match = case_observations_match(mono_path, shard_paths)
                parity_status = (
                    0
                    if dict(statuses).get(parity_name) == 0
                    and shard_status == 0
                    and observations_match
                    else 1
                )
                atomic_json_write(
                    output_directory / "profiles" / parity_name / "parity.json",
                    {
                        "profile": parity_name,
                        "status": parity_status,
                        "worker_sha256": sha256(worker_executable),
                        "monolithic_case_results_sha256": sha256(mono_path) if mono_path.is_file() else None,
                        "sharded_case_result_shas": {path.name: sha256(path) for path in shard_paths},
                        "full_report_observations_match": observations_match,
                        "rule_profile_status": dict(statuses).get(parity_name),
                        "sharded_status": shard_status,
                    },
                )
                print(f"{parity_name}: monolithic/sharded full-report parity {parity_status}", file=sys.stderr)
                statuses.append((f"{parity_name}-shard-parity", parity_status))
    except (KeyError, TypeError, ValueError, OSError) as error:
        if output_directory is not None:
            (output_directory / "preflight-error.json").write_text(
                json.dumps(
                    {
                        "error": f"{type(error).__name__}: {error}",
                        "manifest_path": MANIFEST.relative_to(REPOSITORY).as_posix(),
                        "manifest_sha256": sha256(MANIFEST),
                        "arguments": sys.argv[1:],
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
        parser.error(str(error))
    failed = [rule for rule, status in statuses if status != 0]
    if failed:
        print(f"candidate profiles failed: {', '.join(failed)}", file=sys.stderr)
    if output_directory is not None:
        (output_directory / "run.json").write_text(
            json.dumps(
                {
                    "manifest_path": MANIFEST.relative_to(REPOSITORY).as_posix(),
                    "manifest_sha256": sha256(MANIFEST),
                    "target_version": manifest.get("target_version"),
                    "ruff_argument": str(arguments.ruff),
                    "reference_version": manifest.get("reference_version"),
                    "reference_sha256": manifest.get("reference_sha256"),
                    "worker_build": build_identity,
                    "statuses": dict(statuses),
                    "aggregate_status": 1 if failed else 0,
                },
                indent=2,
                ensure_ascii=False,
            )
            + "\n",
            encoding="utf-8",
        )
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
