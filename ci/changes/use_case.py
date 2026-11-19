"""The `changes` job's use case: the order of its steps over their ports, and nothing else.

A pull request checks the products it touches, and every product when it touches a path they all read: each path it changes
is a product's (`crates/<product>/...`), shared (`SHARED`: the workflows, the CI helpers, the tools, the toolchain and cargo
configuration, the root's Tastemaker package and its definitions), or the root's alone (`ROOT_ONLY`: the files only the `root`
job's groups read, which every pull request runs). A path no list names is refused, not guessed, so a new top-level file is
classified before any pull request skips a product for it. A push to main checks every product, and a hand run the one it names.

A checked product runs its Linux job and every row of `CHECK_ROWS`, its release rows among them, so a platform or packaging
blocker shows on the pull request that brings it. One row waits: a product that builds against another product's tree
(`builds`) does not resolve its packages afresh from crates.io (`release-resolution`) while that product's version has no tag
yet (`pending`, the `tag` job's own answer), since the version it needs is published only by the release this check gates
(ryo-gnv5: Tastemaker on an unreleased ryo held main red, and so held back the release that would publish ryo). The release
publishes the products one at a time in `PRODUCTS`' order, so the version a product needs is on crates.io before its own
publish, and the row runs again in every check after the release tags that version.

The job's answer is what check.yml's jobs read: the products the Linux job runs, the rows of the release's check, and the jobs
the gate lets skip because they have nothing to run (`skips`); each product job's condition is that it is not one of them.
"""

from __future__ import annotations

import dataclasses
import typing

from ci.tag.use_case import PRODUCTS

if typing.TYPE_CHECKING:
    from ci.changes.builds.interface import Builds
    from ci.changes.diff.interface import Diff
    from ci.changes.pending.interface import Pending

# What every product's check reads outside `crates/`, each a top-level file or directory: a change to one checks every product.
# The workflows and CI helpers (`.github`, `ci`; ci/scripts/checks runs inside the products' clippy and api tools), the tools
# the checks install and the dylint library (`tools`), the toolchain and cargo configuration (`rust-toolchain.toml`, `.cargo`,
# `clippy.toml`, `rustfmt.toml`), the root's Tastemaker package, read by every product's tests (`tastemaker`), the root's
# Tastemaker definitions and lock, which `tm check` and the products' tests read (`tastemaker.toml`, `tastemaker.lock`), the
# asymptote workspace that runs every product's check (`asymptote.toml`), and what git hands the tests that read the tree
# (`.gitattributes`, `.gitignore`).
SHARED = (
    ".github",
    "ci",
    "tools",
    "rust-toolchain.toml",
    ".cargo",
    "clippy.toml",
    "rustfmt.toml",
    "tastemaker",
    "tastemaker.toml",
    "tastemaker.lock",
    "asymptote.toml",
    ".gitattributes",
    ".gitignore",
)
# What only the `root` job's groups read, each a top-level file: the root's documents and the configuration of its repository
# checks (typos, taplo, yamllint, editorconfig, gitleaks) and of the pull request title's (commitizen). Every pull request runs
# the `root` and `pull-request` jobs, so a change to one of these is checked there and runs no product.
ROOT_ONLY = (
    "README.md",
    "LICENSE",
    "_typos.toml",
    "taplo.toml",
    ".yamllint.yaml",
    ".editorconfig",
    ".editorconfig-checker.json",
    ".gitleaks.toml",
    ".gitleaksignore",
    "cz.toml",
)

# The release's check, one row per independent build, each run for every checked product: everything the Linux job does not
# run. A row's groups share one asymptote call, so its tools share one build. `store` is whether the row keeps an asymptote store
# (not for a group whose answer stands on state outside the repository: crates.io's live index, the newest release tag); `save`
# is whether its run seeds the Rust cache (one row per machine, since all of a machine's rows share the one key).
CHECK_ROWS: tuple[dict[str, str | bool], ...] = (
    {"runner": "ubuntu-26.04", "groups": "release-clippy", "label": "release-clippy", "store": True, "save": False},
    {"runner": "macos-26", "groups": "api", "label": "api", "store": False, "save": False},
    {"runner": "ubuntu-26.04-arm", "groups": "check,release-tests", "label": "check-tests", "store": True, "save": True},
    {"runner": "macos-26", "groups": "check,release-tests", "label": "check-tests", "store": True, "save": True},
    {"runner": "windows-2025", "groups": "check,release-tests", "label": "check-tests", "store": True, "save": True},
    {"runner": "ubuntu-26.04", "groups": "release-msrv", "label": "release-msrv", "store": True, "save": False},
    {"runner": "ubuntu-26.04", "groups": "release-dylint", "label": "release-dylint", "store": True, "save": False},
    {"runner": "ubuntu-26.04", "groups": "release-hack", "label": "release-hack", "store": True, "save": False},
    {"runner": "ubuntu-26.04", "groups": "release-resolution", "label": "release-resolution", "store": False, "save": False},
)
# The row that resolves a product's packages afresh from crates.io, which waits while a product it builds against is unreleased.
RESOLUTION = "release-resolution"
# The jobs that check one product alone, by its root: ryo's Python release build and its linter benches.
OWN_JOBS = {"python-release": "ryo", "bench": "ryo"}
# Every job the changes decide, in check.yml's order: the Linux job, the release's check, and the jobs above.
DECIDED = ("check-linux", "check", *OWN_JOBS)


class Refused(Exception):
    """A request the job cannot run, said before any product is named."""


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to decide: the event that started the run, and a hand run's product."""

    event: str
    product: str


@dataclasses.dataclass(frozen=True)
class Plan:
    """What the run checks: the products, the release's check rows, and the decided jobs left with nothing to run."""

    products: tuple[str, ...]
    rows: tuple[dict[str, object], ...]
    skips: tuple[str, ...]


def owners(path: str) -> tuple[str, ...]:
    """The products a change to `path`, written from the repository's top, checks: its own, every one, or none."""
    top, _, rest = path.partition("/")
    if top == "crates":
        product = rest.partition("/")[0]
        if product in PRODUCTS and "/" in rest:
            return (product,)
        raise Refused(f"{path} is in no product's directory: {', '.join(PRODUCTS)}")
    if top in SHARED:
        return PRODUCTS
    if top in ROOT_ONLY and not rest:
        return ()
    raise Refused(f"{path} is neither shared nor the root's alone: name it in SHARED or ROOT_ONLY of ci/changes/use_case.py")


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, diff: Diff, builds: Builds, pending: Pending) -> None:
        self.diff = diff
        self.builds = builds
        self.pending = pending

    def run(self, request: Request) -> Plan:
        """What the run `request` describes checks."""
        if request.event == "pull_request":
            touched = {product for path in self.diff.paths() for product in owners(path)}
            products = tuple(product for product in PRODUCTS if product in touched)
        elif request.event == "push":
            products = PRODUCTS
        elif request.event == "workflow_dispatch":
            if request.product not in PRODUCTS:
                raise Refused(f"{request.product!r} is not a product: {', '.join(PRODUCTS)}")
            products = (request.product,)
        else:
            raise Refused(f"{request.event} starts no check")
        waiting = self.waiting(products)
        rows = tuple(
            {"product": product, "check": row}
            for product in products
            for row in CHECK_ROWS
            if not (product in waiting and row["label"] == RESOLUTION)
        )
        running = {"check-linux": bool(products), "check": bool(rows)} | {job: product in products for job, product in OWN_JOBS.items()}
        return Plan(products, rows, tuple(job for job in DECIDED if not running[job]))

    def waiting(self, products: tuple[str, ...]) -> frozenset[str]:
        """The products whose resolution row waits: each builds against a product whose version has no tag yet."""
        built_on = {product: self.builds.built_on(product) for product in products}
        if not any(built_on.values()):
            return frozenset()
        unreleased = self.pending.unreleased()
        return frozenset(product for product, others in built_on.items() if others & unreleased)
