"""What the npm adapter of `smoke` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess

import pytest

from ci.release.smoke.adapters.npm.adapter import NpmSmoke
from ci.release.smoke.interface_test import conforms
from ci.release.use_case import SHARED, Artifact, Product

PRODUCT = Product("demo", "demo-cli", "1.2.3", "MIT", "", False, "demo", ("aarch64-unknown-linux-gnu",))


def test_the_npm_adapter_implements_the_port() -> None:
    """It is a `smoke` adapter."""
    conforms(NpmSmoke)


def recording(monkeypatch, version: str) -> list[list[str]]:
    calls: list[list[str]] = []

    def fake(argv, **_):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0, version if argv[-1] == "--version" else "", "")

    monkeypatch.setattr(subprocess, "run", fake)
    return calls


def test_the_platform_and_launcher_install_offline_and_report_the_crate_s_version(tmp_path, monkeypatch) -> None:
    calls = recording(monkeypatch, "demo 1.2.3\n")
    artifacts = [Artifact("npm", tmp_path / "platform.tgz", "aarch64-unknown-linux-gnu"), Artifact("npm", tmp_path / "launcher.tgz", SHARED)]
    NpmSmoke(tmp_path).smoke(PRODUCT, artifacts, tmp_path)
    install = calls[0]
    assert install[:4] == ["npm", "install", "--global", "--offline"]
    assert install[-2:] == [str(tmp_path / "platform.tgz"), str(tmp_path / "launcher.tgz")]
    assert calls[1] == [str(tmp_path / "prefix/bin/demo"), "--version"]


def test_another_version_fails(tmp_path, monkeypatch) -> None:
    recording(monkeypatch, "demo 0.0.1\n")
    artifacts = [Artifact("npm", tmp_path / "platform.tgz", "aarch64-unknown-linux-gnu"), Artifact("npm", tmp_path / "launcher.tgz", SHARED)]
    with pytest.raises(AssertionError, match="0.0.1"):
        NpmSmoke(tmp_path).smoke(PRODUCT, artifacts, tmp_path)


def test_the_launcher_alone_is_not_installed(tmp_path, monkeypatch) -> None:
    calls = recording(monkeypatch, "")
    NpmSmoke(tmp_path).smoke(PRODUCT, [Artifact("npm", tmp_path / "launcher.tgz", SHARED)], tmp_path)
    assert calls == [], "no platform package to run the launcher with"
