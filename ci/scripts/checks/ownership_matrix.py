"""Fail closed on the existing Ryo ownership matrix and its executable controls."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


MATRIX = Path("crates/ryo/docs/contracts/README.md")
RELATIONS = Path("crates/ryo/crates/relations/src/domain/relations/service.rs")
OPERATIONS = Path("crates/ryo/crates/values/src/domain/plans/types.rs")
DERIVED = Path("crates/ryo/crates/derived/src/domain/contract/types.rs")
HEADERS = {
    "families": "Contract / route",
    "routes": "Route",
    "framework": "Framework prerequisite",
    "relations": "Relation member / source",
    "operators": "Operation member / source",
    "derived": "Derived member / source",
    "crates": "Crate / status",
}
MARKS = {"☑", "☐", "—"}
CONTROL = re.compile(r"\[controls\]\((evidence/contract-controls/[^)]+\.json)\)")
FAMILY_AXES = ("Scheduling", "Caching", "Invalidation", "Data ownership", "Lifetime")
MEMBER_AXES = ("Route enum", "Migration", "Engine owner", "TM", "Runtime", "Fable")


def cells(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def table(document: str, heading: str) -> list[list[str]]:
    lines = document.splitlines()
    matches = [index for index, line in enumerate(lines) if line.startswith("|") and cells(line)[0] == heading]
    if len(matches) != 1:
        raise ValueError(f"matrix table {heading!r}: expected one header, found {len(matches)}")
    start = matches[0]
    if start + 1 >= len(lines) or not lines[start + 1].startswith("| ---"):
        raise ValueError(f"matrix table {heading!r}: missing separator")
    rows = []
    for line in lines[start + 2 :]:
        if not line.startswith("|"):
            break
        rows.append(cells(line))
    if not rows:
        raise ValueError(f"matrix table {heading!r}: empty")
    return rows


def unique(names: list[str], label: str) -> set[str]:
    result = set(names)
    if len(names) != len(result):
        raise ValueError(f"{label}: duplicate declaration")
    return result


def source_members(root: Path) -> dict[str, set[str]]:
    relation_source = (root / RELATIONS).read_text()
    relation_block = relation_source.split("macro_rules! each_relation {", 1)[1].split("crate::each_relation!", 1)[0]
    relations = re.findall(r"^\s{16}([A-Z]\w+)\s+[a-z][a-z_]+\s+\w+\s+\w+;", relation_block, re.M)
    operation_source = (root / OPERATIONS).read_text()
    operation_block = operation_source.split("operation_kinds! {", 1)[1].split("\n}", 1)[0]
    operations = re.findall(r"^\s{4}([A-Z]\w+),\s*$", operation_block, re.M)
    derived_source = (root / DERIVED).read_text()
    derived_block = derived_source.split("kinds!(", 1)[1].split(");", 1)[0]
    derived = re.findall(r"^\s{4}([A-Z]\w+),?\s*$", derived_block, re.M)
    return {
        "relations": unique(relations, "relation catalog"),
        "operators": unique(operations, "operation catalog"),
        "derived": unique(derived, "derived catalog"),
    }


def row_name(cell: str) -> str:
    linked = re.match(r"\[([^]]+)\]", cell)
    if linked:
        return linked.group(1)
    if cell.startswith("`"):
        return cell.split("`", 2)[1]
    return cell


def source_digest(root: Path, revision: str) -> str:
    entries = []
    raw = subprocess.check_output(["git", "-C", str(root), "ls-tree", "-rz", revision])
    for record in raw.split(b"\0"):
        if not record:
            continue
        metadata, path = record.split(b"\t", 1)
        name = path.decode()
        if name.startswith("crates/ryo/docs/contracts/evidence/contract-controls/"):
            continue
        if name.startswith(("crates/ryo/", "ci/", "tastemaker/", ".github/", ".agents/")) or name in {"Cargo.toml", "Cargo.lock"}:
            entries.append((name, metadata.decode()))
    return hashlib.sha256(json.dumps(entries, sort_keys=True).encode()).hexdigest()


def workspace_crates(root: Path) -> set[str]:
    manifest = tomllib.loads((root / "crates/ryo/Cargo.toml").read_text())
    members = set()
    for pattern in manifest["workspace"]["members"]:
        for path in (root / "crates/ryo").glob(pattern):
            if (path / "Cargo.toml").is_file():
                package = tomllib.loads((path / "Cargo.toml").read_text())["package"]["name"]
                members.add("ryo-cli" if pattern == "cli" else package.removeprefix("idiomatic-sh-"))
    return members


def axes_for(section: str, row: list[str]) -> set[str]:
    if section == "families":
        return {axis for axis, mark in zip(FAMILY_AXES, row[2:7]) if mark != "—"}
    if section in ("framework", "relations", "operators", "derived"):
        return {axis for axis, mark in zip(MEMBER_AXES, row[2:8]) if mark != "—"}
    if section == "routes":
        return {"Enumeration"} if len(row) > 3 and row[3] != "—" else set()
    return {axis for axis, mark in zip(("Migration", "Ownership proof"), row[5:7]) if mark != "—"}


def controls(root: Path, row_id: str, row: list[str], axes: set[str], failures: list[str]) -> None:
    links = CONTROL.findall(" ".join(row))
    if len(links) != 1:
        failures.append(f"{row_id}: exactly one matrix-linked executed controls receipt required")
        return
    path = (root / MATRIX.parent / links[0]).resolve()
    if not path.is_relative_to((root / MATRIX.parent / "evidence/contract-controls").resolve()):
        failures.append(f"{row_id}: controls path escapes evidence directory")
        return
    try:
        receipt = json.loads(path.read_text())
        source = receipt["source"]
        if receipt["schema"] != "contract-controls/v1" or receipt["contract"] != row_id:
            raise ValueError("wrong schema or contract")
        if not re.fullmatch(r"[0-9a-f]{40}", source["head"]) or not re.fullmatch(r"[0-9a-f]{64}", source["sha256"]):
            raise ValueError("missing immutable source identity")
        subprocess.check_call(["git", "-C", str(root), "merge-base", "--is-ancestor", source["head"], "HEAD"])
        if source["sha256"] != source_digest(root, "HEAD") or source["sha256"] != source_digest(root, source["head"]):
            raise ValueError("stale source control")
        if set(receipt["proof"]) != axes:
            raise ValueError("proof does not map every applicable matrix cell")
        observed_kinds = set()
        for axis in axes:
            entries = receipt["proof"][axis]
            if not isinstance(entries, list) or not entries:
                raise ValueError(f"{axis}: no enforcement command")
            for proof in entries:
                kind = proof["kind"]
                if kind not in {"tm", "rust", "runtime"}:
                    raise ValueError(f"{axis}: unknown enforcement kind")
                observed_kinds.add(kind)
                for case in ("good", "bad"):
                    item = proof[case]
                    command = item["command"]
                    if not isinstance(command, list) or not command or not all(isinstance(arg, str) and arg for arg in command):
                        raise ValueError(f"{axis}/{kind}/{case}: missing exact command")
                    if item["exit"] != 0 or not re.fullmatch(r"[0-9a-f]{64}", item["output_sha256"]):
                        raise ValueError(f"{axis}/{kind}/{case}: control did not pass")
                    output = (root / MATRIX.parent / item["output"]).resolve()
                    if not output.is_relative_to((root / MATRIX.parent / "evidence/contract-controls").resolve()):
                        raise ValueError("control output escapes evidence directory")
                    content = output.read_bytes()
                    if not content or hashlib.sha256(content).hexdigest() != item["output_sha256"]:
                        raise ValueError("control output missing or changed")
                    if case == "bad" and item.get("forbidden_rejected") is not True:
                        raise ValueError("forbidden example did not reject the violation")
                    if case == "bad" and item.get("build_failed"):
                        raise ValueError("build failure is not a forbidden-call proof")
        if observed_kinds != {"tm", "rust", "runtime"}:
            raise ValueError("row lacks TM, Rust, or runtime proof")
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        failures.append(f"{row_id}: invalid executed controls: {error}")


def inspect(root: Path) -> list[str]:
    document = (root / MATRIX).read_text()
    tables = {name: table(document, header) for name, header in HEADERS.items()}
    authoritative = source_members(root)
    failures: list[str] = []
    families = [re.match(r"(\d+)[a-z]?\.", row[0]) for row in tables["families"]]
    if any(item is None for item in families) or {int(item.group(1)) for item in families if item} != set(range(1, 35)):
        failures.append("family matrix must enumerate every family 1–34")
    if len({row[0] for row in tables["families"]}) != len(tables["families"]):
        failures.append("family matrix has duplicate rows")
    route_ids = [row[0].split()[0] for row in tables["routes"]]
    expected_routes = {*(f"R{number}" for number in range(1, 6)),
                       *(f"O{number}" for number in range(1, 9)),
                       *(f"D{number}" for number in range(1, 9))}
    if unique(route_ids, "route inventory") != expected_routes:
        failures.append("route inventory must enumerate R1–R5, O1–O8, D1–D8")
    for category, expected in (("relations", 69), ("operators", 26), ("derived", 13)):
        rows = tables[category]
        names = unique([row_name(row[0]) for row in rows], f"{category} matrix")
        if len(names) != expected or names != authoritative[category]:
            failures.append(f"{category}: matrix must equal authoritative {expected}-member catalog")
    if len(tables["framework"]) != 7:
        failures.append("relation framework must enumerate all seven prerequisites")
    if not tables["crates"]:
        failures.append("capability crate inventory is empty")
    declared_crates = {row_name(row[0]) for row in tables["crates"]}
    actual_crates = workspace_crates(root)
    if declared_crates != actual_crates:
        failures.append(f"crate inventory differs from workspace: missing {sorted(actual_crates - declared_crates)}, stale {sorted(declared_crates - actual_crates)}")
    placed = tomllib.loads((root / "crates/ryo/tastemaker.toml").read_text())
    nuhex = {item.get("include", [None])[0] for item in placed["specs"] if item.get("spec") == "idiomatic.arch/nu-hex"}
    for member in actual_crates:
        if member in {"ryo-cli", "ryo-sdk"}:
            continue
        directory = member.removeprefix("ryo-")
        candidate = f"crates/{directory}"
        if (root / "crates/ryo" / candidate / "Cargo.toml").is_file() and candidate not in nuhex:
            failures.append(f"{member}: missing declared nu-hex placement")
    for section in ("families", "routes", "framework", "relations", "operators", "derived", "crates"):
        for row in tables[section]:
            row_id = f"{section}/{row_name(row[0])}"
            marks = [value for value in row if value in MARKS]
            if section == "families":
                if len(row) != 8 or len(marks) != 5:
                    failures.append(f"{row_id}: malformed five-axis matrix row")
            elif section in ("framework", "relations", "operators", "derived"):
                if len(row) != 8 or len(marks) != 6:
                    failures.append(f"{row_id}: malformed six-axis inventory row")
            elif section == "routes" and (len(row) != 4 or len(marks) != 1):
                failures.append(f"{row_id}: malformed route row")
            elif section == "crates" and len(row) != 7:
                failures.append(f"{row_id}: malformed crate row")
            if "☐" in marks or (section == "crates" and "☐" in row[-2:]):
                failures.append(f"{row_id}: ownership or enforcement remains unchecked")
            controls(root, row_id, row, axes_for(section, row), failures)
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    root = parser.parse_args().root.resolve()
    try:
        failures = inspect(root)
    except (IndexError, KeyError, OSError, ValueError) as error:
        failures = [str(error)]
    if failures:
        print(f"Ownership matrix gate failed: {len(failures)} missing or invalid obligations", file=sys.stderr)
        for failure in failures[:25]:
            print(f"  {failure}", file=sys.stderr)
        if len(failures) > 25:
            print(f"  … {len(failures) - 25} more", file=sys.stderr)
        return 1
    print("Ownership matrix and executed controls complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
