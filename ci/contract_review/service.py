"""Prospective paired review of the exact protected source, independent of receipt commits."""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

# Earlier work is NOT certified by introducing this gate.
ACTIVATION = "a402537b63a0a96ffd4ff973be610642edf8c26a"
ENGINE = tuple(f"crates/ryo/crates/{name}/" for name in (
    "core", "operators", "derived", "relations", "values", "query_language", "language",
))
CONTROL = ("ci/contract_review/", ".agents/agents/", ".github/workflows/")
EXACT = {"ci/workflows_test.py", "crates/ryo/Cargo.toml", "crates/ryo/Cargo.lock",
         "crates/ryo/docs/contracts/README.md", "crates/ryo/docs/language-contract.md"}
RECEIPTS = "crates/ryo/docs/contracts/evidence/paired-reviews"


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE)


def protected(path: str) -> bool:
    return (path in EXACT or path.startswith(ENGINE + CONTROL)
            or (path.startswith("crates/ryo/languages/") and "/src/" in path)
            or (path.startswith("crates/ryo/docs/contracts/") and not path.startswith(RECEIPTS + "/")))


def inventory(root: Path, revision: str) -> dict[str, str]:
    result = {}
    for record in git(root, "ls-tree", "-rz", revision).split(b"\0"):
        if record:
            metadata, name = record.split(b"\t", 1)
            path = name.decode()
            if protected(path):
                # Include mode and object type: symlinks/submodules are not ordinary files.
                result[path] = metadata.decode()
    return result


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def source_digest(root: Path, revision: str) -> str:
    return digest(json.dumps(inventory(root, revision), sort_keys=True).encode())


def object_json(root: Path, revision: str, path: str) -> dict:
    value = json.loads(git(root, "show", f"{revision}:{path}"))
    if not isinstance(value, dict):
        raise ValueError(f"expected object: {path}")
    return value


def evidence(root: Path, revision: str, record: dict) -> bytes:
    path = record["path"]
    if not isinstance(path, str) or not path.startswith(RECEIPTS + "/") or ".." in Path(path).parts:
        raise ValueError("evidence must be committed beneath paired-reviews")
    content = git(root, "show", f"{revision}:{path}")
    if digest(content) != record["sha256"]:
        raise ValueError(f"evidence digest mismatch: {path}")
    return content


def review(root: Path, head: str, source: dict, item: dict, identity: tuple[str, str]) -> None:
    cli, model = identity
    if item["cli"] != cli or not item["session_id"]:
        raise ValueError(f"wrong reviewer identity: {cli}/{model}")
    if (cli == "codex" and item["model"] != model) or (cli == "claude" and not item["model"].startswith("claude-opus-")):
        raise ValueError(f"wrong reviewer identity: {cli}/{model}")
    if item["source"] != source or item["before"] != item["after"]:
        raise ValueError("review changed source or covers another source")
    if item["before"] != {"head": source["head"], "status": ""} or item["exit"] != 0:
        raise ValueError("review must succeed on clean immutable source")
    model = item["model"]
    invocation = json.loads(evidence(root, head, item["invocation"]))
    if any(invocation.get(key) != item[key] for key in ("cli", "model", "session_id", "exit", "before", "after")):
        raise ValueError("invocation identity does not match review")
    if not invocation.get("command"):
        raise ValueError("missing actual invocation")
    raw = json.loads(evidence(root, head, item["result"]))
    if raw.get("subtype") != "success" or raw.get("is_error") is True or raw.get("session_id") != item["session_id"]:
        raise ValueError("failed or unrelated reviewer result")
    if cli == "claude" and model not in raw.get("modelUsage", {}):
        raise ValueError("Opus actual model is not in CLI result")
    if cli == "codex" and raw.get("model") != model:
        raise ValueError("Astra actual model is not in session result")
    result = raw.get("result", "")
    verdict = re.search(r"^Verdict: (pass|blocking)\s*$", result, re.M)
    if not verdict:
        raise ValueError("missing assessed verdict")
    # Every numbered finding in the actual reviewer table needs a disposition.
    ids = re.findall(r"^\|\s*([A-Za-z]*\d+)\s*\|\s*blocking\s*\|", result, re.M | re.I)
    dispositions = item["findings"]
    if len(ids) != len(set(ids)) or set(dispositions) != set(ids):
        raise ValueError("review findings omitted, duplicated or invented")
    if (verdict[1] == "blocking") != bool(ids):
        raise ValueError("verdict and blocking findings disagree")
    for finding in dispositions.values():
        if finding["state"] not in ("fixed", "rejected") or not finding["reason"].strip():
            raise ValueError("unresolved blocking finding")
        if not evidence(root, head, finding["proof"]).strip():
            raise ValueError("missing finding disposition proof")


def validate(root: Path, head: str = "HEAD") -> str:
    head = git(root, "rev-parse", f"{head}^{{commit}}").decode().strip()
    git(root, "merge-base", "--is-ancestor", ACTIVATION, head)
    changed = [path.decode() for path in git(root, "diff", "--name-only", "-z", ACTIVATION, head).split(b"\0") if path]
    if not any(protected(path) for path in changed):
        return "No protected changes since activation; historical work is not certified."
    expected = source_digest(root, head)
    candidates = [p.decode() for p in git(root, "ls-tree", "-rz", "--name-only", head, RECEIPTS).split(b"\0") if p.endswith(b".review.json")]
    errors = []
    for path in candidates:
        try:
            artifact = object_json(root, head, path)
            if artifact["schema"] != "paired-contract-review/v1":
                raise ValueError("unknown review schema")
            source = artifact["source"]
            if not re.fullmatch(r"[0-9a-f]{40}", source["head"]) or not re.fullmatch(r"[0-9a-f]{40}", source["base"]):
                raise ValueError("source must use full commit identities")
            git(root, "merge-base", "--is-ancestor", source["base"], source["head"])
            if source["tree"] != git(root, "rev-parse", f'{source["head"]}^{{tree}}').decode().strip():
                raise ValueError("wrong reviewed tree")
            if source["diff_sha256"] != digest(git(root, "diff", "--binary", source["base"], source["head"])):
                raise ValueError("wrong reviewed diff")
            if source["protected_sha256"] != expected or source_digest(root, source["head"]) != expected:
                raise ValueError("stale source review")
            for key in ("claim", "matrix", "receipts"):
                if not evidence(root, head, artifact[key]).strip():
                    raise ValueError(f"missing {key}")
            items = artifact["reviews"]
            if len(items) != 2:
                raise ValueError("both Astra and Opus reviews are required")
            review(root, head, source, items[0], ("codex", "gpt-6-astra"))
            review(root, head, source, items[1], ("claude", "opus"))
            return f"Paired contract review passed: {path}; protected source {expected}"
        except (KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
            errors.append(f"{path}: {error}")
    raise ValueError("No valid exact-source Astra/Opus review. " + "; ".join(errors))
