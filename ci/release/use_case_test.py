"""What the `release` job's use case promises, over fakes of every port."""

from __future__ import annotations

from pathlib import Path

import pytest

from ci.release.build.interface import Build
from ci.release.manifest.interface import Manifest
from ci.release.publish.interface import Publish
from ci.release.smoke.interface import Smoke
from ci.release.use_case import SHARED, Artifact, Product, Refused, Registry, Request, UseCase

HOST = "aarch64-unknown-linux-gnu"
TARGETS = ("aarch64-apple-darwin", HOST, "x86_64-unknown-linux-gnu")


def make_product(name: str) -> Product:
    return Product(name, f"{name}-cli", "0.1.0", "MIT", "https://example.invalid/repo", False, name, TARGETS)


class FakeManifest(Manifest):
    def __init__(self, shipped: tuple[str, ...] = ("alpha", "beta"), host: str = HOST) -> None:
        self.names = shipped
        self.machine = host

    def shipped(self) -> tuple[str, ...]:
        return self.names

    def read(self, product: str) -> Product:
        return make_product(product)

    def host(self) -> str:
        return self.machine


class Journal:
    """Every call the fakes receive, in order."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []


class FakeBuild(Build):
    """Builds one package for a target and one for what every target shares, as npm does."""

    def __init__(self, registry: str, journal: Journal) -> None:
        self.registry = registry
        self.journal = journal

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        self.journal.calls.append(("build", self.registry, product.name, scope))
        return [Artifact(self.registry, into / f"{product.name}-{scope}", scope)]


class FakeSmoke(Smoke):
    def __init__(self, registry: str, journal: Journal, fails: bool = False) -> None:
        self.registry = registry
        self.journal = journal
        self.fails = fails

    def smoke(self, product: Product, artifacts: list[Artifact], into: Path) -> None:
        self.journal.calls.append(("smoke", self.registry, product.name, len(artifacts)))
        if self.fails:
            raise RuntimeError(f"{self.registry} does not install")


class FakePublish(Publish):
    def __init__(self, registry: str, journal: Journal) -> None:
        self.registry = registry
        self.journal = journal

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        self.journal.calls.append(("publish", self.registry, product.name, artifact.scope, dry_run))


def registries(journal: Journal, failing: str = "") -> dict[str, Registry]:
    return {
        name: Registry(
            FakeBuild(name, journal),
            # Cargo's package is the source itself and the release's archives are uploaded by the workflow: nothing to install.
            None if name in ("cargo", "github") else FakeSmoke(name, journal, name == failing),
            # The archives upload through the workflow.
            None if name == "github" else FakePublish(name, journal),
        )
        for name in ("cargo", "npm", "pypi", "github")
    }


def request(dry_run: bool = True, products: tuple[str, ...] = ()) -> Request:
    return Request(products, dry_run, Path("/scratch"))


def test_a_dry_run_builds_smokes_and_dry_publishes_every_registry_of_every_product_for_every_target() -> None:
    journal = Journal()
    assert UseCase(FakeManifest(), registries(journal)).run(request()) == 0
    built = [call for call in journal.calls if call[0] == "build"]
    assert built == [
        ("build", registry, name, scope)
        for registry in ("cargo", "npm", "pypi", "github")
        for name in ("alpha", "beta")
        for scope in (*TARGETS, SHARED)
    ]
    published = [call for call in journal.calls if call[0] == "publish"]
    assert len(published) == 2 * 3 * (len(TARGETS) + 1)
    assert all(call[4] is True for call in published), "a dry run uploads nothing"


def test_only_what_this_machine_can_run_is_smoked() -> None:
    journal = Journal()
    UseCase(FakeManifest(("alpha",)), registries(journal)).run(request())
    smoked = [call for call in journal.calls if call[0] == "smoke"]
    assert smoked == [("smoke", "npm", "alpha", 2), ("smoke", "pypi", "alpha", 2)], "the host's package and what every target shares"


def test_each_registry_builds_then_smokes_before_anything_is_published() -> None:
    journal = Journal()
    UseCase(FakeManifest(("alpha",)), registries(journal)).run(request())
    kinds = [call[0] for call in journal.calls]
    assert "publish" not in kinds[: kinds.index("smoke")]
    assert kinds.index("publish") > max(index for index, kind in enumerate(kinds) if kind in ("build", "smoke"))


def test_a_publish_dry_runs_everything_first_then_uploads_crates_io_before_npm_and_pypi() -> None:
    journal = Journal()
    UseCase(FakeManifest(("alpha",)), registries(journal)).run(request(dry_run=False))
    published = [call for call in journal.calls if call[0] == "publish"]
    each = len(TARGETS) + 1
    assert [call[4] for call in published[: 3 * each]] == [True] * (3 * each), "the dry run passes for everything first"
    assert [call[1] for call in published[3 * each :]] == ["cargo"] * each + ["npm"] * each + ["pypi"] * each
    assert [call[4] for call in published[3 * each :]] == [False] * (3 * each)


def test_a_failing_smoke_publishes_nothing() -> None:
    journal = Journal()
    with pytest.raises(RuntimeError):
        UseCase(FakeManifest(("alpha",)), registries(journal, failing="pypi")).run(request(dry_run=False))
    assert not [call for call in journal.calls if call[0] == "publish"]


def test_only_the_named_products_are_released() -> None:
    journal = Journal()
    UseCase(FakeManifest(), registries(journal)).run(request(products=("beta",)))
    assert {call[2] for call in journal.calls} == {"beta"}


def test_nothing_shipped_is_refused() -> None:
    with pytest.raises(Refused, match="no product ships"):
        UseCase(FakeManifest(()), registries(Journal())).run(request())


class EmptyBuild(Build):
    """A registry the product has no package for."""

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        return []


def test_a_product_with_nothing_to_install_is_not_smoked() -> None:
    journal = Journal()
    nothing = registries(journal)
    nothing["npm"] = Registry(EmptyBuild(), FakeSmoke("npm", journal), FakePublish("npm", journal))
    UseCase(FakeManifest(("alpha",)), nothing).run(request())
    assert not [call for call in journal.calls if call[:2] == ("smoke", "npm")]
