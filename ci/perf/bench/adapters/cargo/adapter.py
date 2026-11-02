"""The `bench` step done through Cargo."""

from __future__ import annotations

import contextlib
import difflib
import hashlib
import json
import os
import subprocess
import tomllib
from pathlib import Path

from ci.perf.bench.interface import Bench
from ci.perf.use_case import Refused
from ci.perf.bench.adapters.cargo.language import declared_metrics, measure_languages

# The flags a bench is built with beside `--locked --workspace`, as the products' `asymptote.toml` run it.
FEATURES = {"distribution": ("--all-features",)}
BENCH_DIRECTORY = Path("crates/tastemaker/cli/benches/performance")
FIXTURE_HELPER = Path("crates/tastemaker/cli/tests/replay/benchmark_fixture.rs")
FIXTURE_COPY = '        copy_tree(&source.join("ryo"), &copy.path().join("crates/ryo"));\n'
FIXTURE_HOOK = ('        super::fixture_version::normalize(&copy.path().join("crates/ryo"))\n'
                '            .expect("the benchmark\'s copied fixture uses its executing tool version");\n')
FIXTURE_MODULE = '#[path = "../../tests/replay/benchmark_fixture.rs"]\nmod fixture_version;\n\n'


@contextlib.contextmanager
def tastemaker_fixture(head: Path, tree: Path):
    """An explicit, restored benchmark-setup overlay; historical production source is never patched."""
    driver = tree / BENCH_DIRECTORY / "use_case.rs"
    helper = tree / FIXTURE_HELPER
    original = driver.read_bytes()
    setup = (head / FIXTURE_HELPER).read_bytes()
    source = original.decode("utf-8")
    if source.count(FIXTURE_COPY) != 1:
        raise Refused("unsupported historical Tastemaker benchmark fixture-copy shape")
    if source.count(FIXTURE_HOOK) > 1 or source.count(FIXTURE_MODULE) > 1:
        raise Refused("ambiguous Tastemaker benchmark version setup")
    if FIXTURE_HOOK in source and FIXTURE_MODULE not in source:
        raise Refused("incomplete Tastemaker benchmark version setup")
    if FIXTURE_MODULE not in source:
        anchor = "/// The core's warm and cold `check`.\n"
        if source.count(anchor) != 1:
            raise Refused("unsupported historical Tastemaker benchmark module shape")
        source = source.replace(anchor, FIXTURE_MODULE + anchor, 1)
    if FIXTURE_HOOK not in source:
        source = source.replace(FIXTURE_COPY, FIXTURE_COPY + FIXTURE_HOOK, 1)
    effective = source.encode("utf-8")
    previous_helper = helper.read_bytes() if helper.exists() else None
    provenance = {
        "policy": "scratch-tool-pin-only/v1",
        "execution": "audited-benchmark-setup-overlay",
        "original_driver_sha256": hashlib.sha256(original).hexdigest(),
        "effective_driver_sha256": hashlib.sha256(effective).hexdigest(),
        "helper_sha256": hashlib.sha256(setup).hexdigest(),
        "helper_source": setup.decode("utf-8"),
        "driver_patch": "".join(difflib.unified_diff(original.decode().splitlines(keepends=True), source.splitlines(keepends=True),
                                                   fromfile="original/use_case.rs", tofile="effective/use_case.rs")),
    }
    try:
        if effective != original:
            driver.write_bytes(effective)
        if previous_helper != setup:
            helper.parent.mkdir(parents=True, exist_ok=True)
            helper.write_bytes(setup)
        yield provenance
    finally:
        if effective != original:
            driver.write_bytes(original)
        if previous_helper is None:
            helper.unlink(missing_ok=True)
        elif previous_helper != setup:
            helper.write_bytes(previous_helper)


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
    given. Tastemaker's command bench uses an explicitly recorded setup overlay for its copied fixture's tool pin;
    historical benchmark source is restored after each run, without changing production manifests or version checks."""

    def __init__(self, root: Path, output: Path | None) -> None:
        self.root = root
        self.output = output

    def measure(self, tree: Path, product: str, bench: str, label: str) -> dict[str, float]:
        channel = tomllib.loads((self.root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        if bench == "languages":
            if product != "ryo":
                raise Refused("language timings belong to the ryo workspace")
            return measure_languages(self.root, tree, self.output, label, channel)
        command = cargo_bench(bench)
        compatible = product == "tastemaker" and bench == "commands"
        output = self.output or (self.root / "target/benchmarks/speed" if compatible else None)
        context = tastemaker_fixture(self.root, tree) if compatible else contextlib.nullcontext(None)
        with context as provenance:
            result = subprocess.run(
                command,
                cwd=tree / "crates" / product,
                env={**os.environ, "RUSTUP_TOOLCHAIN": channel},
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True, encoding="utf-8",
            )
        if output is not None:
            output.mkdir(parents=True, exist_ok=True)
            stem = f"{label}-{product}-{bench}"
            (output / f"{stem}.stdout.txt").write_text(result.stdout)
            (output / f"{stem}.stderr.txt").write_text(result.stderr)
            evidence = {"command": command, "status": result.returncode}
            if provenance is not None:
                evidence["benchmark_setup"] = provenance
            (output / f"{stem}.command.json").write_text(
                json.dumps(evidence, indent=2) + "\n"
            )
        if result.returncode:
            raise subprocess.CalledProcessError(
                result.returncode, command, output=result.stdout, stderr=result.stderr
            )
        if self.output is not None:
            (self.output / f"{label}-{product}-{bench}.json").write_text(result.stdout)
        return measures(result.stdout)

    def required(self, tree: Path, product: str, bench: str) -> set[str] | None:
        """Declared language/workload/argument metrics never come from the measured table."""
        return declared_metrics(tree) if product == "ryo" and bench == "languages" else None
