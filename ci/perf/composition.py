"""The `perf` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.perf.bench.adapters.cargo.adapter import CargoBench
from ci.perf.report.adapters.console.adapter import ConsoleReport
from ci.perf.tree.adapters.git.adapter import GitTree
from ci.perf.use_case import UseCase

if typing.TYPE_CHECKING:
    from ci.perf.bench.interface import Bench
    from ci.perf.report.interface import Report
    from ci.perf.tree.interface import Tree

# The repository this job's file sits in: the checkout the benches judge.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the checkout, its trees, its benches and its report."""

    head: Path
    tree: Tree
    bench: Bench
    report: Report


def build(output: Path | None) -> UseCase:
    """The job with its real adapters, keeping every bench's document under `output` when one is given."""
    return compose(Parts(head=ROOT, tree=GitTree(ROOT), bench=CargoBench(ROOT, output), report=ConsoleReport()))


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.head, parts.tree, parts.bench, parts.report)
