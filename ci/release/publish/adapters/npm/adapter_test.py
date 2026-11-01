"""What the npm adapter of `publish` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release.publish.adapters.npm.adapter import NpmPublish, npm_publish
from ci.release.publish.interface_test import conforms
from ci.release.use_case import Artifact


def test_the_npm_adapter_implements_the_port() -> None:
    """It is a `publish` adapter."""
    conforms(NpmPublish)


ARTIFACT = Artifact("npm", Path("/packed/demo-1.2.3.tgz"), "shared")


def test_a_dry_run_uploads_nothing() -> None:
    assert npm_publish(ARTIFACT, True) == ["npm", "publish", "--access", "public", "--dry-run", "/packed/demo-1.2.3.tgz"]


def test_an_upload_carries_provenance_and_is_no_dry_run() -> None:
    argv = npm_publish(ARTIFACT, False)
    assert "--provenance" in argv and "--dry-run" not in argv


def test_the_mode_reaches_the_registry(monkeypatch) -> None:
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: calls.append(argv))
    for dry_run in (True, False):
        NpmPublish().publish(None, ARTIFACT, dry_run)
    assert "--dry-run" in calls[0] and "--dry-run" not in calls[1], "a dry run never uploads and an upload is no dry run"


def test_an_upload_is_handed_no_token_and_goes_through_trusted_publishing(monkeypatch) -> None:
    """npm exchanges the workflow's OIDC token itself: the adapter configures no token of its own."""
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **kwargs: calls.append(kwargs))
    NpmPublish().publish(None, ARTIFACT, False)
    assert "env" not in calls[0]
