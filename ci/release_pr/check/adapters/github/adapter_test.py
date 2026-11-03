"""What the GitHub adapter of `check` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release_pr.check.adapters.github.adapter import GithubCheck
from ci.release_pr.check.interface_test import conforms


def test_the_github_adapter_implements_the_port() -> None:
    """It is a `check` adapter."""
    conforms(GithubCheck)


def test_the_check_workflow_is_dispatched_on_the_branch_for_the_product(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        return subprocess.CompletedProcess(argv, 0)

    monkeypatch.setattr(subprocess, "run", run)
    GithubCheck(Path("/repo")).start("tastemaker", "release-plz-tastemaker-1")
    assert calls == [(["gh", "workflow", "run", "check.yml", "--ref", "release-plz-tastemaker-1", "-f", "product=tastemaker"], Path("/repo"))]
