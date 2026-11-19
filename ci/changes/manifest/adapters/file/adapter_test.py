"""What the file adapter of `manifest` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

from ci.changes.manifest.adapters.file.adapter import FileManifest
from ci.changes.manifest.interface_test import conforms


def test_the_file_adapter_implements_the_port() -> None:
    """It is a `manifest` adapter."""
    conforms(FileManifest)


def test_a_product_names_the_products_its_tool_inputs_reach_and_not_the_whole_tree(tmp_path) -> None:
    manifest = tmp_path / "crates" / "asymptote" / "asymptote.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(
        '[[tools]]\nid = "test-cli"\ngroup = "check"\ninputs = [".", "../tastemaker", "../..", "../../rust-toolchain.toml"]\n\n'
        '[[tools]]\nid = "bench"\ngroup = "bench"\ninputs = ["../ryo/languages/{lang}", "languages/{lang}"]\n\n'
        '[[tools]]\nid = "fmt"\ngroup = "lint"\n'
    )
    assert FileManifest(tmp_path).named("asymptote") == frozenset({"tastemaker", "ryo"})


def test_a_product_whose_inputs_stay_home_names_none(tmp_path) -> None:
    manifest = tmp_path / "crates" / "ryo" / "asymptote.toml"
    manifest.parent.mkdir(parents=True)
    manifest.write_text('[[tools]]\nid = "fmt"\ngroup = "lint"\ninputs = [".", "../../rustfmt.toml", "../.."]\n')
    assert FileManifest(tmp_path).named("ryo") == frozenset()
