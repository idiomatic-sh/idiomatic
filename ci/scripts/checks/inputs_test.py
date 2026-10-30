"""What the products' `asymptote.toml` promise about the tools that compile: their `inputs` reach everything they compile."""

from __future__ import annotations

import tomllib
from pathlib import Path

# The commands whose result depends on what cargo compiles.
COMPILING = ("clippy", "dylint", "cargo test", "cargo bench", "cargo check", "cargo hack", "cargo-hack")


def paths(table: object) -> list[str]:
    """Every `path` a Cargo manifest names, at any depth, but under its tools' `metadata`,
    which cargo never compiles (dylint's lint libraries, its run's own input)."""
    if isinstance(table, dict):
        found = [table["path"]] if isinstance(table.get("path"), str) else []
        return found + [path for key, value in table.items() if key != "metadata" for path in paths(value)]
    if isinstance(table, list):
        return [path for value in table for path in paths(value)]
    return []


def test_a_compiling_tool_reruns_when_anything_it_compiles_changes() -> None:
    """A product's compiling tools (clippy, dylint, cargo test, cargo bench) are reused while their `inputs` did not move, so
    those inputs reach every path dependency out of the product, and the build configuration and toolchain: a change to a
    sibling SDK the product compiles moves the stamp, and the run is never reused over it."""
    root = Path(__file__).resolve().parents[3]
    for manifest in sorted(root.glob("crates/*/asymptote.toml")):
        product = manifest.parent.resolve()
        workspace = tomllib.loads((product / "Cargo.toml").read_text())
        members = [product / "Cargo.toml"] + [
            member / "Cargo.toml"
            for pattern in workspace.get("workspace", {}).get("members", [])
            for member in product.glob(pattern)
            if (member / "Cargo.toml").is_file()
        ]
        compiled = {
            (cargo.parent / path).resolve() for cargo in members for path in paths(tomllib.loads(cargo.read_text()))
        }
        outside = {path for path in compiled if not path.is_relative_to(product)}
        for tool in tomllib.loads(manifest.read_text()).get("tools", []):
            command = " ".join(tool["run"])
            if "inputs" not in tool or not any(each in command for each in COMPILING):
                continue
            inputs = [(product / given).resolve() for given in tool["inputs"]]
            needed = outside | {root / ".cargo", root / "rust-toolchain.toml"}
            missing = [path for path in needed if not any(path.is_relative_to(given) for given in inputs)]
            assert not missing, f"{manifest.parent.name} {tool['id']} reuses its run over {sorted(map(str, missing))}"
