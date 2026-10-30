"""What the pypi adapter of `build` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import pytest

from ci.release.build.adapters.pypi.adapter import PypiBuild
from ci.release.build.interface_test import conforms
from ci.release.use_case import SHARED, Artifact, Product, Refused

TARGET = "x86_64-unknown-linux-gnu"
PRODUCT = Product("demo", "idiomatic-sh-demo", "0.1.0", "MIT", "", False, "demo", (TARGET,))


def test_the_pypi_adapter_implements_the_port() -> None:
    """It is a `build` adapter."""
    conforms(PypiBuild)


def test_a_target_s_wheel_is_the_one_its_build_machine_made(tmp_path) -> None:
    (tmp_path / "crates/demo/distribution/registries/pypi").mkdir(parents=True)
    wheel = tmp_path / "dist" / TARGET / "idiomatic_sh_demo-0.1.0-py3-none-manylinux_2_17_x86_64.whl"
    wheel.parent.mkdir(parents=True)
    wheel.write_text("wheel")
    (wheel.parent / "idiomatic_sh_other-0.1.0-py3-none-any.whl").write_text("another product's")
    assert PypiBuild(tmp_path, tmp_path / "dist").build(PRODUCT, TARGET, tmp_path / "out") == [Artifact("pypi", wheel, TARGET)]


def test_nothing_is_shared_by_every_target(tmp_path) -> None:
    (tmp_path / "crates/demo/distribution/registries/pypi").mkdir(parents=True)
    assert PypiBuild(tmp_path, tmp_path / "dist").build(PRODUCT, SHARED, tmp_path) == []


def test_a_product_with_no_pypi_registry_has_no_wheel(tmp_path) -> None:
    assert PypiBuild(tmp_path, tmp_path / "dist").build(PRODUCT, TARGET, tmp_path) == []


def test_a_target_without_its_wheel_is_refused_naming_the_wheel(tmp_path) -> None:
    (tmp_path / "crates/demo/distribution/registries/pypi").mkdir(parents=True)
    with pytest.raises(Refused, match=r"no wheel .*idiomatic_sh_demo-\*\.whl"):
        PypiBuild(tmp_path, tmp_path / "dist").build(PRODUCT, TARGET, tmp_path)
