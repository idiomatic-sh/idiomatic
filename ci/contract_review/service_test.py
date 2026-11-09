"""Real Git-object and normal CLI controls; the review payloads here are synthetic fixtures."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess

import pytest

from ci.contract_review import service

ROOT = Path(__file__).resolve().parents[2]
COMMAND = ROOT / "ci/contract_review/command.py"
GOOD = [{"id": "1", "verdict": "PASS", "evidence": "x.rs:1"}]


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

    def review(source, **fields):
        return {"source": source, "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD, **fields}

    source = commit(service.ACTIVATION, {"crates/ryo/crates/core/review-control.txt": b"new engine behavior\n"})
    return tmp_path, source, commit, review


def run(path, head):
    return subprocess.run(["python3", str(COMMAND), "--root", str(path), "--head", head], capture_output=True, text=True)


def test_review_of_the_source_and_a_receipt_only_commit_pass(repository):
    path, source, commit, review = repository
    head = commit(source, {service.REVIEW: review(source)})
    done = run(path, head)
    assert done.returncode == 0 and "Contract review passed" in done.stdout


def test_no_protected_change_needs_no_review(repository):
    path, _, _, _ = repository
    assert run(path, service.ACTIVATION).returncode == 0


def test_missing_review_is_refused(repository):
    path, source, _, _ = repository
    done = run(path, source)
    assert done.returncode == 1 and "No valid exact-source contract review" in done.stderr


def test_stale_review_after_a_protected_change_is_refused(repository):
    path, source, commit, review = repository
    reviewed = commit(source, {service.REVIEW: review(source)})
    head = commit(reviewed, {"crates/ryo/crates/core/more.txt": b"unreviewed\n"})
    assert run(path, head).returncode == 1


@pytest.mark.parametrize("mutation", [
    lambda r: {**r, "verdict": "BLOCKING"},
    lambda r: {**r, "verdict": "MAYBE"},
    lambda r: {**r, "reviewer": "someone"},
    lambda r: {**r, "source": r["source"][:12]},
    lambda r: {**r, "checks": []},
    lambda r: {**r, "checks": ["x"]},
    lambda r: {**r, "checks": [1]},
    lambda r: {**r, "checks": [{"id": "1", "verdict": "PASS", "evidence": ""}]},
    lambda r: {**r, "checks": [{"id": "1", "verdict": "FAIL", "evidence": "e"}]},
    lambda r: {k: v for k, v in r.items() if k != "checks"},
])
def test_malformed_or_failing_review_is_refused(repository, mutation):
    path, source, commit, review = repository
    head = commit(source, {service.REVIEW: mutation(review(source))})
    done = run(path, head)
    assert done.returncode == 1 and "No valid exact-source contract review" in done.stderr
