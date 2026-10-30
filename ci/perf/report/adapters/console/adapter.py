"""The `report` step done through the console."""

from __future__ import annotations

from ci.perf.report.interface import Report

# How each verdict is said: a failure and a warning stand out in a log.
SAID = {"fail": "FAIL", "warn": "warning", "pass": "ok"}


class ConsoleReport(Report):
    """`report` through the console: one line on stdout for each measure, `<verdict>: <measure> <ratio>x`."""

    def ratio(self, measure: str, ratio: float, verdict: str) -> None:
        print(f"perf: {SAID[verdict]}: {measure} {ratio:.3f}x")

    def skipped(self, reason: str) -> None:
        print(f"perf: skipped: {reason}")
