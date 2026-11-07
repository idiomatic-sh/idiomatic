"""What the GitHub adapter of `tags` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.tag.tags.adapters.github.adapter import GithubTags
from ci.tag.tags.interface_test import conforms


def test_the_github_adapter_implements_the_port() -> None:
    """It is a `tags` adapter."""
    conforms(GithubTags)


def test_a_tag_exists_only_when_a_ref_is_named_for_it_exactly(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        # GitHub matches refs by prefix: v0.1.1 also lists v0.1.10.
        return subprocess.CompletedProcess(argv, 0, '[{"ref": "refs/tags/idiomatic-sh-ryo-v0.1.10"}]')

    monkeypatch.setattr(subprocess, "run", run)
    tags = GithubTags(Path("/repo"))
    assert tags.exists("idiomatic-sh-ryo-v0.1.1") is False
    assert tags.exists("idiomatic-sh-ryo-v0.1.10") is True
    assert calls[0] == (["gh", "api", "repos/{owner}/{repo}/git/matching-refs/tags/idiomatic-sh-ryo-v0.1.1"], Path("/repo"))


def test_a_tag_is_created_at_the_commit_the_revision_names(monkeypatch) -> None:
    calls = []

    def run(argv, **_):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0, "75d0eac7\n")

    monkeypatch.setattr(subprocess, "run", run)
    GithubTags(Path("/repo")).create("idiomatic-sh-ryo-v0.2.0", "HEAD")
    assert calls == [
        ["git", "rev-parse", "--verify", "HEAD^{commit}"],
        ["gh", "api", "--method", "POST", "repos/{owner}/{repo}/git/refs", "-f", "ref=refs/tags/idiomatic-sh-ryo-v0.2.0", "-f", "sha=75d0eac7"],
    ]
