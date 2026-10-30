"""What the pypi adapter of `smoke` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess

from ci.release.smoke.adapters.pypi.adapter import PypiSmoke
from ci.release.smoke.interface_test import conforms
from ci.release.use_case import Artifact, Product


def test_the_pypi_adapter_implements_the_port() -> None:
    """It is a `smoke` adapter."""
    conforms(PypiSmoke)


def test_the_wheel_installs_without_the_index_and_runs_both_ways(tmp_path, monkeypatch) -> None:
    pypi = tmp_path / "crates/demo/distribution/registries/pypi"
    pypi.mkdir(parents=True)
    (pypi / "pyproject.toml").write_text('[tool.maturin]\nmodule-name = "org_demo"\n')
    calls = []

    def fake(argv, **_):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0, "demo 1.2.3\n" if argv[-1] == "--version" else "", "")

    monkeypatch.setattr(subprocess, "run", fake)
    product = Product("demo", "demo-cli", "1.2.3", "MIT", "", False, "demo", ())
    PypiSmoke(tmp_path).smoke(product, [Artifact("pypi", tmp_path / "demo.whl", "x")], tmp_path)
    assert calls[1] == ["uv", "pip", "install", "--python", str(tmp_path / "venv/bin/python"), "--no-index", "--no-deps", str(tmp_path / "demo.whl")]
    assert calls[2] == [str(tmp_path / "venv/bin/demo"), "--version"]
    assert calls[3][1:] == ["-m", "org_demo", "--version"]
