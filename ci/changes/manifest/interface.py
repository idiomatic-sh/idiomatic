"""The `manifest` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Manifest(typing.Protocol):
    """The `manifest` step, done one way per adapter: the other products whose directories a product's checks read."""

    def named(self, product: str) -> frozenset[str]:
        """The other products whose directory, or a path inside it, an input of a tool of `crates/<product>/asymptote.toml`
        names. An input of the whole tree names no product: the run-time reads of the whole tree are `RUN_TIME_READS`'."""
        ...
