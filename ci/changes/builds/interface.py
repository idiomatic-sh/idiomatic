"""The `builds` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Builds(typing.Protocol):
    """The `builds` step, done one way per adapter: the other products whose tree a product builds against."""

    def built_on(self, product: str) -> frozenset[str]:
        """The products whose directories `crates/<product>/Cargo.toml` patches its crates.io dependencies to."""
        ...
