"""What the `perf` job's use case promises, over fakes of every port."""

from __future__ import annotations

from pathlib import Path

import pytest

from ci.perf.bench.interface import Bench
from ci.perf.report.interface import Report
from ci.perf.tree.interface import Tree
from ci.perf.use_case import ROUNDS, Refused, Request, UseCase

HEAD = Path("/head")
BASE = Path("/scratch/base")


class Journal:
    """Every call the fakes receive, in order."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []


class FakeTree(Tree):
    def __init__(self, journal: Journal, changed: tuple[str, ...]) -> None:
        self.journal = journal
        self.paths = changed

    def changed(self, base: str) -> tuple[str, ...]:
        self.journal.calls.append(("changed", base))
        return self.paths

    def checkout(self, base: str, into: Path) -> Path:
        self.journal.calls.append(("checkout", base, into))
        return into

    def remove(self, tree: Path) -> None:
        self.journal.calls.append(("remove", tree))


class FakeBench(Bench):
    """Every measure is 100 in the base; in the head, `slower` scales the named measures, round by round."""

    def __init__(self, journal: Journal, slower: dict[str, list[float]] | None = None, fails: bool = False) -> None:
        self.journal = journal
        self.slower = slower or {}
        self.fails = fails
        self.rounds: dict[str, int] = {}

    def measure(self, tree: Path, product: str, bench: str, label: str) -> dict[str, float]:
        self.journal.calls.append(("measure", tree, product, bench, label))
        if self.fails:
            raise RuntimeError(f"{bench} of {product} does not build")
        measured = {"check/cpu-ms": 100.0, "check/peak-megabytes": 100.0}
        if tree != HEAD:
            return measured
        index = int(label.split("-")[1]) - 1 if "-" in label else 0
        return {name: value * self.slower.get(f"{product}/{bench}/{name}", [1.0] * ROUNDS)[index] for name, value in measured.items()}


class FakeReport(Report):
    def __init__(self, journal: Journal) -> None:
        self.journal = journal

    def ratio(self, measure: str, ratio: float, verdict: str) -> None:
        self.journal.calls.append(("ratio", measure, round(ratio, 4), verdict))

    def skipped(self, reason: str) -> None:
        self.journal.calls.append(("skipped", reason))


def job(journal: Journal, changed: tuple[str, ...] = ("crates/ryo/cli/src/main.rs",), **bench) -> UseCase:
    return UseCase(HEAD, FakeTree(journal, changed), FakeBench(journal, **bench), FakeReport(journal))


def against(*benches: str) -> Request:
    return Request(benches, "origin/main", Path("/scratch"))


def test_a_record_runs_each_bench_once_over_the_checkout_and_judges_nothing() -> None:
    journal = Journal()
    assert job(journal).run(Request(("distribution", "lint"), None, Path("/scratch"))) == 0
    assert journal.calls == [
        ("measure", HEAD, "asymptote", "distribution", "head"),
        ("measure", HEAD, "tastemaker", "distribution", "head"),
        ("measure", HEAD, "ryo", "distribution", "head"),
        ("measure", HEAD, "ryo", "lint", "head"),
    ]


def test_a_comparison_alternates_base_and_head_for_every_round_then_removes_the_base() -> None:
    journal = Journal()
    assert job(journal).run(against("lint")) == 0
    measured = [call for call in journal.calls if call[0] == "measure"]
    assert measured == [
        ("measure", tree, "ryo", "lint", f"{side}-{index}")
        for index in range(1, ROUNDS + 1)
        for tree, side in ((BASE, "base"), (HEAD, "head"))
    ]
    assert journal.calls[:2] == [("changed", "origin/main"), ("checkout", "origin/main", BASE)]
    assert ("remove", BASE) in journal.calls


def test_each_measure_is_judged_by_the_median_of_its_rounds_ratios() -> None:
    journal = Journal()
    # One noisy round of 1.5 does not fail the median; a median of 1.08 warns; a median of 1.2 fails.
    slower = {
        "ryo/lint/check/cpu-ms": [1.0, 1.5, 1.0, 1.0, 1.0],
        "ryo/lint/check/peak-megabytes": [1.08, 1.08, 1.0, 1.1, 1.09],
    }
    assert job(journal, slower=slower).run(against("lint")) == 0
    judged = [call for call in journal.calls if call[0] == "ratio"]
    assert judged == [("ratio", "ryo/lint/check/cpu-ms", 1.0, "pass"), ("ratio", "ryo/lint/check/peak-megabytes", 1.08, "warn")]
    failing = Journal()
    assert job(failing, slower={"ryo/lint/check/cpu-ms": [1.2] * ROUNDS}).run(against("lint")) == 1
    assert ("ratio", "ryo/lint/check/cpu-ms", 1.2, "fail") in failing.calls


def test_a_ratio_at_the_margins_passes_them() -> None:
    journal = Journal()
    margins = {"ryo/lint/check/cpu-ms": [1.15] * ROUNDS, "ryo/lint/check/peak-megabytes": [1.05] * ROUNDS}
    assert job(journal, slower=margins).run(against("lint")) == 0
    assert [call[3] for call in journal.calls if call[0] == "ratio"] == ["warn", "pass"]


def test_only_the_benches_the_change_reaches_run() -> None:
    journal = Journal()
    job(journal, changed=("crates/tastemaker/cli/src/main.rs",)).run(against("commands", "lint"))
    assert {call[3] for call in journal.calls if call[0] == "measure"} == {"commands"}, "lint runs only when ryo changed"
    nothing = Journal()
    assert job(nothing, changed=("README.md", ".github/workflows/check.yml")).run(against("commands", "lint")) == 0
    assert nothing.calls == [("changed", "origin/main"), ("skipped", "no change against origin/main reaches commands, lint")]


def test_a_failing_bench_still_removes_the_base() -> None:
    journal = Journal()
    with pytest.raises(RuntimeError):
        job(journal, fails=True).run(against("lint"))
    assert journal.calls[-1] == ("remove", BASE)


def test_no_bench_or_an_unknown_one_is_refused() -> None:
    with pytest.raises(Refused, match="no bench named"):
        job(Journal()).run(against())
    with pytest.raises(Refused, match="no bench clippy"):
        job(Journal()).run(against("clippy"))
