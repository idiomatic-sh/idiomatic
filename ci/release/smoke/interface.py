"""The `smoke` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from pathlib import Path

    from ci.release.use_case import Artifact, Product

def version_matches(product: Product, printed: str) -> bool:
    """The crate's version, with Tastemaker's public linked-provider suffix."""
    expected = f"{product.command} {product.version}"
    if product.command == "tm":
        return printed.endswith(")\n") and printed.partition(" (")[0] == expected
    return printed == f"{expected}\n"


class Smoke(typing.Protocol):
    """The `smoke` step, done one way per registry: the built packages installed as a user installs them and the command run.
    The registry's packaging tests are not this step's: they are the `release-smoke` tool of asymptote's `release-tests` group."""

    def smoke(self, product: Product, artifacts: list[Artifact], into: Path) -> None:
        """Proves the registry's packages of `product`; raises when one does not install or
        run as it must."""
        ...
