"""What the cargo adapter of `publish` promises beside the port's contract, proven
without running the tool it wraps, and the order it publishes in, proven over this
repository's own Cargo metadata."""

from __future__ import annotations

import dataclasses
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

import pytest

from ci.release.publish.adapters.cargo import adapter
from ci.release.publish.adapters.cargo.adapter import (
    DEFAULT_WAIT_SECONDS,
    CargoPublish,
    cargo_metadata,
    owned,
    cargo_publish,
    patches,
    publication_order,
    rate_limit_wait,
)
from ci.release.publish.interface_test import conforms
from ci.release.use_case import Artifact, Product

ROOT = Path(__file__).resolve().parents[5]


def test_the_cargo_adapter_implements_the_port() -> None:
    """It is a `publish` adapter."""
    conforms(CargoPublish)


def dependency(name: str, kind: str | None = None, req: str = "^1.0.0", path: bool = True) -> dict:
    return {"name": name, "kind": kind, "req": req, "path": f"/checkout/{name}" if path else None}


def package(name: str, *dependencies: dict, publish: list[str] | None = None, source: str | None = None) -> dict:
    return {
        "name": name,
        "version": "1.0.0",
        "source": source,
        "publish": publish,
        "manifest_path": f"/checkout/{name}/Cargo.toml",
        "dependencies": list(dependencies),
    }


def names(order: list[dict]) -> list[str]:
    return [each["name"] for each in order]


# cli needs core and, through it, base; a feature-only language needs base; `unused` is not reached.
METADATA = {
    "workspace_root": "/checkout",
    "packages": [
        package("cli", dependency("core"), dependency("language", None), dependency("serde", path=False)),
        package("core", dependency("base"), dependency("core", "dev", "*")),
        package("base"),
        package("language", dependency("base")),
        package("unused", dependency("base")),
        package("serde", source="registry+https://github.com/rust-lang/crates.io-index"),
    ]
}


def test_the_dependencies_come_before_the_package_that_needs_them() -> None:
    assert names(publication_order(METADATA, "cli")) == ["base", "core", "language", "cli"]


def test_only_the_packages_the_command_line_reaches_are_published() -> None:
    assert "unused" not in names(publication_order(METADATA, "cli"))
    assert names(publication_order(METADATA, "base")) == ["base"]


def test_a_registry_dependency_is_not_published() -> None:
    assert "serde" not in names(publication_order(METADATA, "cli"))


def test_a_version_the_workspace_patches_to_the_checkout_is_published_first() -> None:
    """A dependency that names a version and no path, which the workspace's `[patch.crates-io]` resolves to the checkout's directory
    (tastemaker's on ryo's crates), is the checkout's: it must be on crates.io before the package that needs it."""
    patched = {"workspace_root": "/checkout", "packages": [package("cli", dependency("sdk", path=False)), package("sdk")]}
    assert names(publication_order(patched, "cli")) == ["sdk", "cli"]


def test_a_development_dependency_with_a_version_is_published_first_and_one_without_is_not() -> None:
    kept = {"packages": [package("a", dependency("b", "dev", "^1.0.0")), package("b")]}
    stripped = {"packages": [package("a", dependency("b", "dev", "*")), package("b")]}
    assert names(publication_order(kept, "a")) == ["b", "a"]
    assert names(publication_order(stripped, "a")) == ["a"]


def test_the_order_does_not_depend_on_the_order_cargo_lists_the_packages_in() -> None:
    reversed_metadata = {"packages": list(reversed(METADATA["packages"]))}
    assert publication_order(reversed_metadata, "cli") == publication_order(METADATA, "cli")


def test_a_dependency_that_cannot_be_published_is_refused() -> None:
    metadata = {"packages": [package("a", dependency("b")), package("b", publish=[])]}
    with pytest.raises(ValueError, match="b is needed by a and says publish = false"):
        publication_order(metadata, "a")


def test_a_cycle_is_refused() -> None:
    metadata = {"packages": [package("a", dependency("b")), package("b", dependency("a"))]}
    with pytest.raises(ValueError, match="cycle"):
        publication_order(metadata, "a")


def test_the_patches_resolve_each_package_from_its_directory() -> None:
    assert patches([package("a"), package("b")]) == (
        '[patch.crates-io]\n"a" = { path = "/checkout/a" }\n"b" = { path = "/checkout/b" }\n'
    )


def test_a_dry_run_uploads_nothing_and_resolves_the_unpublished_dependencies() -> None:
    each = package("a")
    assert cargo_publish(each, True, Path("p.toml")) == [
        "cargo", "publish", "--locked", "--manifest-path", "/checkout/a/Cargo.toml", "--dry-run", "--config", "p.toml",
    ]  # fmt: skip
    assert cargo_publish(each, False, Path("p.toml")) == [
        "cargo", "publish", "--locked", "--manifest-path", "/checkout/a/Cargo.toml",
    ]  # fmt: skip


PRODUCT = Product("demo", "cli", "1.0.0", "MIT", "", True, "demo", ())


class Cargo:
    """The two programs the adapter runs, recorded: metadata answers `METADATA`."""

    def __init__(self) -> None:
        self.publishes: list[list[str]] = []
        # The refusals the next publishes answer with, in turn, before they succeed.
        self.refusals: list[tuple[int, str]] = []

    def __call__(self, argv: list[str], **_: object) -> subprocess.CompletedProcess:
        if argv[:2] == ["cargo", "metadata"]:
            return subprocess.CompletedProcess(argv, 0, json.dumps(METADATA), "")
        self.publishes.append(argv)
        if self.refusals:
            code, said = self.refusals.pop(0)
            return subprocess.CompletedProcess(argv, code, "", said)
        return subprocess.CompletedProcess(argv, 0, "", "")

    def published(self) -> list[str]:
        return [argv[argv.index("--manifest-path") + 1].split("/")[-2] for argv in self.publishes]


@pytest.fixture
def cargo(monkeypatch: pytest.MonkeyPatch) -> Cargo:
    recorded = Cargo()
    monkeypatch.setattr(subprocess, "run", recorded)
    monkeypatch.setattr(adapter, "on_crates_io", lambda name, version: False)
    monkeypatch.delenv("CARGO_REGISTRY_TOKEN", raising=False)
    return recorded


def test_a_dry_run_publishes_every_crate_in_order_without_a_token(tmp_path, cargo) -> None:
    CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), True)
    assert cargo.published() == ["base", "core", "language", "cli"]
    assert all("--dry-run" in argv for argv in cargo.publishes)


def test_an_upload_publishes_every_crate_in_order_and_needs_the_token(tmp_path, cargo, monkeypatch) -> None:
    artifact = Artifact("cargo", tmp_path, "shared")
    with pytest.raises(RuntimeError, match="no crates.io token"):
        CargoPublish().publish(PRODUCT, artifact, False)
    assert cargo.publishes == []
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    CargoPublish().publish(PRODUCT, artifact, False)
    assert cargo.published() == ["base", "core", "language", "cli"]
    assert not any("--dry-run" in argv for argv in cargo.publishes)


def test_a_crate_one_product_published_is_not_published_again_for_the_next(tmp_path, cargo, monkeypatch) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    artifact = Artifact("cargo", tmp_path, "shared")
    job = CargoPublish()
    job.publish(PRODUCT, artifact, False)
    job.publish(dataclasses.replace(PRODUCT, package="language"), artifact, False)
    assert cargo.published() == ["base", "core", "language", "cli"]


def test_a_crate_one_product_dry_ran_is_not_dry_run_again_for_the_next(tmp_path, cargo) -> None:
    artifact = Artifact("cargo", tmp_path, "shared")
    job = CargoPublish()
    job.publish(PRODUCT, artifact, True)
    job.publish(dataclasses.replace(PRODUCT, package="language"), artifact, True)
    assert cargo.published() == ["base", "core", "language", "cli"]


def test_a_dry_run_does_not_stand_for_the_upload(tmp_path, cargo, monkeypatch) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    artifact = Artifact("cargo", tmp_path, "shared")
    job = CargoPublish()
    job.publish(PRODUCT, artifact, True)
    job.publish(PRODUCT, artifact, False)
    assert cargo.published() == ["base", "core", "language", "cli"] * 2


def test_a_crate_whose_manifest_says_publish_false_runs_no_publish(tmp_path, cargo, capsys) -> None:
    unpublishable = dataclasses.replace(PRODUCT, publishable=False)
    for dry_run in (True, False):
        CargoPublish().publish(unpublishable, Artifact("cargo", tmp_path, "shared"), dry_run)
    assert cargo.publishes == []
    assert "not yet possible" in capsys.readouterr().out


def test_another_product_s_crate_is_patched_in_but_never_published(tmp_path, cargo, monkeypatch, capsys) -> None:
    foreign = dict(package("base"), manifest_path="/elsewhere/base/Cargo.toml")
    monkeypatch.setitem(METADATA, "packages", [foreign if each["name"] == "base" else each for each in METADATA["packages"]])
    assert not owned(METADATA, foreign) and owned(METADATA, package("cli"))
    CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), True)
    assert cargo.published() == ["core", "language", "cli"]
    assert "base 1.0.0: another product's crate" in capsys.readouterr().out


def test_a_crate_version_already_on_crates_io_is_not_uploaded_again(tmp_path, cargo, monkeypatch, capsys) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    monkeypatch.setattr(adapter, "on_crates_io", lambda name, version: name in {"base", "core"})
    CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), False)
    assert cargo.published() == ["language", "cli"]
    assert "base 1.0.0: already on crates.io" in capsys.readouterr().out


def test_a_dry_run_dry_runs_every_crate_even_when_it_is_on_crates_io(tmp_path, cargo, monkeypatch) -> None:
    monkeypatch.setattr(adapter, "on_crates_io", lambda name, version: True)
    CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), True)
    assert cargo.published() == ["base", "core", "language", "cli"]


REFUSED = (
    "error: failed to publish to registry at https://crates.io\n\nCaused by:\n  the remote server responded with an error "
    "(status 429 Too Many Requests): You have published too many new crates in a short period of time. Please try again "
    "after Fri, 30 Oct 2026 17:10:00 GMT or email help@crates.io to have your limit increased."
)


def test_the_rate_limit_waits_until_the_time_crates_io_names() -> None:
    now = datetime(2026, 10, 30, 17, 0, 0, tzinfo=timezone.utc)
    assert rate_limit_wait(REFUSED, now) == 600 + 15
    assert rate_limit_wait("status 429 Too Many Requests", now) == DEFAULT_WAIT_SECONDS
    assert rate_limit_wait("error: some other failure", now) is None


def test_a_rate_limited_upload_waits_and_uploads_again(tmp_path, cargo, monkeypatch) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    waits: list[float] = []
    monkeypatch.setattr(adapter.time, "sleep", waits.append)
    cargo.refusals = [(101, REFUSED)]
    CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), False)
    assert cargo.published() == ["base", "base", "core", "language", "cli"]
    assert len(waits) == 1


def test_another_failure_stops_the_publish(tmp_path, cargo, monkeypatch) -> None:
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "from-the-release-environment")
    cargo.refusals = [(101, "error: the manifest is invalid")]
    with pytest.raises(subprocess.CalledProcessError):
        CargoPublish().publish(PRODUCT, Artifact("cargo", tmp_path, "shared"), False)
    assert cargo.published() == ["base"]


COMMAND_LINES = {
    "tastemaker": "idiomatic-sh-tastemaker",
    "ryo": "idiomatic-sh-ryo",
    "asymptote": "idiomatic-sh-asymptote",
}


@pytest.mark.parametrize(("product", "package"), COMMAND_LINES.items())
def test_each_products_real_order_puts_every_dependency_before_its_dependent(product: str, package: str) -> None:
    """Over this repository's own Cargo metadata, not a list: every crate the command line
    needs is before the crate that needs it, and the command line is last."""
    metadata = cargo_metadata(ROOT / "crates" / product / "cli" / "Cargo.toml")
    order = publication_order(metadata, package)
    position = {each["name"]: index for index, each in enumerate(order)}
    assert names(order)[-1] == package
    for each in order:
        for needed in each["dependencies"]:
            if needed["name"] in position and needed["name"] != each["name"] and needed["req"] != "*":
                assert position[needed["name"]] < position[each["name"]], f"{needed['name']} before {each['name']}"
        assert each["publish"] != [], f"{each['name']} is needed and cannot be published"
        assert each["description"] and each["license"] == "MIT" and each["repository"], each["name"]
        assert not [d for d in each["dependencies"] if d["req"] == "*" and d["kind"] != "dev"], each["name"]


@pytest.mark.parametrize(("product", "package"), COMMAND_LINES.items())
def test_each_products_real_order_holds_every_crate_of_this_checkout_the_command_line_links(product: str, package: str) -> None:
    """Completeness, from Cargo's resolved graph and not from the dependency lists the order
    is built from: every crate of this checkout that the command line's normal and build
    dependencies reach, renamed ones included, is published."""
    metadata = cargo_metadata(ROOT / "crates" / product / "cli" / "Cargo.toml")
    local = {each["id"]: each["name"] for each in metadata["packages"] if each["source"] is None}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = next(identity for identity, name in local.items() if name == package)
    reached, pending = set(), [root]
    while pending:
        identity = pending.pop()
        if identity in reached:
            continue
        reached.add(identity)
        pending += [
            dep["pkg"]
            for dep in nodes[identity]["deps"]
            if dep["pkg"] in local and any(kind["kind"] != "dev" for kind in dep["dep_kinds"])
        ]
    published = set(names(publication_order(metadata, package)))
    assert {local[identity] for identity in reached} <= published


@pytest.mark.parametrize(("product", "package"), COMMAND_LINES.items())
def test_each_product_publishes_only_its_own_crates(product: str, package: str) -> None:
    """Each product is released on its own: of the crates its command line needs, it publishes those of its own workspace, all
    named for it (ryo's every `idiomatic-sh-ryo-*`, tastemaker's and asymptote's their command line and core), and none of another
    product's (tastemaker's ryo crates come from ryo's release)."""
    metadata = cargo_metadata(ROOT / "crates" / product / "cli" / "Cargo.toml")
    order = publication_order(metadata, package)
    own = {each["name"] for each in order if owned(metadata, each)}
    assert own and all(name == package or name.startswith(f"{package}-") for name in own)
    assert not [each["name"] for each in order if not owned(metadata, each) and each["name"].startswith(f"{package}-")]
