"""The `manifest` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Manifest(typing.Protocol):
    """The `manifest` step, done one way per adapter: the groups a product's tools declare."""

    def groups(self, product: str) -> frozenset[str] | None:
        """The groups the tools of `crates/<product>/asymptote.toml` declare, or None when the product has no such file."""
        ...
