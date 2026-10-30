"""The `report` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Report(typing.Protocol):
    """The `report` step, done one way per adapter: what the job found, said where its reader sees it."""

    def ratio(self, measure: str, ratio: float, verdict: str) -> None:
        """One measure's head/base ratio and its verdict: `fail`, `warn` or `pass`."""
        ...

    def skipped(self, reason: str) -> None:
        """Nothing was compared, and why."""
        ...
