"""What the pypi adapter of `publish` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release.publish.adapters.pypi.adapter import PypiPublish, uv_publish
from ci.release.publish.interface_test import conforms
from ci.release.use_case import Artifact


def test_the_pypi_adapter_implements_the_port() -> None:
    """It is a `publish` adapter."""
    conforms(PypiPublish)


ARTIFACT = Artifact("pypi", Path("/wheels/demo.whl"), "x86_64-unknown-linux-gnu")


def test_a_dry_run_uploads_nothing() -> None:
    assert uv_publish(ARTIFACT, True) == ["uv", "publish", "--dry-run", "/wheels/demo.whl"]


def test_an_upload_goes_through_trusted_publishing_only() -> None:
    assert uv_publish(ARTIFACT, False) == ["uv", "publish", "--trusted-publishing", "always", "/wheels/demo.whl"]


def test_the_mode_reaches_the_registry(monkeypatch) -> None:
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: calls.append(argv))
    for dry_run in (True, False):
        PypiPublish().publish(None, ARTIFACT, dry_run)
    assert "--dry-run" in calls[0] and "--dry-run" not in calls[1], "a dry run never uploads and an upload is no dry run"
