"""The `publish` step done through Cargo, crates.io's publish."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

from ci.release.publish.interface import Publish
from ci.release.use_case import Artifact, Product

# A dependency on a package of this checkout names a `path`; Cargo reports a dependency with no
# version requirement as `*`, and strips such a development dependency when it packages.
UNVERSIONED = "*"


def local_dependencies(package: dict, local: dict[str, dict]) -> set[str]:
    """The names of the checkout's packages that `package` needs on crates.io before it can
    be published: its normal and build dependencies that name a path, and its development
    dependencies that name a path and a version (Cargo keeps those in the published manifest
    and strips the rest)."""
    return {
        dependency["name"]
        for dependency in package["dependencies"]
        if dependency.get("path")
        and dependency["name"] != package["name"]
        and dependency["name"] in local
        and (dependency["kind"] != "dev" or dependency["req"] != UNVERSIONED)
    }


def publication_order(metadata: dict, package: str) -> list[dict]:
    """Every package that must be published for `package` to be, itself last: the checkout's
    packages it reaches through the dependencies Cargo reports, each after the ones it
    depends on, ties broken by name so that the order is the same on every machine. A
    package that cannot be published, or a cycle, is refused."""
    local = {each["name"]: each for each in metadata["packages"] if each["source"] is None}
    needs: dict[str, set[str]] = {}
    pending = [package]
    while pending:
        name = pending.pop()
        if name in needs:
            continue
        if local[name]["publish"] == []:
            raise ValueError(f"{name} is needed by {package} and says publish = false")
        needs[name] = local_dependencies(local[name], local)
        pending += needs[name]
    order: list[dict] = []
    done: set[str] = set()
    while len(done) < len(needs):
        ready = sorted(name for name in needs if name not in done and needs[name] <= done)
        if not ready:
            raise ValueError(f"{package} has a dependency cycle among {sorted(set(needs) - done)}")
        done.add(ready[0])
        order.append(local[ready[0]])
    return order


def cargo_metadata(manifest: Path) -> dict:
    """Cargo's metadata of the package at `manifest`, with its dependencies."""
    said = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--manifest-path", str(manifest)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(said)


def patches(packages: list[dict]) -> str:
    """Cargo configuration that resolves `packages` from their directories, so that a dry run
    finds the versions that are not on crates.io yet."""
    lines = ["[patch.crates-io]"]
    lines += [f"{json.dumps(each['name'])} = {{ path = {json.dumps(str(Path(each['manifest_path']).parent))} }}" for each in packages]
    return "\n".join(lines) + "\n"


def cargo_publish(package: dict, dry_run: bool, config: Path | None = None) -> list[str]:
    """Cargo's publish of one package, from its own manifest: its dry run (against the
    `config` that resolves the unpublished dependencies), or an upload with the token
    crates.io gives the workflow."""
    argv = ["cargo", "publish", "--locked", "--manifest-path", package["manifest_path"]]
    if dry_run:
        argv.append("--dry-run")
        if config is not None:
            argv += ["--config", str(config)]
    return argv


class CargoPublish(Publish):
    """`publish` through Cargo: the command line crate and every crate of this checkout it
    depends on, dependencies first, the order read from Cargo's metadata. A crate published
    for one product is not published again for another in the same run, and neither is a crate
    already dry-run: another product's dry run would resolve it against a lockfile that is not
    its own, and `--locked` refuses. While the manifest
    says `publish = false`, Cargo refuses even the dry run for that reason alone, so neither
    runs and the job says it is not yet possible (the `distribution` spec); the manifest's
    own key is the switch."""

    def __init__(self) -> None:
        # The crates already handed to Cargo in this run, by mode: dry-run (`True`) or published (`False`).
        self.done: dict[bool, set[str]] = {True: set(), False: set()}

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        if not product.publishable:
            print(f"{product.package}: crates.io is not yet possible: the manifest says publish = false", flush=True)
            return
        if not dry_run and not os.environ.get("CARGO_REGISTRY_TOKEN"):
            # The workflow hands the token: the `release` environment's secret on the first
            # run, crates.io's trusted publishing (its auth action) after.
            raise RuntimeError(f"{product.package}: no crates.io token: the workflow handed none")
        order = publication_order(cargo_metadata(artifact.path / "Cargo.toml"), product.package)
        with tempfile.TemporaryDirectory(prefix="publish-") as scratch:
            config = Path(scratch) / "patches.toml"
            config.write_text(patches(order))
            for package in order:
                if package["name"] in self.done[dry_run]:
                    continue
                print(f"{package['name']} {package['version']}: cargo publish{' --dry-run' if dry_run else ''}", flush=True)
                subprocess.run(cargo_publish(package, dry_run, config), cwd=artifact.path, check=True)
                self.done[dry_run].add(package["name"])
