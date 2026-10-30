"""The `bench` step done through Cargo."""

from __future__ import annotations

import json
import os
import subprocess
import tomllib
from pathlib import Path

from ci.perf.bench.interface import Bench

# The flags a bench is built with beside `--locked --workspace`, as the products' `asymptote.toml` run it.
FEATURES = {"distribution": ("--all-features",)}


def cargo_bench(bench: str) -> list[str]:
    """Cargo's run of one bench over a product's workspace."""
    return ["cargo", "bench", "--locked", "--workspace", *FEATURES.get(bench, ()), "--bench", bench]


def measures(document: str) -> dict[str, float]:
    """Each measure of a Bencher Metric Format document, `<benchmark>/<measure>`, with its value."""
    return {
        f"{benchmark}/{measure}": float(values["value"])
        for benchmark, measured in json.loads(document).items()
        for measure, values in measured.items()
    }


class CargoBench(Bench):
    """`bench` through Cargo, in the product's directory of the tree, with the toolchain of the checkout's `rust-toolchain.toml`,
    so a base and its head build alike; the Bencher Metric Format document the bench prints is kept under `output` when one is
    given."""

    def __init__(self, root: Path, output: Path | None) -> None:
        self.root = root
        self.output = output

    def measure(self, tree: Path, product: str, bench: str, label: str) -> dict[str, float]:
        channel = tomllib.loads((self.root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        document = subprocess.run(
            cargo_bench(bench),
            cwd=tree / "crates" / product,
            env={**os.environ, "RUSTUP_TOOLCHAIN": channel},
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        ).stdout
        if self.output is not None:
            self.output.mkdir(parents=True, exist_ok=True)
            (self.output / f"{label}-{product}-{bench}.json").write_text(document)
        return measures(document)
