"""What the github adapter of `build` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import tarfile
import zipfile

import pytest

from ci.release.build.adapters.github.adapter import ARCHIVES, GithubBuild
from ci.release.build.interface_test import conforms
from ci.release.use_case import SHARED, Artifact, Product, Refused

PRODUCT = Product("demo", "idiomatic-sh-demo", "0.1.0", "MIT", "", False, "demo", ())


def test_the_github_adapter_implements_the_port() -> None:
    """It is a `build` adapter."""
    conforms(GithubBuild)


def wheel(dist, target: str, executable: str) -> None:
    path = dist / target / "idiomatic_sh_demo-0.1.0-py3-none-any.whl"
    path.parent.mkdir(parents=True)
    with zipfile.ZipFile(path, "w") as opened:
        opened.writestr(f"idiomatic_sh_demo-0.1.0.data/scripts/{executable}", "binary")
        opened.writestr("idiomatic_sh_demo/__init__.py", "")


def test_a_unix_target_s_binary_is_archived_as_a_tar_gz_holding_it_alone(tmp_path) -> None:
    target = "aarch64-apple-darwin"
    wheel(tmp_path / "dist", target, "demo")
    (artifact,) = GithubBuild(tmp_path, tmp_path / "dist").build(PRODUCT, target, tmp_path / "out")
    assert artifact == Artifact("github", tmp_path / "dist" / ARCHIVES / "idiomatic-sh-demo-v0.1.0" / f"idiomatic-sh-demo-{target}.tar.gz", target)
    with tarfile.open(artifact.path) as opened:
        (member,) = opened.getmembers()
        assert (member.name, member.mode, opened.extractfile(member).read()) == ("demo", 0o755, b"binary")


def test_a_windows_target_s_binary_is_archived_as_a_zip_holding_it_alone(tmp_path) -> None:
    target = "x86_64-pc-windows-msvc"
    wheel(tmp_path / "dist", target, "demo.exe")
    (artifact,) = GithubBuild(tmp_path, tmp_path / "dist").build(PRODUCT, target, tmp_path / "out")
    assert artifact.path.name == f"idiomatic-sh-demo-{target}.zip"
    with zipfile.ZipFile(artifact.path) as opened:
        assert opened.namelist() == ["demo.exe"]
        assert opened.read("demo.exe") == b"binary"


def test_an_archive_is_the_same_bytes_every_time(tmp_path) -> None:
    target = "aarch64-apple-darwin"
    wheel(tmp_path / "dist", target, "demo")
    build = GithubBuild(tmp_path, tmp_path / "dist")
    (first,) = build.build(PRODUCT, target, tmp_path / "out")
    kept = first.path.read_bytes()
    (second,) = build.build(PRODUCT, target, tmp_path / "out")
    assert second.path.read_bytes() == kept


def test_nothing_is_shared_by_every_target(tmp_path) -> None:
    assert GithubBuild(tmp_path, tmp_path / "dist").build(PRODUCT, SHARED, tmp_path) == []


def test_a_wheel_without_the_executable_is_refused(tmp_path) -> None:
    wheel(tmp_path / "dist", "aarch64-apple-darwin", "other")
    with pytest.raises(ValueError):
        GithubBuild(tmp_path, tmp_path / "dist").build(PRODUCT, "aarch64-apple-darwin", tmp_path)


def test_a_target_without_its_wheel_is_refused_naming_the_wheel(tmp_path) -> None:
    with pytest.raises(Refused, match=r"no wheel .*-\*\.whl"):
        GithubBuild(tmp_path, tmp_path / "dist").build(PRODUCT, "x86_64-unknown-linux-gnu", tmp_path)
