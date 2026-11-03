"""What the git adapter of `manifest` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.tag.manifest.adapters.git.adapter import GitManifest
from ci.tag.manifest.interface_test import conforms


def test_the_git_adapter_implements_the_port() -> None:
    """It is a `manifest` adapter."""
    conforms(GitManifest)


def test_the_version_is_the_workspace_s_at_the_commit(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        return subprocess.CompletedProcess(argv, 0, '[workspace]\nmembers = ["cli"]\n\n[workspace.package]\nversion = "0.2.0"\n')

    monkeypatch.setattr(subprocess, "run", run)
    assert GitManifest(Path("/repo")).version("ryo", "HEAD^") == "0.2.0"
    assert calls == [(["git", "show", "HEAD^:crates/ryo/Cargo.toml"], Path("/repo"))]


def test_a_commit_without_the_manifest_or_its_version_has_none(monkeypatch) -> None:
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 128, "", "fatal: path does not exist"))
    assert GitManifest(Path("/repo")).version("ryo", "HEAD^") is None
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, '[workspace]\nmembers = ["cli"]\n'))
    assert GitManifest(Path("/repo")).version("ryo", "HEAD") is None
