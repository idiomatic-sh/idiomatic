"""What the file adapter of `manifest` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

from ci.groups.manifest.adapters.file.adapter import FileManifest
from ci.groups.manifest.interface_test import conforms


def test_the_file_adapter_implements_the_port() -> None:
    """It is a `manifest` adapter."""
    conforms(FileManifest)


def test_the_groups_are_the_ones_the_product_s_tools_declare(tmp_path) -> None:
    manifest = tmp_path / "crates" / "ryo" / "asymptote.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text('[[tools]]\nid = "fmt"\ngroup = "lint"\n\n[[tools]]\nid = "taplo"\ngroup = "lint"\n\n[[tools]]\nid = "heap"\ngroup = "memory"\n')
    assert FileManifest(tmp_path).groups("ryo") == frozenset({"lint", "memory"})


def test_a_product_without_the_manifest_has_none(tmp_path) -> None:
    assert FileManifest(tmp_path).groups("ryo") is None
