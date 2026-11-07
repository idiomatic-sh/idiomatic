"""What the console adapter of `report` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

from ci.perf.report.adapters.console.adapter import ConsoleReport
from ci.perf.report.interface_test import conforms


def test_the_console_adapter_implements_the_port() -> None:
    """It is a `report` adapter."""
    conforms(ConsoleReport)


def test_each_ratio_is_one_line_with_its_verdict(capsys) -> None:
    report = ConsoleReport()
    report.ratio("ryo/lint/ruff/cold-milliseconds", 1.2, "fail")
    report.ratio("ryo/commands/check/cpu-ms", 1.0625, "warn")
    report.skipped("no change against origin/main reaches commands")
    assert capsys.readouterr().out.splitlines() == [
        "perf: FAIL: ryo/lint/ruff/cold-milliseconds 1.200x",
        "perf: warning: ryo/commands/check/cpu-ms 1.062x",
        "perf: skipped: no change against origin/main reaches commands",
    ]
