# /// script
# requires-python = ">=3.12"
# ///
"""The `sdk-api` tool of a product: the public API of its SDK crate, what it leaks, what it records and what it breaks, run from
the product's directory.

    uv run ../../ci/scripts/checks/sdk_api.py

The SDK is named by `[workspace.metadata.api]` of the `Cargo.toml` in the current directory; a product that names none passes.
The SDK is read with all its features, the test-support `fixtures` feature included, so no feature widens the surface unrecorded;
see `check`.
The rustdoc JSON (written by the pinned nightly, `doc(hidden)` items included) must reach no type of an internal crate, `cargo public-api` must print the listing
the product records, and `cargo semver-checks` must find nothing broken against the SDK's baseline. The baseline is its published
crates.io version; until the first publish there is none, and the workspace at `baseline-rev` stands in for it.
"""

from __future__ import annotations

import difflib
import hashlib
import json
import os
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path

# The crates a product's SDK wraps: no type defined in one of them may be reachable from the SDK's public API.
INTERNAL = ("ryo_core", "ryo_language", "ryo_relations", "ryo_values", "ryo_query_language")

# The one trait implementation allowed to name an internal crate's type: `From<Language> for LanguageConnection`, which the command line
# and the tests of the products compose the engine with. It is `doc(hidden)` and stays public because the SDK's composition and the
# command line's registry (generated from the `languages` template) both build the engine's language connections through it.
ALLOWED_IMPLS = ("ryo_core::outbound::language::adapters::use_case::connection::LanguageConnection",)


def toplevel(directory: Path) -> Path:
    """The top level of the repository `directory` is in, asked of git without the variables a git hook exports (`GIT_DIR`, ...):
    with `GIT_DIR` and no `GIT_WORK_TREE` git answers the current directory, so a check run by the hook would find its own
    directory the repository's top."""
    clean = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
    shown = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], cwd=directory, check=True, capture_output=True, text=True, encoding="utf-8", env=clean
    ).stdout
    return Path(shown.strip())


def settings(manifest: Path) -> dict:
    """The `[workspace.metadata.api]` of a product's workspace: the SDK `package`, the `nightly` toolchain that writes rustdoc's
    JSON, the `baseline-rev` semver is checked against, the `listing` file the API is recorded in, and the `allow`ed paths."""
    if not manifest.is_file():
        return {}
    return tomllib.loads(manifest.read_text()).get("workspace", {}).get("metadata", {}).get("api", {})


def allowed_impl(document: dict, impl: dict, leaked: str) -> bool:
    """Whether `impl` is one listed in `ALLOWED_IMPLS`: an implementation for the listed internal type, whose type is the one leaked."""
    paths = document["paths"]
    target = ((impl.get("for") or {}).get("resolved_path") or {}).get("id")
    if str(target) not in paths:
        return False
    implemented_for = "::".join(paths[str(target)]["path"])
    return leaked == implemented_for and implemented_for in ALLOWED_IMPLS


def leaks(document: dict, allowed: list[str]) -> list[str]:
    """The public items of the crate in rustdoc's JSON `document`, `doc(hidden)` ones included, that name a type defined in an
    internal crate, each as `<path of the item> -> <path of the type>`, except the items whose path starts with an allowed one
    and the types the crate itself re-exports from an allowed module. A method's path is its type's, then its name."""
    external = {int(key): value["name"] for key, value in document["external_crates"].items()}
    paths = document["paths"]

    def found(node: object, into: set[int]) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "id" and isinstance(value, int):
                    into.add(value)
                else:
                    found(value, into)
        elif isinstance(node, list):
            for value in node:
                found(value, into)

    # Where each item sits: the path of the module that lists it.
    parents: dict[str, str] = {}
    for key, item in document["index"].items():
        if "module" in item["inner"] and key in paths:
            for child in item["inner"]["module"]["items"]:
                parents[str(child)] = "::".join(paths[key]["path"])
    # Where each method sits: the path of the type its impl is for. An allowed path names a module's contract, not what a method of
    # one of its types hands out: a method is held to the surface whatever its type.
    methods: set[str] = set()
    for item in document["index"].values():
        target = ((item["inner"].get("impl") or {}).get("for") or {}).get("resolved_path") or {}
        if str(target.get("id")) in paths:
            for child in item["inner"]["impl"]["items"]:
                parents[str(child)] = "::".join(paths[str(target["id"])]["path"])
                methods.add(str(child))
    # The types an allowed module re-exports are the SDK's own surface, wherever they are named.
    surface = {
        (item["inner"]["use"] or {}).get("id")
        for key, item in document["index"].items()
        if "use" in item["inner"] and any(parents.get(key, "").startswith(prefix) for prefix in allowed)
    }
    leaked = []
    for key, item in document["index"].items():
        # An impl block has no visibility of its own: it is public API when it is not synthetic or blanket, so its trait and type are
        # held to the surface too.
        impl = item["inner"].get("impl")
        if impl is not None:
            if item.get("crate_id") != 0 or impl.get("is_synthetic") or impl.get("blanket_impl"):
                continue
        elif item.get("crate_id") != 0 or item.get("visibility") != "public":
            continue
        named: set[int] = set()
        found(item["inner"], named)
        if key in paths:
            where = "::".join(paths[key]["path"])
        elif key in parents:
            where = f"{parents[key]}::{item.get('name') or (item['inner'].get('use') or {}).get('name')}"
        else:
            where = item.get("name") or "an impl"
        if key not in methods and any(where.startswith(prefix) for prefix in allowed):
            continue
        for target in sorted(named - surface):
            entry = paths.get(str(target))
            if entry and entry["crate_id"] != 0 and external.get(entry["crate_id"], "").replace("idiomatic_sh_", "") in INTERNAL:
                if impl is not None and allowed_impl(document, impl, "::".join(entry["path"])):
                    continue
                leaked.append(f"{where} -> {'::'.join(entry['path'])}")
    return sorted(set(leaked))


def check(directory: Path) -> None:
    """Holds the SDK of the product in `directory` to its recorded public API: no leak of an internal crate's type, the recorded
    listing, and no semver break against the baseline. Passes when the product names no SDK."""
    api = settings(directory / "Cargo.toml")
    evidence = directory / "target/sdk-api"
    # Only this check owns these names. Never publish a previous run's comparison after an earlier tool failure.
    for name in ("actual.txt", "recorded.txt", "diff.txt", "metadata.json"):
        (evidence / name).unlink(missing_ok=True)
    if not api:
        return
    root = toplevel(directory)
    workspace = directory.resolve().relative_to(root.resolve()).as_posix()
    package, nightly = api["package"], api["nightly"]
    # All features: a feature-gated item is public API to whoever switches its feature on, so the leaks and the listing are read
    # with every feature, `fixtures` included. `fixtures` is test support (tokio's `test-util` is the same practice) and may change
    # without notice, so semver is checked on the default features alone, the surface a shipped build compiles; what keeps
    # `fixtures` out of a shipped build is Tastemaker's `fixtures-feature` rules (the crate spec's and the language spec's, and
    # `workspace-fixtures-feature` for `[workspace.dependencies]`).
    subprocess.run(
        [
            "cargo", f"+{nightly}", "rustdoc", "--lib", "-p", package, "--all-features",
            "--", "-Z", "unstable-options", "--output-format", "json", "--document-hidden-items",
        ],
        cwd=directory,
        check=True,
    )
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=directory, check=True, capture_output=True, text=True, encoding="utf-8"
        ).stdout
    )
    target = Path(metadata["target_directory"])
    document = json.loads((target / "doc" / f"{api['library']}.json").read_text())
    found = leaks(document, api.get("allow", []))
    if found:
        raise RuntimeError("the SDK's public API reaches an internal crate's type:\n  " + "\n  ".join(found))
    listing = subprocess.run(
        ["cargo", f"+{nightly}", "public-api", "-ss", "-p", package, "--all-features"], cwd=directory, check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout
    recorded = (directory / api["listing"]).read_text(encoding="utf-8")
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / "actual.txt").write_text(listing, encoding="utf-8", newline="\n")
    (evidence / "recorded.txt").write_text(recorded, encoding="utf-8", newline="\n")
    difference = "".join(
        difflib.unified_diff(
            recorded.splitlines(keepends=True),
            listing.splitlines(keepends=True),
            fromfile=api["listing"],
            tofile="generated public API",
        )
    )
    (evidence / "diff.txt").write_text(difference, encoding="utf-8", newline="\n")
    (evidence / "metadata.json").write_text(
        json.dumps(
            {
                "package": package,
                "nightly": nightly,
                "listing": api["listing"],
                "comparison": "match" if listing == recorded else "mismatch",
                "source_commit": os.environ.get("GITHUB_SHA"),
                "run_id": os.environ.get("GITHUB_RUN_ID"),
                "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
                "recorded_sha256": hashlib.sha256(recorded.encode("utf-8")).hexdigest(),
                "actual_sha256": hashlib.sha256(listing.encode("utf-8")).hexdigest(),
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    if listing != recorded:
        raise RuntimeError(
            f"the SDK's public API is not the one recorded in {api['listing']}: record it with `cargo +{nightly} public-api -ss -p {package} --all-features` after the change is meant; comparison evidence: {evidence}"
        )
    # The SDK's own manifest, from the workspace's metadata: semver-checks otherwise finds a package by name anywhere
    # beneath the directory, a fixture crate of the same name included.
    manifest = next(each["manifest_path"] for each in metadata["packages"] if each["name"] == package)
    # The baseline is the workspace as it was at `baseline-rev`, without its asset folder (`tastemaker/`, named `canonical/`
    # in a baseline from before the product was renamed): the fixture crates in it may
    # carry the SDK's own name, which semver-checks cannot tell from the SDK when it finds a package by name in a clone.
    with tempfile.TemporaryDirectory(prefix="baseline-") as scratch:
        archive, tree = Path(scratch) / "baseline.tar", Path(scratch) / "tree"
        subprocess.run(["git", "archive", "--format=tar", "-o", str(archive), api["baseline-rev"], workspace], cwd=root, check=True)
        with tarfile.open(archive) as tar:
            tar.extractall(tree, members=[each for each in tar if not each.name.startswith((f"{workspace}/tastemaker/", f"{workspace}/canonical/"))], filter="data")
        subprocess.run(
            ["cargo", "semver-checks", "--manifest-path", manifest, "-p", package, "--baseline-root", str(tree / workspace)],
            cwd=directory,
            check=True,
        )


if __name__ == "__main__":
    check(Path.cwd())
