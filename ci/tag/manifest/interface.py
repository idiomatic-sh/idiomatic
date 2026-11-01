"""The `manifest` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Manifest(typing.Protocol):
    """The `manifest` step, done one way per adapter: a product's workspace version at a commit."""

    def version(self, product: str, revision: str) -> str | None:
        """The workspace version `crates/<product>/Cargo.toml` declares at `revision`, or None when it has none there."""
        ...
