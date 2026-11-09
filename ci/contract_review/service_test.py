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


def hook():
    import importlib.util
    spec = importlib.util.spec_from_file_location("guard_shell", ROOT / ".claude/hooks/guard_shell.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


SHA = "a" * 40
candidate_id = service.candidate_id
VARIANTS = [  # (reviewer, document, valid): the behaviour table; the hook calls service.shape, and one test ties the two
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, True),
    ("integrator", {"source": SHA, "verdict": "PASS", "reviewer": "integrator", "checks": GOOD}, True),
    ("contract-review", {"source": SHA, "verdict": "BLOCKING", "reviewer": "contract-review", "checks": GOOD}, True),
    ("contract-review", {"source": SHA, "verdict": "MAYBE", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA[:12], "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "integrator", "checks": GOOD}, False),
    ("integrator", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review"}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": []}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": ["x"]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [1]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review",
                         "checks": [{"id": "1", "verdict": "PASS", "evidence": " "}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review",
                         "checks": [{"id": "1", "verdict": "OK", "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review",
                         "checks": [{"id": "1", "verdict": "FAIL", "evidence": "e"}]}, False),
    ("contract-review", ["not", "an", "object"], False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": 1, "verdict": "PASS", "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": " ", "verdict": "PASS", "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": "PASS"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": "PASS", "evidence": 5}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": None, "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": "BLOCKING", "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA.upper(), "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": "claude/checkpoint-9", "verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"verdict": "PASS", "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA, "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA, "verdict": ["PASS"], "reviewer": "contract-review", "checks": GOOD}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": {"id": "1", "verdict": "PASS", "evidence": "e"}}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": "PASS", "evidence": "e"}, {}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": [{"id": "1", "verdict": "NOPE", "evidence": "e"}]}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": "contract-review", "checks": "none"}, False),
    ("contract-review", {"source": SHA, "verdict": "PASS", "reviewer": None, "checks": GOOD}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": []}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "compiler", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "strengthen no-pub-leak", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": "x"}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": ["x"]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "tool", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": "none"}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": "3", "text": "t"}]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": [], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("see ryo-ab12.3", "pub fn"), "class": "see ryo-ab12.3", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "t"}, 5]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": "c-00000000", "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "pub fn x() {}"}]}]}, False),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": []}, True),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "new", "mechanism": "rule@1", "plant_counts": [1]}]}, True),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "none", "mechanism": "", "plant_counts": [], "reason": "one-off class, no cost to enforce"}]}, True),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "strengthen", "mechanism": "rule@1", "plant_counts": [1, 0]}]}, False),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "new", "mechanism": "rule@1", "plant_counts": []}]}, False),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "none", "mechanism": "rule@1", "plant_counts": [], "reason": " "}]}, False),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "maybe", "mechanism": "rule@1", "plant_counts": [1]}]}, False),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": "x"}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": []}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub   fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "plants": [{"file": "a.rs", "line": 3, "text": "t"}]}]}, True),
    ("tm-steward", {"source": SHA, "verdict": "PASS", "reviewer": "tm-steward", "checks": GOOD, "candidate_decisions": [{"id": "c-1", "decision": "none", "mechanism": "", "plant_counts": [], "reason": "one-off"}]}, False),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "instance_status": "fixed", "plants": [{"file": "a.rs", "line": 3, "text": "t"}]}]}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "instance_status": "open", "plants": [{"file": "a.rs", "line": 3, "text": "t"}]}]}, True),
    ("leak-review", {"source": SHA, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD, "enforcement_candidates": [{"id": candidate_id("c", "pub fn"), "class": "c", "signature": "pub fn", "scope": ["crates/a"], "suggested_mechanism": "new rule", "instance_status": "gone", "plants": [{"file": "a.rs", "line": 3, "text": "t"}]}]}, False),
]


@pytest.mark.parametrize("reviewer,document,valid", VARIANTS)
def test_the_ci_and_hook_validators_agree_on_every_variant(reviewer, document, valid):
    guard = hook()
    hook_ok = guard.review_shape(document, reviewer) is None
    try:
        service.shape(document, reviewer)
        ci_ok = True
    except ValueError:
        ci_ok = False
    assert hook_ok == ci_ok == valid


def test_a_malformed_asymptote_record_is_refused(repository):
    path, source, commit, review = repository
    bad = {"source": source, "verdict": "PASS", "reviewer": "someone", "checks": GOOD}
    head = commit(source, {service.REVIEW: review(source), service.ASYMPTOTE: bad})
    assert run(path, head).returncode == 1
    good = {**bad, "reviewer": "integrator"}
    head = commit(source, {service.REVIEW: review(source), service.ASYMPTOTE: good})
    assert run(path, head).returncode == 0
