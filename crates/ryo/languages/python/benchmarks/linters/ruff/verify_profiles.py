"""Materialize and verify prepared Ruff candidate profiles."""

from __future__ import annotations

import argparse
import contextlib
import collections
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


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
    paths = {helper_path}
    for rule in profiles:
        paths.update(
            {
                (PROFILE_DIR / rule / "benchmark.toml").relative_to(REPOSITORY).as_posix(),
                (RUFF_DIR / "rules" / rule / "candidate.ryoql").relative_to(REPOSITORY).as_posix(),
                (RUFF_DIR / "rules" / rule / "cases.toml").relative_to(REPOSITORY).as_posix(),
            }
        )
        if "oracle_file" in profiles[rule]:
            paths.add(profiles[rule]["oracle_file"])
    for descriptor in manifest.get("combined_profiles", {}).values():
        paths.update(descriptor["input_files"])
    return paths


def verify_files(manifest: dict[str, object], helper_path: str) -> None:
    files = manifest["files"]
    if set(files) != manifest_paths(manifest, helper_path):
        raise ValueError("prepared input paths do not match the candidate acceptance cohorts")
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


def command_for(ruff: Path, profile_root: Path) -> list[str]:
    return [
        "cargo",
        "bench",
        "--locked",
        "-p",
        "idiomatic-sh-ryo-python",
        "--bench",
        "ruff",
        "--",
        "verify-cases",
        str(ruff),
        str(profile_root),
    ]


def stage_binding(
    manifest: dict[str, object], selected: list[str], stack: contextlib.ExitStack
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
        cases_path = RUFF_DIR / "rules" / rule / "cases.toml"
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        recorded = profiles[rule]
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
        destination = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix=f"ryo-ruff-{rule}-")))
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        shutil.copyfile(profile, destination / "benchmark.toml")
        shutil.copyfile(candidate_path, rule_dir / "query.ryoql")
        shutil.copyfile(cases_path, rule_dir / "cases.toml")
        staged.append((rule, destination))
    return staged


def stage_combined(
    profile_name: str,
    descriptor: dict[str, object],
    manifest: dict[str, object],
    stack: contextlib.ExitStack,
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
    if (
        overlay.get("profile") != profile_name
        or overlay.get("reference_sha256") not in manifest["reference_sha256"]
        or overlay.get("selection_count") != len(selected_rules)
        or overlay.get("selected_rules_sha256")
        != hashlib.sha256(json.dumps(sorted(selected_rules), separators=(",", ":")).encode("utf-8")).hexdigest()
        or overlay.get("oracle_sha256") != descriptor["oracle_sha256"]
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
        if case.get("comparison", "rule") != "full":
            raise ValueError(f"{profile_name} overlay requires a full-report case: {key}")
        if item["cases_file"] != (RUFF_DIR / "rules" / item["rule"] / "cases.toml").relative_to(REPOSITORY).as_posix():
            raise ValueError(f"shape-110 overlay names a noncanonical case file: {key}")
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
            ):
                raise ValueError(f"invalid {profile_name} expected finding: {key}")
        original = collections.Counter(
            (entry["rule"], entry["file"], entry["start"], entry["end"])
            for entry in case["expected"]
        )
        replacement = collections.Counter(
            (entry["rule"], entry["file"], entry["start"], entry["end"])
            for entry in findings
        )
        if original - replacement:
            raise ValueError(f"{profile_name} overlay drops canonical findings: {key}")
        added_findings += sum((replacement - original).values())
        expected_findings += len(findings)
        overlays[key] = (item, case)
    if len(overlays) != overlay["changed_case_count"]:
        raise ValueError(f"{profile_name} overlay case count mismatch")
    if (
        added_findings != overlay["additional_diagnostic_count"]
        or expected_findings != overlay["expected_diagnostic_count"]
    ):
        raise ValueError(f"{profile_name} overlay diagnostic totals mismatch")

    inventory = []
    staged_cases: dict[str, list[dict[str, object]]] = {}
    for rule in selected_rules:
        source_dir = RUFF_DIR / "rules" / rule
        candidate = source_dir / "candidate.ryoql"
        query = candidate if candidate.is_file() else source_dir / "query.ryoql"
        cases_path = source_dir / "cases.toml"
        if not query.is_file() or not cases_path.is_file():
            raise ValueError(f"{profile_name} source is missing for {rule}")
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        staged_cases[rule] = cases
        for case in cases:
            comparison = case.get("comparison", "rule")
            inventory.append({"rule": rule, "name": case["name"], "comparison": comparison})
    counts = collections.Counter(
        item.get("comparison", "rule")
        for cases in staged_cases.values()
        for item in cases
    )
    if len(inventory) != descriptor["case_count"] or counts != descriptor["comparison_counts"]:
        raise ValueError(f"{profile_name} comparison inventory mismatch")
    inventory_hash = hashlib.sha256(
        json.dumps(inventory, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if inventory_hash != descriptor["inventory_sha256"]:
        raise ValueError(f"{profile_name} inventory hash mismatch")

    destination = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix=f"ryo-ruff-{profile_name}-")))
    shutil.copyfile(profile_path, destination / "benchmark.toml")
    for rule in selected_rules:
        rule_dir = destination / "rules" / rule
        rule_dir.mkdir(parents=True)
        source_dir = RUFF_DIR / "rules" / rule
        query_path = source_dir / ("candidate.ryoql" if (source_dir / "candidate.ryoql").is_file() else "query.ryoql")
        shutil.copyfile(query_path, rule_dir / "query.ryoql")
        cases = staged_cases[rule]
        for case in cases:
            overlay_entry = overlays.get((rule, case["name"]))
            if overlay_entry is not None:
                case["expected"] = overlay_entry[0]["expected"]
        write_cases(rule_dir / "cases.toml", cases)
    return profile_name, destination


def self_test() -> int:
    """Exercise fail-closed profile-selection and hash-mismatch guards without invoking Cargo."""
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-profile-guards-") as temporary:
        root = Path(temporary)
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
        last_profile = sorted(manifest["profiles"])[-1]
        manifest["profiles"][last_profile]["case_count"] += 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        fake_bin = root / "fake-bin"
        fake_bin.mkdir()
        marker = root / "cargo-was-started"
        fake_cargo = fake_bin / "cargo"
        fake_cargo.write_text(f"#!/bin/sh\nprintf started > {marker}\n", encoding="utf-8")
        fake_cargo.chmod(0o755)
        overlay_relative = pristine_manifest["combined_profiles"][SHAPE_PROFILE]["expectations_file"]
        baseline_overlay = (root / overlay_relative).read_bytes()
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
    print("profile selection and hash-mismatch guards passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ruff", type=Path, nargs="?", help="pinned Ruff executable")
    parser.add_argument("rules", nargs="*", help="optional rule IDs; default: every prepared profile")
    parser.add_argument("--profile", help="run one named combined prepared profile")
    parser.add_argument("--self-test", action="store_true", help="exercise fail-closed profile guards without running Cargo")
    arguments = parser.parse_args()
    if arguments.self_test:
        return self_test()
    if arguments.ruff is None:
        parser.error("the pinned Ruff executable is required unless --self-test is selected")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    helper_path = Path(__file__).relative_to(REPOSITORY).as_posix()
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
                staged.append(stage_combined(arguments.profile, descriptor, manifest, stack))
            else:
                staged.extend(stage_binding(manifest, selections, stack))
            statuses = []
            for name, profile_root in staged:
                try:
                    result = subprocess.run(
                        command_for(ruff, profile_root),
                        cwd=REPOSITORY / "crates" / "ryo",
                        check=False,
                    )
                    status = result.returncode
                except OSError as error:
                    print(f"{name}: could not start verifier: {error}", file=sys.stderr)
                    status = 1
                print(f"{name}: verifier exit status {status}", file=sys.stderr)
                statuses.append((name, status))
    except (KeyError, TypeError, ValueError, OSError) as error:
        parser.error(str(error))
    failed = [rule for rule, status in statuses if status != 0]
    if failed:
        print(f"candidate profiles failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
