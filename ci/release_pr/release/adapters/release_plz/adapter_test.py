"""What the release-plz adapter of `release` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release_pr.release.adapters.release_plz.adapter import ReleasePlzRelease
from ci.release_pr.release.interface import Opened
from ci.release_pr.release.interface_test import conforms

PRINTED = """{"prs":[{"head_branch":"release-plz-ryo-2026-11-02T10-00-00Z","base_branch":"main",
"html_url":"https://github.com/idiomatic-sh/idiomatic/pull/40","number":40,
"releases":[{"package_name":"idiomatic-sh-ryo","version":"0.1.3"},{"package_name":"idiomatic-sh-ryo-core","version":"0.1.3"}]}]}"""


def test_the_release_plz_adapter_implements_the_port() -> None:
    """It is a `release` adapter."""
    conforms(ReleasePlzRelease)


def test_the_pull_request_is_opened_over_the_product_s_workspace_and_configuration_with_gh_s_token(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"], kwargs["env"]["GIT_TOKEN"]))
        return subprocess.CompletedProcess(argv, 0, PRINTED)

    monkeypatch.setattr(subprocess, "run", run)
    monkeypatch.setenv("GH_TOKEN", "token")
    assert ReleasePlzRelease(Path("/repo")).open("ryo") == Opened(
        number=40,
        url="https://github.com/idiomatic-sh/idiomatic/pull/40",
        branch="release-plz-ryo-2026-11-02T10-00-00Z",
        versions=("0.1.3", "0.1.3"),
    )
    assert calls == [
        (
            ["release-plz", "release-pr", "--manifest-path", "crates/ryo/Cargo.toml", "--config", "crates/ryo/release-plz.toml", "--output", "json"],
            Path("/repo"),
            "token",
        )
    ]


def test_a_product_with_nothing_unreleased_opens_no_pull_request(monkeypatch) -> None:
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, '{"prs":[]}\n'))
    assert ReleasePlzRelease(Path("/repo")).open("asymptote") is None
