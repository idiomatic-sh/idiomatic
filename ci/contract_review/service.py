"""Exact-source contract review: the committed review evidence must cover the protected source of the head."""
from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

# Earlier work is NOT certified by introducing this gate.
ACTIVATION = "a402537b63a0a96ffd4ff973be610642edf8c26a"
PROTECTED = ("crates/ryo/", "ci/", "tools/", ".github/", ".cargo/", ".agents/agents/")
EXACT = {"rust-toolchain.toml", "Cargo.toml", "Cargo.lock", "asymptote.toml"}
RECEIPTS = "crates/ryo/docs/contracts/evidence"
REVIEW = RECEIPTS + "/checkpoint/contract-review.json"
ASYMPTOTE = RECEIPTS + "/checkpoint/asymptote.json"
VERDICTS = ("PASS", "FAIL", "NOTE", "BLOCKING")


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE)


def protected(path: str) -> bool:
    return not path.startswith(RECEIPTS + "/") and (path in EXACT or path.startswith(PROTECTED))


def shape(review: object, reviewer: str = "contract-review") -> None:
    """Raise unless review has the shared schema in crates/ryo/docs/contracts/review/README.md#review-evidence-schema."""
    if not isinstance(review, dict):
        raise ValueError("review is not a JSON object")
    if review.get("verdict") not in VERDICTS:
        raise ValueError("review has no known verdict")
    if not isinstance(review.get("source"), str) or not re.fullmatch(r"[0-9a-f]{40}", review["source"]):
        raise ValueError("review source must be a full 40-character sha")
    if review.get("reviewer") != reviewer:
        raise ValueError(f"review is not by {reviewer}")
    checks = review.get("checks")
    if not isinstance(checks, list) or not checks:
        raise ValueError("review has no non-empty checks list")
    for check in checks:
        if not isinstance(check, dict) or not isinstance(check.get("id"), str) or not check["id"].strip():
            raise ValueError("a check is not an object with a string id")
        if check.get("verdict") not in VERDICTS:
            raise ValueError(f"check {check['id']} has an unknown verdict")
        if not isinstance(check.get("evidence"), str) or not check["evidence"].strip():
            raise ValueError(f"check {check['id']} has no evidence")
        if review["verdict"] == "PASS" and check["verdict"] in ("FAIL", "BLOCKING"):
            raise ValueError(f"review is PASS though check {check['id']} is {check['verdict']}")


def validate(root: Path, head: str = "HEAD") -> str:
    head = git(root, "rev-parse", f"{head}^{{commit}}").decode().strip()
    git(root, "merge-base", "--is-ancestor", ACTIVATION, head)
    changed = [path.decode() for path in git(root, "diff", "--name-only", "-z", ACTIVATION, head).split(b"\0") if path]
    if not any(protected(path) for path in changed):
        return "No protected changes since activation; historical work is not certified."
    try:
        try:
            recorded = json.loads(git(root, "show", f"{head}:{ASYMPTOTE}"))
        except subprocess.CalledProcessError:
            recorded = None
        if recorded is not None:
            shape(recorded, "integrator")
        review = json.loads(git(root, "show", f"{head}:{REVIEW}"))
        shape(review)
        if review["verdict"] != "PASS":
            raise ValueError(f"review verdict is {review['verdict']}, not PASS")
        source = review["source"]
        git(root, "merge-base", "--is-ancestor", source, head)
        since = [p.decode() for p in git(root, "diff", "--name-only", "-z", source, head).split(b"\0") if p]
        if any(protected(path) for path in since):
            raise ValueError("protected source changed after the reviewed source " + source)
    except (KeyError, ValueError, subprocess.CalledProcessError) as error:
        raise ValueError(f"No valid exact-source contract review. {REVIEW}: {error}") from error
    return f"Contract review passed: {REVIEW} covers {head}"
