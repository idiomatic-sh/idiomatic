"""Real Git-object and normal CLI controls; reviewer payloads here are synthetic fixtures."""
from __future__ import annotations

from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess

import pytest

from ci.contract_review import service

ROOT = Path(__file__).resolve().parents[2]
COMMAND = ROOT / "ci/contract_review/command.py"


@pytest.fixture
def repository(tmp_path: Path):
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    objects = service.git(ROOT, "rev-parse", "--path-format=absolute", "--git-path", "objects").decode().strip()
    (tmp_path / ".git/objects/info/alternates").write_text(objects + "\n")
    env = dict(os.environ, GIT_AUTHOR_NAME="Review control", GIT_AUTHOR_EMAIL="test@example.invalid",
               GIT_COMMITTER_NAME="Review control", GIT_COMMITTER_EMAIL="test@example.invalid")

    def commit(parent, files):
        service.git(tmp_path, "read-tree", parent)
        for path, value in files.items():
            data = value if isinstance(value, bytes) else json.dumps(value).encode()
            blob = subprocess.check_output(["git", "-C", str(tmp_path), "hash-object", "-w", "--stdin"], input=data).decode().strip()
            service.git(tmp_path, "update-index", "--add", "--cacheinfo", f"100644,{blob},{path}")
        tree = service.git(tmp_path, "write-tree").decode().strip()
        return subprocess.check_output(["git", "-C", str(tmp_path), "commit-tree", tree, "-p", parent], input=b"synthetic review control\n", env=env).decode().strip()

    source_head = commit(service.ACTIVATION, {"crates/ryo/crates/core/review-control.txt": b"new engine behavior\n"})
    source = {"base": service.ACTIVATION, "head": source_head,
              "tree": service.git(tmp_path, "rev-parse", f"{source_head}^{{tree}}").decode().strip(),
              "diff_sha256": service.digest(service.git(tmp_path, "diff", "--binary", service.ACTIVATION, source_head)),
              "protected_sha256": service.source_digest(tmp_path, source_head)}
    files = {}

    def evidence(name, value):
        path = service.RECEIPTS + "/control/" + name
        data = json.dumps(value).encode()
        files[path] = data
        return {"path": path, "sha256": service.digest(data)}

    reviews = []
    for cli, model in (("codex", "gpt-6-astra"), ("claude", "claude-opus-5-5")):
        session = "synthetic-fixture-" + cli
        state = {"head": source_head, "status": ""}
        invocation = {"cli": cli, "model": model, "session_id": session, "before": state, "after": state,
                      "exit": 0, "command": [cli, "synthetic control; not actual approval", source["base"], source["head"], source["tree"]]}
        result = {"subtype": "success", "is_error": False, "session_id": session, "model": model,
                  "modelUsage": {model: {}}, "uuid": "synthetic-round-" + cli, "result": f'Source: base={source["base"]} head={source["head"]} tree={source["tree"]}\nVerdict: pass\n\n| # | severity | finding |\n|---|---|---|\n'}
        reviews.append({**invocation, "source": source, "round_id": result["uuid"], "invocation": evidence(cli + "-invocation.json", invocation),
                        "result": evidence(cli + "-result.json", result), "findings": {}})
    artifact = {"schema": "paired-contract-review/v1", "source": source, "reviews": reviews, "dispositions": {},
                **{key: evidence(key + ".json", {"fixture": key}) for key in ("claim", "matrix", "receipts")}}
    artifact_path = service.RECEIPTS + "/control/approved.review.json"
    return tmp_path, commit, source_head, files, artifact, artifact_path


def run(root, head):
    return subprocess.run(["python3", str(COMMAND), "--root", str(root), "--head", head], text=True, capture_output=True)


def test_normal_gate_accepts_exact_pair_and_receipt_only_commit(repository):
    root, commit, source, files, artifact, path = repository
    approved = commit(source, {**files, path: artifact})
    result = run(root, approved)
    assert result.returncode == 0, result.stderr
    assert "Paired contract review passed" in result.stdout
    assert run(root, commit(approved, {service.RECEIPTS + "/notes.txt": b"receipt only"})).returncode == 0


@pytest.mark.parametrize("mutation", ["missing", "stale", "missing-opus", "wrong-model", "changed-source", "transport", "tampered-result", "unresolved", "omitted-finding", "wrong-tree", "wrong-diff", "missing-proof", "gate-edit", "sdk-edit", "partial-base", "self-rejected", "hyphen-id", "formatted-blocker", "duplicate-verdict", "relabelled-result", "wrong-round"])
def test_normal_gate_rejects_missing_stale_and_unresolved_reviews(repository, mutation):
    root, commit, source, files, original, path = repository
    artifact = deepcopy(original)
    review = artifact["reviews"][1]
    if mutation == "missing":
        assert run(root, source).returncode == 1
        return
    if mutation == "missing-opus":
        artifact["reviews"].pop()
    elif mutation == "wrong-model":
        review["model"] = "claude-fable-5-1"
    elif mutation == "changed-source":
        review["after"]["status"] = " M source.rs"
    elif mutation in ("wrong-tree", "wrong-diff"):
        artifact["source"]["tree" if mutation == "wrong-tree" else "diff_sha256"] = "0" * (40 if mutation == "wrong-tree" else 64)
    elif mutation == "missing-proof":
        files.pop(artifact["receipts"]["path"])
    elif mutation == "partial-base":
        artifact["source"]["base"] = source
        artifact["source"]["diff_sha256"] = service.digest(service.git(root, "diff", "--binary", source, source))
    elif mutation == "wrong-round":
        review["round_id"] = "another-round"
    elif mutation in ("transport", "tampered-result", "unresolved", "omitted-finding", "self-rejected", "hyphen-id", "formatted-blocker", "duplicate-verdict", "relabelled-result"):
        record = review["result"]
        result = json.loads(files[record["path"]])
        if mutation == "transport":
            result["subtype"] = "error"
        elif mutation == "tampered-result":
            result["result"] = "Verdict: Unassessed"
        elif mutation == "relabelled-result":
            result["result"] = result["result"].replace(source, service.ACTIVATION)
        elif mutation == "duplicate-verdict":
            result["result"] += "Verdict: blocking\n"
        elif mutation in ("hyphen-id", "formatted-blocker"):
            severity = "**blocking**" if mutation == "formatted-blocker" else "blocking"
            result["result"] += f"| SC-1 | {severity} | source.rs:1 | wrong output |\n"
        else:
            result["result"] = result["result"].replace("Verdict: pass", "Verdict: blocking") + "| F1 | blocking | source.rs:1 | wrong output |\n"
            if mutation == "self-rejected":
                review["findings"] = {"F1": {"state": "rejected", "reason": "author says fine", "proof": artifact["claim"]}}
            if mutation == "unresolved":
                review["findings"] = {"F1": {"state": "open", "reason": "pending"}}
        files[record["path"]] = json.dumps(result).encode()
        if mutation != "tampered-result":
            record["sha256"] = service.digest(files[record["path"]])
    approved = commit(source, {**files, path: artifact})
    if mutation in ("stale", "gate-edit", "sdk-edit"):
        changed = "crates/ryo/crates/core/review-control.txt" if mutation == "stale" else "ci/contract_review/command.py"
        if mutation == "sdk-edit":
            changed = "crates/ryo/crates/sdk/src/review_control.rs"
        approved = commit(approved, {changed: b"changed after review"})
    result = run(root, approved)
    assert result.returncode == 1, result.stdout
    assert "No valid exact-source" in result.stderr


def test_malformed_candidate_does_not_hide_valid_approval(repository):
    root, commit, source, files, artifact, path = repository
    malformed = deepcopy(artifact)
    malformed["reviews"][1]["model"] = []
    approved = commit(source, {**files, path: artifact, service.RECEIPTS + "/000.review.json": malformed})
    result = run(root, approved)
    assert result.returncode == 0, result.stderr
