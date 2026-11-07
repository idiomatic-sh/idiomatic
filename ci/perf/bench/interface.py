"""The `bench` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from pathlib import Path


class Bench(typing.Protocol):
    """The `bench` step, done one way per adapter: one bench of one product run in one tree."""

    def measure(self, tree: Path, product: str, bench: str, label: str) -> dict[str, float]:
        """Runs `bench` of `product` in `tree` and returns each of its measures, `<benchmark>/<measure>`, with the value the
        bench reports for it; `label` names the run (`head`, `base-1`) where the document is kept."""
        ...

    def required(self, tree: Path, product: str, bench: str) -> set[str] | None:
        """Declared metric identities, independent of observed results; None means the legacy bench has no declared registry."""
        return None
