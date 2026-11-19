"""What the file adapter of `builds` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

from ci.changes.builds.adapters.file.adapter import FileBuilds
from ci.changes.builds.interface_test import conforms


def test_the_file_adapter_implements_the_port() -> None:
    """It is a `builds` adapter."""
    conforms(FileBuilds)


def test_a_product_builds_on_the_products_its_patches_point_into(tmp_path) -> None:
    manifest = tmp_path / "crates" / "tastemaker" / "Cargo.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(
        '[workspace]\nmembers = ["cli"]\n\n[patch.crates-io]\n'
        'idiomatic-sh-ryo-sdk = { path = "../ryo/crates/sdk" }\n'
        'idiomatic-sh-ryo-python = { path = "../ryo/languages/python" }\n'
        'own = { path = "crates/own" }\n'
        'outside = { path = "../../vendor/outside" }\n'
        'upstream = { git = "https://example.invalid/upstream" }\n'
    )
    assert FileBuilds(tmp_path).built_on("tastemaker") == frozenset({"ryo"})


def test_a_product_without_patches_builds_on_none(tmp_path) -> None:
    manifest = tmp_path / "crates" / "ryo" / "Cargo.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text('[workspace]\nmembers = ["cli"]\n')
    assert FileBuilds(tmp_path).built_on("ryo") == frozenset()
