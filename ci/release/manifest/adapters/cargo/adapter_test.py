"""What the cargo adapter of `manifest` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import json
import subprocess

import pytest

from ci.release.manifest.adapters.cargo.adapter import PLATFORMS, SHIPPED, CargoManifest, cli_package
from ci.release.manifest.interface_test import conforms


def test_the_cargo_adapter_implements_the_port() -> None:
    """It is a `manifest` adapter."""
    conforms(CargoManifest)


def package(name: str, targets: list[tuple[str, str]], publish: list[str] | None) -> dict:
    return {
        "name": name,
        "version": "0.1.0",
        "license": "MIT",
        "repository": "https://example.invalid/repo",
        "publish": publish,
        "targets": [{"name": target, "kind": [kind]} for target, kind in targets],
    }


METADATA = {
    "packages": [
        package("demo-core", [("demo_core", "lib")], []),
        package("demo-cli", [("demo", "bin"), ("demo_cli", "lib"), ("smoke", "test")], []),
    ]
}


def test_the_command_line_package_is_the_one_with_the_smoke_test_target() -> None:
    assert cli_package(METADATA)["name"] == "demo-cli"
    with pytest.raises(ValueError, match="found 0"):
        cli_package({"packages": [METADATA["packages"][0]]})


def platforms(product, targets) -> None:
    """Makes `product`'s npm platform packages, one directory per target."""
    for target in targets:
        (product / PLATFORMS / target).mkdir(parents=True)


def test_a_product_reads_from_cargo_metadata_and_npm_s_platform_packages(tmp_path, monkeypatch) -> None:
    platforms(tmp_path / "crates/demo", ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"])
    (tmp_path / "crates/demo" / SHIPPED).parent.mkdir(parents=True)
    (tmp_path / "crates/demo" / SHIPPED).write_text("")
    (tmp_path / "crates/other").mkdir()

    def fake(argv, **_):
        assert argv[:2] == ["cargo", "metadata"]
        return subprocess.CompletedProcess(argv, 0, json.dumps(METADATA), "")

    monkeypatch.setattr(subprocess, "run", fake)
    manifest = CargoManifest(tmp_path)
    assert manifest.shipped() == ("demo",), "only a product with the crates.io smoke test ships"
    product = manifest.read("demo")
    assert (product.package, product.command, product.version) == ("demo-cli", "demo", "0.1.0")
    assert product.targets == ("aarch64-apple-darwin", "x86_64-unknown-linux-gnu")
    assert not product.publishable, "`publish = false` is `publish: []` in Cargo's metadata"


def test_a_product_that_ships_to_crates_io_alone_plans_the_targets_the_other_products_npm_platforms_name(tmp_path, monkeypatch) -> None:
    """Its GitHub Release archives the release's targets, the one set every product's npm platforms name (the `distribution` spec)."""
    (tmp_path / "crates/demo" / SHIPPED).parent.mkdir(parents=True)
    (tmp_path / "crates/demo" / SHIPPED).write_text("")
    for other in ("first", "second"):
        platforms(tmp_path / "crates" / other, ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"])
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, json.dumps(METADATA), ""))
    assert CargoManifest(tmp_path).read("demo").targets == ("aarch64-apple-darwin", "x86_64-unknown-linux-gnu")


def test_npm_platforms_that_disagree_on_the_targets_are_refused(tmp_path, monkeypatch) -> None:
    (tmp_path / "crates/demo" / SHIPPED).parent.mkdir(parents=True)
    (tmp_path / "crates/demo" / SHIPPED).write_text("")
    for other, target in (("first", "x86_64-unknown-linux-gnu"), ("second", "aarch64-apple-darwin")):
        platforms(tmp_path / "crates" / other, [target])
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, json.dumps(METADATA), ""))
    with pytest.raises(ValueError, match="disagree"):
        CargoManifest(tmp_path).read("demo")


def test_the_host_is_rustc_s(monkeypatch, tmp_path) -> None:
    said = "rustc 1.97.1\nbinary: rustc\nhost: aarch64-unknown-linux-gnu\nrelease: 1.97.1\n"
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, said, ""))
    assert CargoManifest(tmp_path).host() == "aarch64-unknown-linux-gnu"
