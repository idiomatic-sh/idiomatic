"""Materialize and verify prepared Ruff candidate profiles."""

from __future__ import annotations

import argparse
import collections
import contextlib
import hashlib
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import tomllib

RUFF_DIR = Path(__file__).resolve().parent
REPOSITORY = RUFF_DIR.parents[6]
PROFILE_DIR = RUFF_DIR / "profiles"
MANIFEST = PROFILE_DIR / "prepared.json"
SHAPE_PROFILE = "shape-110"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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
    base_data = tomllib.loads(base_profile.read_text(encoding="utf-8"))
    paths.update(module_path(path) for path in profile_modules(base_data).values())
    for rule in profiles:
        profile = PROFILE_DIR / rule / "benchmark.toml"
        recorded = profiles[rule]
        case_file = prepared_case_path(rule, recorded)
        paths.update({
                profile.relative_to(REPOSITORY).as_posix(),
                (RUFF_DIR / "rules" / rule / "candidate.ryoql").relative_to(REPOSITORY).as_posix(),
                case_file.relative_to(REPOSITORY).as_posix(),
            })
        profile_data = tomllib.loads(profile.read_text(encoding="utf-8"))
        paths.update(module_path(path) for path in profile_modules(profile_data).values())
        if "oracle_file" in profiles[rule]:
            paths.add(profiles[rule]["oracle_file"])
        if "reference_audit_file" in profiles[rule]:
            paths.add(profiles[rule]["reference_audit_file"])
    for descriptor in manifest.get("combined_profiles", {}).values():
        paths.update(descriptor["input_files"])
        combined_profile = REPOSITORY / descriptor["profile_file"]
        combined_data = tomllib.loads(combined_profile.read_text(encoding="utf-8"))
        paths.update(module_path(path) for path in profile_modules(combined_data).values())
    return paths


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


def write_staged_profile(source: Path, destination: Path, registry: dict[str, str]) -> None:
    """Serialize the supported profile fields, replacing any existing module table."""
    parsed = tomllib.loads(source.read_text(encoding="utf-8"))
    current = profile_modules(parsed)
    registry = profile_modules({"query-modules": registry})
    if any(key in registry and registry[key] != value for key, value in current.items()):
        raise ValueError("staged profile conflicts with its query module registry")
    merged = {**current, **registry}
    parsed.pop("query-modules", None)
    if merged:
        parsed["query-modules"] = merged
    rendered = "".join(f"{key} = {toml_value(value)}\n" for key, value in parsed.items())
    destination.write_text(rendered, encoding="utf-8")


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
    if set(files) != manifest_paths(manifest, helper_path):
        raise ValueError("prepared input paths do not match the candidate acceptance cohorts")
    defaults = set(tomllib.loads((RUFF_DIR / "benchmark.toml").read_text(encoding="utf-8"))["defaults"])
    if set(manifest["profiles"]) - defaults:
        raise ValueError("prepared profile contains a non-default rule")
    for descriptor in manifest.get("combined_profiles", {}).values():
        selected = descriptor["selected_rules"]
        if len(selected) != len(set(selected)) or set(selected) - defaults:
            raise ValueError("combined profile contains a non-default rule")
    if files.get(helper_path) != sha256(Path(__file__)):
        raise ValueError("profile helper hash does not match prepared.json")
    for relative, expected in files.items():
        path = REPOSITORY / relative
        if not path.is_file() or sha256(path) != expected:
            raise ValueError(f"prepared input hash mismatch: {relative}")


def profile_tool_matches(profile_data: dict[str, object], manifest: dict[str, object]) -> bool:
    return (
        profile_data.get("tool") == "ruff"
        and profile_data.get("version") == manifest["reference_version"]
        and profile_data.get("sha256") == manifest["reference_sha256"]
    )


def command_for(ruff: Path, profile_root: Path, reference_only: bool = False) -> list[str]:
    return [
        "cargo",
        "bench",
        "--verbose",
        "--locked",
        "-p",
        "idiomatic-sh-ryo-python",
        "--bench",
        "ruff",
        "--",
        "verify-reference-cases" if reference_only else "verify-cases",
        str(ruff),
        str(profile_root),
    ]


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
) -> int:
    """Persist raw command output and exact identities before temporary inputs can be removed."""
    status = result.returncode if result is not None else 1
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
        "error": error,
        "identities": identities or {},
        "staged_inputs": staged_hashes(profile_root) if profile_root.is_dir() else {},
        "stdout_file": f"profiles/{name}/stdout.bin",
        "stderr_file": f"profiles/{name}/stderr.bin",
        "reproduction_directory": f"profiles/{name}/repro",
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
    try:
        result = runner(
            command,
            cwd=REPOSITORY / "crates" / "ryo",
            capture_output=True,
            text=False,
            check=False,
        )
    except Exception as error:
        worker_identity(identities, None)
        return persist_run_record(
            output_directory,
            name,
            command,
            profile_root,
            None,
            error=f"{type(error).__name__}: {error}",
            identities=identities,
        )
    worker_identity(identities, result.stderr)
    return persist_run_record(
        output_directory,
        name,
        command,
        profile_root,
        result,
        identities=identities,
    )


def worker_identity(identities: dict[str, object], stderr: bytes | None) -> None:
    """Record the exact benchmark executable Cargo launched, when it reports one."""
    worker = identities.setdefault("worker", {})
    if not isinstance(worker, dict):
        return
    executable = None
    if stderr is not None:
        for line in stderr.decode(errors="replace").splitlines():
            if not line.startswith("Running `") or not line.endswith("`"):
                continue
            try:
                arguments = shlex.split(line[len("Running `") : -1])
            except ValueError:
                continue
            if arguments and Path(arguments[0]).name.startswith("ruff-"):
                candidate = Path(arguments[0])
                if candidate.is_file():
                    executable = candidate.resolve()
                    break
    worker["executable"] = (
        {"path": str(executable), "sha256": sha256(executable)}
        if executable is not None
        else {"available": False, "reason": "Cargo launch path not found in captured stderr"}
    )


def run_identity(
    manifest: dict[str, object], ruff: Path, profile_root: Path
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
    for rule in selected:
        profile = PROFILE_DIR / rule / "benchmark.toml"
        profile_data = tomllib.loads(profile.read_text(encoding="utf-8"))
        if profile_data.get("defaults") != [rule]:
            raise ValueError(f"profile must select only {rule}")
        if not profile_tool_matches(profile_data, manifest):
            raise ValueError(f"tool identity mismatch for {rule}")
        candidate_path = RUFF_DIR / "rules" / rule / "candidate.ryoql"
        recorded = profiles[rule]
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
        destination = stage_directory(rule, stack, output_directory)
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        registry = profile_modules(profile_data)
        copy_modules(registry, destination)
        write_staged_profile(profile, destination / "benchmark.toml", registry)
        shutil.copyfile(candidate_path, rule_dir / "query.ryoql")
        shutil.copyfile(cases_path, rule_dir / "cases.toml")
        staged.append((rule, destination))
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
        cases_path = REPOSITORY / item["cases_file"]
        if sha256(cases_path) != item["canonical_cases_sha256"]:
            raise ValueError(f"{profile_name} canonical case hash mismatch: {item['cases_file']}")
        source_cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        matching = [case for case in source_cases if case["name"] == item["case"]]
        if len(matching) != 1:
            raise ValueError(f"{profile_name} overlay case is absent or duplicated: {key}")
        case = matching[0]
        if not full_report and case.get("comparison", "rule") != "full":
            raise ValueError(f"{profile_name} overlay requires a full-report case: {key}")
        if item["cases_file"] != (RUFF_DIR / "rules" / item["rule"] / "cases.toml").relative_to(REPOSITORY).as_posix():
            raise ValueError(f"{profile_name} overlay names a noncanonical case file: {key}")
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
        source_dir = RUFF_DIR / "rules" / rule
        candidate = source_dir / "candidate.ryoql"
        query = candidate if candidate.is_file() else source_dir / "query.ryoql"
        cases_path = source_dir / combined_filename(rule, descriptor, "cases.toml")
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
    write_staged_profile(profile_path, destination / "benchmark.toml", registry)
    for rule in selected_rules:
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        source_dir = RUFF_DIR / "rules" / rule
        query_path = source_dir / combined_query_filename(rule, descriptor)
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


def combined_query_filename(rule: str, descriptor: dict[str, object]) -> str:
    """Use the source filename sealed by this combined cohort, even if a candidate is added later."""
    return combined_filename(rule, descriptor, "candidate.ryoql", "query.ryoql")


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
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-profile-guards-") as temporary:
        root = Path(temporary)
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
        if verify_profile(
            "mock", repro, command, durable, {"identity": "sealed"}, runner=fake_runner
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
            or result["identities"]["worker"]["executable"]["available"] is not False
            or result["staged_inputs"] != {"benchmark.toml": sha256(repro / "benchmark.toml")}
        ):
            print("durable verifier output or identities were not preserved", file=sys.stderr)
            return 1

        fake_worker = root / "ruff-fake-worker"
        fake_worker.write_bytes(b"worker binary fixture")
        worker_record = {"worker": {}}
        worker_identity(worker_record, f"Running `{fake_worker} --exact`\n".encode())
        if worker_record["worker"]["executable"] != {
            "path": str(fake_worker.resolve()),
            "sha256": sha256(fake_worker),
        }:
            print("launched worker binary identity was not captured", file=sys.stderr)
            return 1

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
            [sys.executable, str(helper), ruff, "INVALID-RULE"],
            capture_output=True,
            text=True,
            check=False,
        )
        if invalid_selection.returncode != 2 or "unknown or duplicate profile rule" not in invalid_selection.stderr:
            print("invalid profile selection did not fail closed", file=sys.stderr)
            return 1
        duplicate_selection = subprocess.run(
            [sys.executable, str(helper), ruff, "F401", "F401"],
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
            [sys.executable, str(helper), ruff, "F401"],
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
        marker = root / "cargo-was-started"
        fake_cargo = fake_bin / "cargo"
        fake_cargo.write_text(f"#!/bin/sh\nprintf '%s\\n' \"$*\" >> {marker}\n", encoding="utf-8")
        fake_cargo.chmod(0o755)
        reference_only = subprocess.run(
            [sys.executable, str(helper), ruff, "--reference-only", "F401"],
            capture_output=True,
            text=True,
            check=False,
            env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
        )
        invocations = marker.read_text().splitlines() if marker.exists() else []
        worker_invocations = [line for line in invocations if "verify-reference-cases" in line]
        if (
            reference_only.returncode != 0
            or len(worker_invocations) != 1
        ):
            print(
                "reference-only mode did not dispatch the reference verifier: "
                f"status={reference_only.returncode}; stderr={reference_only.stderr!r}; "
                f"invocations={invocations!r}",
                file=sys.stderr,
            )
            return 1
        marker.unlink()
        invalid_rule_manifest = json.loads(json.dumps(pristine_manifest))
        invalid_rule_manifest["combined_profiles"]["expanded-defaults"]["selected_rules"].append(
            "NOT-SHIPPED"
        )
        temporary_manifest.write_text(
            json.dumps(invalid_rule_manifest, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        invalid_rule = subprocess.run(
            [sys.executable, str(helper), ruff, "--profile", "expanded-defaults"],
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
            later_name = max(trial_manifest["combined_profiles"])
            descriptor = trial_manifest["combined_profiles"][later_name]
            overlay_path = root / descriptor["expectations_file"]
            overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
            overlay["cases"][0]["case"] = "unknown-later-combined-case"
            overlay_path.write_text(
                json.dumps(overlay, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            overlay_hash = sha256(overlay_path)
            descriptor["expectations_sha256"] = overlay_hash
            trial_manifest["files"][descriptor["expectations_file"]] = overlay_hash
            temporary_manifest.write_text(
                json.dumps(trial_manifest, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            marker.unlink(missing_ok=True)
            check = subprocess.run(
                [sys.executable, str(helper), ruff],
                capture_output=True,
                text=True,
                check=False,
                env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
            )
            valid = (
                check.returncode == 2
                and "overlay case is absent or duplicated" in check.stderr
                and not marker.exists()
            )
            return valid, check.stderr

        later_combined_refused, later_combined_output = later_combined_preflight_failure()
        if not later_combined_refused:
            print("later combined profile did not fail before candidate work", file=sys.stderr)
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
            [sys.executable, str(helper), str(Path(sys.executable).resolve())],
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
                [sys.executable, str(helper), ruff, "--profile", SHAPE_PROFILE],
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
            descriptor = trial_manifest["combined_profiles"]["expanded-defaults"]
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
                [sys.executable, str(helper), ruff, "--profile", "expanded-defaults"],
                capture_output=True,
                text=True,
                check=False,
                env={**os.environ, "PATH": f"{fake_bin}:{os.environ['PATH']}"},
            )
            success = check.returncode == 2 and expected_error in check.stderr and not marker.exists()
            if not success:
                print(
                    f"expanded {kind}: status={check.returncode}; stderr={check.stderr.strip()!r}; "
                    f"cargo-started={marker.exists()}",
                    file=sys.stderr,
                )
            return success

        expanded_overlay_relative = pristine_manifest["combined_profiles"]["expanded-defaults"]["expectations_file"]
        expanded_overlay_bytes = (root / expanded_overlay_relative).read_bytes()
        for kind, message in (
            ("source-hash", "case source hash mismatch"),
            ("case-key", "overlay case is absent or duplicated"),
            ("missing-case", "complete expected inventory count mismatch"),
            ("missing-rule", "invalid expanded-defaults expected finding"),
            ("duplicate", "overlay diagnostics differ from the sealed oracle"),
        ):
            (root / expanded_overlay_relative).write_bytes(expanded_overlay_bytes)
            if not expanded_preflight_failure(kind, message):
                print(f"expanded full-report {kind} refusal did not fail before candidate work", file=sys.stderr)
                return 1
    print("profile selection and hash-mismatch guards passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ruff", type=Path, nargs="?", help="pinned Ruff executable")
    parser.add_argument("rules", nargs="*", help="optional rule IDs; default: every prepared profile")
    parser.add_argument("--profile", help="run one named combined prepared profile")
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

    output_directory = arguments.output_dir.resolve() if arguments.output_dir else None
    if output_directory is not None:
        output_directory.mkdir(parents=True, exist_ok=False)
        shutil.copyfile(MANIFEST, output_directory / "prepared.json")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    helper_path = Path(__file__).relative_to(REPOSITORY).as_posix()
    statuses = []
    try:
        verify_files(manifest, helper_path)
        if arguments.profile is not None:
            if arguments.rules:
                raise ValueError("rule IDs cannot be combined with a named combined profile")
            if arguments.profile not in manifest.get("combined_profiles", {}):
                raise ValueError(f"unknown prepared combined profile: {arguments.profile}")
            selections = [arguments.profile]
        else:
            profiles = manifest["profiles"]
            selected = arguments.rules or sorted(profiles)
            if len(set(selected)) != len(selected) or set(selected) - profiles.keys():
                raise ValueError("unknown or duplicate profile rule")
            selections = selected
        ruff = arguments.ruff.resolve()
        with contextlib.ExitStack() as stack:
            staged = []
            if arguments.profile is not None:
                descriptor = manifest["combined_profiles"][arguments.profile]
                staged.append(
                    stage_combined(arguments.profile, descriptor, manifest, stack, output_directory)
                )
            else:
                staged.extend(stage_binding(manifest, selections, stack, output_directory))
                if not arguments.rules:
                    for name, descriptor in sorted(manifest.get("combined_profiles", {}).items()):
                        staged.append(
                            stage_combined(name, descriptor, manifest, stack, output_directory)
                        )
            for name, profile_root in staged:
                command = command_for(ruff, profile_root, arguments.reference_only)
                identities = run_identity(manifest, ruff, profile_root)
                status = verify_profile(name, profile_root, command, output_directory, identities)
                print(f"{name}: verifier exit status {status}", file=sys.stderr)
                statuses.append((name, status))
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
                    "ruff_argument": str(arguments.ruff),
                    "reference_version": manifest.get("reference_version"),
                    "reference_sha256": manifest.get("reference_sha256"),
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
