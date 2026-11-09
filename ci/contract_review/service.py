"""Exact-source contract review: the committed review evidence must cover the protected source of the head."""
from __future__ import annotations

import hashlib
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
    if "enforcement_candidates" in review:
        candidates(review["enforcement_candidates"])
    if "candidate_decisions" in review:
        decided(review["candidate_decisions"])


TRACKER = re.compile(r"\bryo-[a-z0-9]{3,}(\.[0-9]+)*\b")
MECHANISMS = re.compile(r"compiler|new rule|strengthen \S.*")


def candidate_id(klass: str, signature: str) -> str:
    """The stable id of a candidate: the class as a slug, then the first 8 hex of the sha256 of the signature with whitespace normalised."""
    slug = re.sub(r"[^a-z0-9]+", "-", klass.lower()).strip("-")
    return slug + "-" + hashlib.sha256(" ".join(signature.split()).encode()).hexdigest()[:8]


def candidates(listing: object) -> None:
    """Raise unless listing is a valid enforcement_candidates list (review README, review evidence schema)."""
    if not isinstance(listing, list):
        raise ValueError("enforcement_candidates is not a list")
    for entry in listing:
        if not isinstance(entry, dict):
            raise ValueError("an enforcement candidate is not an object")
        for key in ("class", "signature", "suggested_mechanism"):
            if not isinstance(entry.get(key), str) or not entry[key].strip():
                raise ValueError(f"an enforcement candidate has no string {key}")
        if not MECHANISMS.fullmatch(entry["suggested_mechanism"]):
            raise ValueError("suggested_mechanism is not compiler, strengthen <rule id> or new rule")
        if entry.get("id") != candidate_id(entry["class"], entry["signature"]):
            raise ValueError("an enforcement candidate id is not candidate_id(class, signature)")
        scope = entry.get("scope")
        if not isinstance(scope, list) or not scope or not all(isinstance(p, str) and p.strip() for p in scope):
            raise ValueError("an enforcement candidate scope is not a non-empty list of paths")
        plants = entry.get("plants")
        if not isinstance(plants, list) or not plants:
            raise ValueError("an enforcement candidate plants is not a non-empty list")
        for plant in plants:
            if not (isinstance(plant, dict) and isinstance(plant.get("file"), str) and plant["file"].strip()
                    and isinstance(plant.get("line"), int) and not isinstance(plant["line"], bool)
                    and isinstance(plant.get("text"), str)):
                raise ValueError("a plant is not {file, line, text}")
        if TRACKER.search(json.dumps(entry)):
            raise ValueError("an enforcement candidate carries a tracker id")


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


DECISIONS = ("compiler", "strengthen", "new", "none")


def decided(listing: object) -> None:
    """Raise unless listing is a valid candidate_decisions list (review README, review evidence schema)."""
    if not isinstance(listing, list):
        raise ValueError("candidate_decisions is not a list")
    for entry in listing:
        if not isinstance(entry, dict) or not isinstance(entry.get("id"), str) or not entry["id"].strip():
            raise ValueError("a candidate decision is not an object with a string id")
        if entry.get("decision") not in DECISIONS:
            raise ValueError(f"candidate {entry['id']} has an unknown decision")
        counts = entry.get("plant_counts")
        if not isinstance(counts, list) or not all(isinstance(c, int) and not isinstance(c, bool) for c in counts):
            raise ValueError(f"candidate {entry['id']} plant_counts is not a list of integers")
        if entry["decision"] == "none":
            reason = entry.get("reason")
            if not isinstance(reason, str) or len(reason.strip()) < 20 or len(reason.split()) < 3:
                raise ValueError(f"candidate {entry['id']} is decided none without a real reason "
                                 "(a sentence of at least 20 characters and 3 words)")
            continue
        if not isinstance(entry.get("mechanism"), str) or not entry["mechanism"].strip():
            raise ValueError(f"candidate {entry['id']} has no mechanism")
        if not counts or any(c < 1 for c in counts):
            raise ValueError(f"candidate {entry['id']} needs plant counts of at least 1 for decision {entry['decision']}")


def candidate_ids(review: object) -> dict:
    """The enforcement candidates a leak-review evidence file carries (shape already validated): id -> plant count."""
    listing = review.get("enforcement_candidates") if isinstance(review, dict) else None
    return {entry["id"]: len(entry["plants"]) for entry in listing} if isinstance(listing, list) else {}


def undecided(steward: object, ids: list) -> list:
    """The candidate ids with no decision in the tm-steward evidence."""
    listing = steward.get("candidate_decisions") if isinstance(steward, dict) else None
    have = {entry.get("id") for entry in listing} if isinstance(listing, list) else set()
    return [i for i in ids if i not in have]


def plant_mismatch(steward: object, plants: dict) -> list:
    """The ids decided other than none whose plant_counts do not number the candidate's plants one for one."""
    listing = steward.get("candidate_decisions") if isinstance(steward, dict) else None
    out = []
    for entry in listing if isinstance(listing, list) else []:
        if entry.get("id") in plants and entry.get("decision") != "none" and len(entry.get("plant_counts")) != plants[entry["id"]]:
            out.append(entry["id"])
    return out
