"""The `changes` job's use case: the order of its steps over their ports, and nothing else.

A pull request checks every product whose checks read a path it changes. Each path is a product's (`crates/<product>/...`),
shared (`SHARED`: the workflows, the CI helpers, the tools, the toolchain and cargo configuration, the root's Tastemaker package
and its definitions), or outside every product beside those (`ROOT_ONLY`: the root's documents and the configuration of its own
checks). A change to a product's directory checks it and every product that reads it, closed over who reads whom: a product
reads another whose tree it builds against (`builds`, its `[patch.crates-io]`), whose directory an input of its tools names
(`manifest`, its asymptote.toml), or whose directory its tests read at run time (`RUN_TIME_READS`). A shared path checks every
product. Every change checks the products whose tests read the whole tree (`RUN_TIME_READS`' `""`), wherever its paths lie: a
product's directory (asymptote's tastemaker.lock, which Tastemaker's replay tests copy) or outside every product's, which the
`root` job, run by every pull request, reads too. A path no list names is refused, not guessed, so a new top-level file is
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
    from ci.changes.manifest.interface import Manifest
    from ci.changes.pending.interface import Pending

# What a workspace's tests read at run time beyond the packages cargo builds, each from the repository's top, swept by hand: a
# test that reads a file no path dependency reaches is invisible to cargo metadata, so it is named here, with the code that reads it.
RUN_TIME_READS = {
    "crates/ryo": [
        # cli/tests/replay/rule_fixtures.rs:40,49 reads the root's Tastemaker package.
        "tastemaker",
    ],
    # cli/tests/replay/check.rs:325-356 checks a `git archive` of the repository's HEAD, and benchmark_fixture.rs:35-46 copies
    # every tastemaker.lock of the repository: tastemaker's tests read the whole tree.
    "crates/tastemaker": [""],
    # distribution/target_test.rs:102 installs crates/tastemaker/cli, which builds ryo's crates and reads the root's Tastemaker
    # packages, tastemaker/benchmarks among them, and the targets' pin.toml copy its fixtures: the whole tree.
    "crates/asymptote": [""],
    "tools/lints": [],
}
# The products whose tests read the whole tree at run time: every change, wherever its paths lie, checks them.
WHOLE_TREE = tuple(product for product in PRODUCTS if "" in RUN_TIME_READS.get(f"crates/{product}", []))

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
# What no product's tools name, each a top-level file: the root's documents and the configuration of its repository checks
# (typos, taplo, yamllint, editorconfig, gitleaks) and of the pull request title's (commitizen). Every pull request runs the
# `root` and `pull-request` jobs, which read them; a change to one also checks the products whose tests read the whole tree.
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


def place(path: str) -> str | None:
    """Where `path`, written from the repository's top, lies: the product whose directory holds it, or None for a path outside
    every product's directory, shared or not (`shared` tells them apart)."""
    top, _, rest = path.partition("/")
    if top == "crates":
        product = rest.partition("/")[0]
        if product in PRODUCTS and "/" in rest:
            return product
        raise Refused(f"{path} is in no product's directory: {', '.join(PRODUCTS)}")
    if top in SHARED or (top in ROOT_ONLY and not rest):
        return None
    raise Refused(f"{path} is neither shared nor the root's alone: name it in SHARED or ROOT_ONLY of ci/changes/use_case.py")


def shared(path: str) -> bool:
    """Whether every product's check reads `path`."""
    return path.partition("/")[0] in SHARED


def run_time_named(product: str) -> frozenset[str]:
    """The other products whose directory, or a path inside it, `product`'s tests read at run time (`RUN_TIME_READS`)."""
    named = RUN_TIME_READS.get(f"crates/{product}", [])
    return frozenset(other for other in PRODUCTS if other != product and any(each.split("/")[:2] == ["crates", other] for each in named))


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, diff: Diff, builds: Builds, manifest: Manifest, pending: Pending) -> None:
        self.diff = diff
        self.builds = builds
        self.manifest = manifest
        self.pending = pending

    def run(self, request: Request) -> Plan:
        """What the run `request` describes checks."""
        if request.event == "pull_request":
            paths = self.diff.paths()
            places = [place(path) for path in paths]
            if any(shared(path) for path in paths):
                touched = set(PRODUCTS)
            else:
                touched = self.readers({each for each in places if each is not None}) | ({*WHOLE_TREE} if paths else set())
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

    def readers(self, changed: set[str]) -> set[str]:
        """The products `changed` and every product that reads one of them, directly or through another."""
        reads = {product: self.builds.built_on(product) | self.manifest.named(product) | run_time_named(product) for product in PRODUCTS}
        checked = set(changed)
        while grown := {product for product in PRODUCTS if product not in checked and reads[product] & checked}:
            checked |= grown
        return checked

    def waiting(self, products: tuple[str, ...]) -> frozenset[str]:
        """The products whose resolution row waits: each builds against a product whose version has no tag yet."""
        built_on = {product: self.builds.built_on(product) for product in products}
        if not any(built_on.values()):
            return frozenset()
        unreleased = self.pending.unreleased()
        return frozenset(product for product, others in built_on.items() if others & unreleased)
