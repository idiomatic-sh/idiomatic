"""What the npm adapter of `build` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import json
import subprocess
import zipfile

import pytest

from ci.release.build.adapters.npm.adapter import NpmBuild
from ci.release.build.interface_test import conforms
from ci.release.use_case import SHARED, Product, Refused

PRODUCT = Product(
    "demo",
    "idiomatic-sh-demo",
    "1.2.3",
    "MIT",
    "https://example.invalid/repo",
    False,
    "demo",
    ("aarch64-apple-darwin", "x86_64-pc-windows-msvc"),
)


def test_the_npm_adapter_implements_the_port() -> None:
    """It is a `build` adapter."""
    conforms(NpmBuild)


def registry(tmp_path):
    """npm's registry directory of the demo product: its launcher and one platform package per target."""
    npm = tmp_path / "crates/demo/distribution/registries/npm"
    for target in PRODUCT.targets:
        (npm / "platforms" / target).mkdir(parents=True)
        (npm / "platforms" / target / "package.json").write_text(json.dumps({"name": f"@scope/demo-{target}"}))
        (npm / "platforms" / target / "README.md").write_text(f"# @scope/demo-{target}\n")
    (npm / "package.json").write_text(json.dumps({"name": "@scope/demo"}))
    (npm / "launcher.js").write_text("// launcher\n")
    (tmp_path / "LICENSE").write_text("MIT License\n")
    return npm


def wheel(dist, target: str, executable: str) -> None:
    path = dist / target / "idiomatic_sh_demo-1.2.3-py3-none-any.whl"
    path.parent.mkdir(parents=True)
    with zipfile.ZipFile(path, "w") as opened:
        opened.writestr(f"idiomatic_sh_demo-1.2.3.data/scripts/{executable}", "binary")


def recording(monkeypatch) -> list[list[str]]:
    calls: list[list[str]] = []

    def fake(argv, **options):
        calls.append(argv)
        name = "platform" if "platform" in str(options.get("cwd")) else "launcher"
        output = json.dumps([{"filename": f"{name}.tgz"}]) if argv[:2] == ["npm", "pack"] else ""
        return subprocess.CompletedProcess(argv, 0, output, "")

    monkeypatch.setattr(subprocess, "run", fake)
    return calls


def test_what_every_target_shares_is_the_launcher_depending_on_every_planned_platform(tmp_path, monkeypatch) -> None:
    registry(tmp_path)
    calls = recording(monkeypatch)
    (artifact,) = NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, SHARED, tmp_path / "out")
    assert (artifact.registry, artifact.scope, artifact.path.name) == ("npm", SHARED, "launcher.tgz")
    assert ["npm", "pkg", "set", "version=1.2.3", "license=MIT", "repository=https://example.invalid/repo"] in calls
    assert [
        "npm", "pkg", "set",
        "optionalDependencies.@scope/demo-aarch64-apple-darwin=1.2.3",
        "optionalDependencies.@scope/demo-x86_64-pc-windows-msvc=1.2.3",
    ] in calls  # fmt: skip
    assert (tmp_path / "out/launcher/LICENSE").read_text() == "MIT License\n", "the launcher carries the repository's LICENSE"


def test_a_target_s_platform_package_holds_the_binary_of_its_wheel(tmp_path, monkeypatch) -> None:
    target = "aarch64-apple-darwin"
    registry(tmp_path)
    wheel(tmp_path / "dist", target, "demo")
    recording(monkeypatch)
    (platform,) = NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, target, tmp_path / "out")
    assert (platform.scope, platform.path.name) == (target, "platform.tgz")
    executable = tmp_path / "out/platform/bin/demo"
    assert executable.read_text() == "binary", "the wheel's binary ships in bin/"
    assert executable.stat().st_mode & 0o111, "and runs"
    assert (tmp_path / "out/platform/README.md").read_text() == f"# @scope/demo-{target}\n", "its README ships beside it"


def test_a_windows_binary_ships_as_an_exe(tmp_path, monkeypatch) -> None:
    target = "x86_64-pc-windows-msvc"
    registry(tmp_path)
    wheel(tmp_path / "dist", target, "demo.exe")
    recording(monkeypatch)
    NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, target, tmp_path / "out")
    assert (tmp_path / "out/platform/bin/demo.exe").read_text() == "binary"


def test_a_product_with_no_npm_registry_has_no_package(tmp_path) -> None:
    assert NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, SHARED, tmp_path / "out") == []
    assert NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, "aarch64-apple-darwin", tmp_path / "out") == []


def test_a_target_without_its_wheel_is_refused_naming_the_wheel(tmp_path) -> None:
    registry(tmp_path)
    with pytest.raises(Refused, match=r"no wheel .*idiomatic_sh_demo-\*\.whl"):
        NpmBuild(tmp_path, tmp_path / "dist").build(PRODUCT, "aarch64-apple-darwin", tmp_path / "out")
