"""The `groups` job's use case: the order of its steps over their ports, and nothing else.

A job that runs the same groups for every product asks each product for the ones its tools declare, since asymptote refuses a
`--groups` naming a group no tool of the product declares. A group no product declares is refused, not dropped, so a misspelt
group never leaves a check unseen; so are a product with no `asymptote.toml` and a product left with no group to run. A hand
release's check is one job per row of `RELEASE_ROWS`: the product's rows are the ones whose group it declares, and the root's.
"""

from __future__ import annotations

import dataclasses
import typing

from ci.tag.use_case import PRODUCTS

if typing.TYPE_CHECKING:
    from ci.groups.manifest.interface import Manifest


# A hand release's check, one job per row: a row's group on its machine, from the product's root, or from the repository's root
# (`root`) for the root's own `repository` group, which no product has. The tests (check, release-tests) run on every machine;
# the groups whose verdict is the same on every platform run once, on Linux, but the SDK's API, on macOS, where cargo-public-api
# ships a prebuilt. bench holds the linter benches. check.yml runs these rows for each product a run checks (ci/changes).
RELEASE_ROWS: tuple[dict[str, str], ...] = (
    {"runner": "ubuntu-26.04", "group": "repository", "root": "."},
    {"runner": "ubuntu-26.04", "group": "lint"},
    {"runner": "ubuntu-26.04", "group": "clippy"},
    {"runner": "macos-26", "group": "api"},
    {"runner": "ubuntu-26.04", "group": "check"},
    {"runner": "ubuntu-26.04-arm", "group": "check"},
    {"runner": "macos-26", "group": "check"},
    {"runner": "windows-2025", "group": "check"},
    {"runner": "ubuntu-26.04", "group": "release-tests"},
    {"runner": "ubuntu-26.04-arm", "group": "release-tests"},
    {"runner": "macos-26", "group": "release-tests"},
    {"runner": "windows-2025", "group": "release-tests"},
    {"runner": "ubuntu-26.04", "group": "release-clippy"},
    {"runner": "ubuntu-26.04", "group": "release-coverage"},
    {"runner": "ubuntu-26.04", "group": "memory"},
    {"runner": "ubuntu-26.04", "group": "bench"},
    {"runner": "ubuntu-26.04", "group": "release-msrv"},
    {"runner": "ubuntu-26.04", "group": "release-dylint"},
    {"runner": "ubuntu-26.04", "group": "release-hack"},
    {"runner": "ubuntu-26.04", "group": "release-resolution"},
)


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

    def rows(self, product: str) -> tuple[dict[str, str], ...]:
        """The rows of `RELEASE_ROWS` a hand release of `product` checks, in their order: the root's, and each whose group the
        product's tools declare, through `run`, so a group no product declares is refused here too."""
        asked = tuple(dict.fromkeys(row["group"] for row in RELEASE_ROWS if "root" not in row))
        kept = set(self.run(Request(product, asked)))
        return tuple(row for row in RELEASE_ROWS if "root" in row or row["group"] in kept)
