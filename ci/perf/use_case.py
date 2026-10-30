"""The `perf` job's use case: the order of its steps over their ports, and nothing else.

Without a base, every bench asked for runs once over the checkout, its documents kept (the weekly record). With a base (a pull
request's `main`), the base is checked out beside the checkout and every bench the change reaches runs over both, alternately,
base then head, for `ROUNDS` rounds, so a hosted runner's drift falls on both alike. Each measure's ratio is the median of the
rounds' head/base ratios: above `FAIL` it fails the job, above `WARN` it is a warning. Absolute times are never compared: they
are the runner's, not the change's.
"""

from __future__ import annotations

import dataclasses
import statistics
import typing

if typing.TYPE_CHECKING:
    from pathlib import Path

    from ci.perf.bench.interface import Bench
    from ci.perf.report.interface import Report
    from ci.perf.tree.interface import Tree

ROUNDS = 5
FAIL = 1.15
WARN = 1.05
# The products each bench is of: `commands` and `distribution` every product's, `lint` (ryo's cost over ruff) ryo's.
PRODUCTS = {"commands": ("asymptote", "tastemaker", "ryo"), "distribution": ("asymptote", "tastemaker", "ryo"), "lint": ("ryo",)}
# What a change must touch for a bench to run against a base: any crate or the lock, and for `lint`, ryo.
REACHES = {"commands": ("crates/", "Cargo.lock"), "distribution": ("crates/", "Cargo.lock"), "lint": ("crates/ryo/",)}


class Refused(Exception):
    """A request the job cannot run, said before any step runs."""


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to do: the benches (`PRODUCTS`' names), the base to compare against (none for a record),
    and a scratch directory it may fill."""

    benches: tuple[str, ...]
    base: str | None
    scratch: Path


def reaches(bench: str, changed: tuple[str, ...]) -> bool:
    """Whether a change to `changed` can move `bench`."""
    return any(path.startswith(REACHES[bench]) for path in changed)


class UseCase:
    """The job, over the ports its composition supplies, for the checkout at `head`."""

    def __init__(self, head: Path, tree: Tree, bench: Bench, report: Report) -> None:
        self.head = head
        self.tree = tree
        self.bench = bench
        self.report = report

    def run(self, request: Request) -> int:
        """Runs the benches for `request`: 0 when no measure's ratio is above `FAIL`, 1 when one is."""
        if not request.benches:
            raise Refused("no bench named")
        unknown = sorted(set(request.benches) - set(PRODUCTS))
        if unknown:
            raise Refused(f"no bench {', '.join(unknown)}")
        if request.base is None:
            for name in request.benches:
                for product in PRODUCTS[name]:
                    self.bench.measure(self.head, product, name, "head")
            return 0
        changed = self.tree.changed(request.base)
        benches = [name for name in request.benches if reaches(name, changed)]
        if not benches:
            self.report.skipped(f"no change against {request.base} reaches {', '.join(request.benches)}")
            return 0
        base = self.tree.checkout(request.base, request.scratch / "base")
        ratios: dict[str, list[float]] = {}
        try:
            for index in range(1, ROUNDS + 1):
                for name in benches:
                    for product in PRODUCTS[name]:
                        before = self.bench.measure(base, product, name, f"base-{index}")
                        after = self.bench.measure(self.head, product, name, f"head-{index}")
                        for measure in sorted(before.keys() & after.keys()):
                            if before[measure] > 0:
                                ratios.setdefault(f"{product}/{name}/{measure}", []).append(after[measure] / before[measure])
        finally:
            self.tree.remove(base)
        failed = False
        for measure, each in sorted(ratios.items()):
            ratio = statistics.median(each)
            failed |= ratio > FAIL
            self.report.ratio(measure, ratio, "fail" if ratio > FAIL else "warn" if ratio > WARN else "pass")
        return 1 if failed else 0
