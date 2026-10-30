"""The `build` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from pathlib import Path

    from ci.release.use_case import Artifact, Product


class Build(typing.Protocol):
    """The `build` step, done one way per registry: the registry's packages of a product
    for one scope, built into a directory."""

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        """The packages of `product` for `scope` (a target triple, on its own machine, or
        what every target shares), in publish order; none when the registry has none."""
        ...
