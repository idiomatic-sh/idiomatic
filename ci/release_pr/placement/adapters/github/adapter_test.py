"""What the GitHub adapter of `placement` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import base64
import json
import subprocess
from pathlib import Path

from ci.release_pr.placement.adapters.github.adapter import GithubPlacement
from ci.release_pr.placement.interface_test import conforms

TEXT = 'tastemaker = "0.1.0"\n'
SHOWN = json.dumps({"sha": "blob", "content": base64.b64encode(TEXT.encode()).decode()})


def test_the_github_adapter_implements_the_port() -> None:
    """It is a `placement` adapter."""
    conforms(GithubPlacement)


def test_the_file_is_read_from_the_branch(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        return subprocess.CompletedProcess(argv, 0, SHOWN)

    monkeypatch.setattr(subprocess, "run", run)
    assert GithubPlacement(Path("/repo")).read("ryo", "release-plz-ryo-1") == TEXT
    assert calls == [(["gh", "api", "repos/{owner}/{repo}/contents/crates/ryo/tastemaker.toml?ref=release-plz-ryo-1"], Path("/repo"))]


def test_the_file_is_committed_on_the_branch_over_the_blob_it_replaces(monkeypatch) -> None:
    calls = []

    def run(argv, **_):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0, SHOWN)

    monkeypatch.setattr(subprocess, "run", run)
    GithubPlacement(Path("/repo")).write("ryo", "release-plz-ryo-1", "new\n", "chore(ryo): release 0.1.3")
    assert calls[1] == [
        "gh", "api", "--method", "PUT", "repos/{owner}/{repo}/contents/crates/ryo/tastemaker.toml",
        "-f", "message=chore(ryo): release 0.1.3",
        "-f", f"content={base64.b64encode(b'new' + bytes([10])).decode()}",
        "-f", "sha=blob",
        "-f", "branch=release-plz-ryo-1",
    ]  # fmt: skip
