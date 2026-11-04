"""Prospective paired review of the exact protected source, independent of receipt commits."""
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
RECEIPTS = "crates/ryo/docs/contracts/evidence/paired-reviews"


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE)


def protected(path: str) -> bool:
    return not path.startswith(RECEIPTS + "/") and (path in EXACT or path.startswith(PROTECTED))


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
    command = invocation.get("command")
    if not isinstance(command, list) or not all(isinstance(arg, str) for arg in command):
        raise ValueError("missing actual invocation command")
    if any(source[key] not in " ".join(command) for key in ("base", "head", "tree")):
        raise ValueError("invocation did not name the exact reviewed source")
    raw = json.loads(evidence(root, head, item["result"]))
    if raw.get("subtype") != "success" or raw.get("is_error") is True or raw.get("session_id") != item["session_id"]:
        raise ValueError("failed or unrelated reviewer result")
    if not item["round_id"] or raw.get("uuid") != item["round_id"]:
        raise ValueError("review round identity mismatch")
    if cli == "claude" and model not in raw.get("modelUsage", {}):
        raise ValueError("Opus actual model is not in CLI result")
    if cli == "codex" and raw.get("model") != model:
        raise ValueError("Astra actual model is not in session result")
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError("duplicate verdict field")
            value[key] = item
        return value

    verdict = json.loads(raw["result"], object_pairs_hook=unique)
    required = {"schema", "source", "verdict", "findings", "dispositions"}
    if not isinstance(verdict, dict) or set(verdict) != required or verdict["schema"] != "contract-review-verdict/v1":
        raise ValueError("missing or unrecognized structured review verdict")
    if verdict["source"] != {key: source[key] for key in ("base", "head", "tree")}:
        raise ValueError("raw reviewer result does not bind exact source")
    if verdict["verdict"] not in ("pass", "blocking", "unassessed") or not isinstance(verdict["findings"], list):
        raise ValueError("invalid verdict or findings schema")
    for finding in verdict["findings"]:
        if set(finding) != {"id", "severity", "file", "failure", "required_proof"} or finding["severity"] != "blocking":
            raise ValueError("unrecognized finding schema or severity")
        if not all(isinstance(value, str) and value.strip() for value in finding.values()):
            raise ValueError("empty or unparseable finding")
    if not isinstance(verdict["dispositions"], list):
        raise ValueError("invalid disposition schema")
    for disposition in verdict["dispositions"]:
        if set(disposition) != {"id", "state", "reason", "evidence"} or disposition["state"] not in ("fixed", "rejected"):
            raise ValueError("unresolved disposition")
        if not all(isinstance(value, str) and value.strip() for value in disposition.values()):
            raise ValueError("empty or unparseable disposition")
    # Author dispositions never turn a blocking reviewer result into approval.
    if verdict["verdict"] != "pass" or verdict["findings"] or item["findings"] != {}:
        raise ValueError("both reviewers must pass this exact source without unresolved findings")


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
            if source["base"] != ACTIVATION:
                raise ValueError("review must cover every change since activation")
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
            for finding in artifact["dispositions"].values():
                if finding["state"] not in ("fixed", "rejected") or not finding["reason"].strip() or not evidence(root, head, finding["proof"]).strip():
                    raise ValueError("unresolved or unproved historical finding")
            items = artifact["reviews"]
            if len(items) != 2:
                raise ValueError("both Astra and Opus reviews are required")
            review(root, head, source, items[0], ("codex", "gpt-6-astra"))
            review(root, head, source, items[1], ("claude", "opus"))
            return f"Paired contract review passed: {path}; protected source {expected}"
        except (AttributeError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
            errors.append(f"{path}: {error}")
    raise ValueError("No valid exact-source Astra/Opus review. " + "; ".join(errors))
