"""What the cargo adapter of `build` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

from ci.release.build.adapters.cargo.adapter import CargoBuild
from ci.release.build.interface_test import conforms
from ci.release.use_case import SHARED, Artifact, Product


def test_the_cargo_adapter_implements_the_port() -> None:
    """It is a `build` adapter."""
    conforms(CargoBuild)


def test_the_crate_is_one_package_every_target_shares(tmp_path) -> None:
    product = Product("demo", "demo-cli", "0.1.0", "MIT", "", False, "demo", ("x86_64-unknown-linux-gnu",))
    assert CargoBuild(tmp_path).build(product, SHARED, tmp_path / "out") == [Artifact("cargo", tmp_path / "crates/demo/cli", SHARED)]
    assert CargoBuild(tmp_path).build(product, "x86_64-unknown-linux-gnu", tmp_path / "out") == []
