"""The `groups` job's use case: the order of its steps over their ports, and nothing else.

A job that runs the same groups for every product asks each product for the ones its tools declare, since asymptote refuses a
`--groups` naming a group no tool of the product declares. A group no product declares is refused, not dropped, so a misspelt
group never leaves a check unseen; so are a product with no `asymptote.toml` and a product left with no group to run.
"""

from __future__ import annotations

import dataclasses
import typing

from ci.tag.use_case import PRODUCTS

if typing.TYPE_CHECKING:
    from ci.groups.manifest.interface import Manifest


class Refused(Exception):
    """A request the job cannot run, said before any group is named."""


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to do: the product, and the groups the job asks of every product, in order."""

    product: str
    asked: tuple[str, ...]


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, manifest: Manifest) -> None:
        self.manifest = manifest

    def run(self, request: Request) -> tuple[str, ...]:
        """The groups of `request.asked`, in their order, that `request.product`'s tools declare."""
        declared = {product: self.manifest.groups(product) for product in PRODUCTS}
        if request.product not in declared:
            raise Refused(f"{request.product} is not a product: {', '.join(PRODUCTS)}")
        own = declared[request.product]
        if own is None:
            raise Refused(f"{request.product} has no crates/{request.product}/asymptote.toml")
        anyone = set().union(*(each for each in declared.values() if each is not None))
        unknown = [group for group in request.asked if group not in anyone]
        if unknown:
            raise Refused(f"no product declares {', '.join(unknown)}")
        kept = tuple(group for group in request.asked if group in own)
        if not kept:
            raise Refused(f"{request.product} declares none of {', '.join(request.asked)}")
        return kept
