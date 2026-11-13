"""What the `sdk_api` script promises, proven without running the tools it wraps."""

from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import subprocess
import tarfile
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location("sdk_api", Path(__file__).with_name("sdk_api.py"))
sdk_api = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sdk_api)
leaks = sdk_api.leaks

# The internal crates the leak cases are judged against, as the ryo SDK's manifest gave them before the execution crates joined it.
INTERNAL = ("ryo_core", "ryo_language", "ryo_query_language", "ryo_relations", "ryo_values")


def graph(direct: tuple[str, ...], *, manifest_path: str = "sdk/Cargo.toml", forwarded: dict[str, tuple[str, ...]] | None = None) -> dict:
    """The `workspace_members`, `packages` and `resolve` of a `cargo metadata` answer: the workspace member `demo-sdk` depends on each
    member of `direct`, each member on the members `forwarded` names, every member on the registry crate `serde`, and `demo-sdk` on
    the member `demo-bench` as a dev-dependency only."""
    forwarded = forwarded or {}
    names = set(direct) | {n for each in forwarded.values() for n in each} | {"demo_bench"}
    def member(name: str) -> str:
        return f"path+file:///w/{name}#0.1.0"
    def edge(pkg: str, kind: str | None) -> dict:
        return {"pkg": pkg, "dep_kinds": [{"kind": kind, "target": None}]}
    serde = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0"
    packages = [{"id": member("demo_sdk"), "name": "demo-sdk", "manifest_path": manifest_path, "targets": [{"name": "demo", "kind": ["lib"]}]}]
    packages += [{"id": member(n), "name": n.replace("_", "-"), "manifest_path": f"/w/{n}/Cargo.toml", "targets": [{"name": n, "kind": ["lib"]}]} for n in sorted(names)]
    packages.append({"id": serde, "name": "serde", "manifest_path": "/r/serde/Cargo.toml", "targets": [{"name": "serde", "kind": ["lib"]}]})
    nodes = [{"id": member("demo_sdk"), "deps": [edge(member(n), None) for n in direct] + [edge(member("demo_bench"), "dev"), edge(serde, None)]}]
    nodes += [{"id": member(n), "deps": [edge(member(f), None) for f in forwarded.get(n, ())] + [edge(serde, None)]} for n in sorted(names)]
    nodes.append({"id": serde, "deps": []})
    return {"workspace_members": [member("demo_sdk")] + [member(n) for n in sorted(names)], "packages": packages, "resolve": {"nodes": nodes}}

MANIFEST = """[workspace.metadata.api]
package = "demo-sdk"
library = "demo"
nightly = "nightly-2026-08-01"
baseline-rev = "abc123"
listing = "api.txt"
allow = ["demo::old"]
"""


def document(name: str, crate: str) -> dict:
    """A rustdoc JSON with one public function `demo::<name>` whose signature names one type of `crate`."""
    return {
        "external_crates": {"1": {"name": crate}},
        "paths": {"10": {"crate_id": 0, "path": ["demo", name]}, "20": {"crate_id": 1, "path": [crate, "types", "Thing"]}},
        "index": {"10": {"crate_id": 0, "visibility": "public", "name": name, "inner": {"function": {"sig": {"output": {"resolved_path": {"id": 20}}}}}}},
    }


def test_a_public_item_naming_an_internal_crates_type_is_a_leak() -> None:
    assert leaks(document("lint", "ryo_core"), [], INTERNAL) == ["demo::lint -> ryo_core::types::Thing"]
    assert leaks(document("lint", "idiomatic_sh_ryo_language"), [], INTERNAL) == ["demo::lint -> idiomatic_sh_ryo_language::types::Thing"]


def test_a_type_of_another_crate_is_no_leak() -> None:
    assert leaks(document("lint", "serde"), [], INTERNAL) == []


def test_an_allowed_path_is_not_reported() -> None:
    assert leaks(document("old", "ryo_core"), ["demo::old"], INTERNAL) == []


def method(crate: str, exported: bool) -> dict:
    """A rustdoc JSON with a public type `demo::old::Handle` whose public method `engine`, `doc(hidden)` or not, names one type of
    `crate`, and, when `exported`, a re-export of that type in the allowed module `demo::old`."""
    index = {
        "10": {"crate_id": 0, "visibility": "public", "name": "old", "inner": {"module": {"items": [11, 13]}}},
        "11": {"crate_id": 0, "visibility": "public", "name": "Handle", "inner": {"struct": {}}},
        "12": {
            "crate_id": 0,
            "visibility": "public",
            "name": "engine",
            "attrs": ["#[doc(hidden)]"],
            "inner": {"function": {"sig": {"output": {"resolved_path": {"id": 20}}}}},
        },
        "14": {"crate_id": 0, "visibility": "default", "inner": {"impl": {"for": {"resolved_path": {"id": 11}}, "items": [12]}}},
    }
    if exported:
        index["13"] = {"crate_id": 0, "visibility": "public", "name": None, "inner": {"use": {"name": "Thing", "id": 20}}}
    return {
        "external_crates": {"1": {"name": crate}},
        "paths": {
            "10": {"crate_id": 0, "path": ["demo", "old"]},
            "11": {"crate_id": 0, "path": ["demo", "old", "Handle"]},
            "20": {"crate_id": 1, "path": [crate, "types", "Thing"]},
        },
        "index": index,
    }


def test_a_hidden_method_of_an_allowed_type_naming_an_internal_type_is_a_leak() -> None:
    assert leaks(method("ryo_language", False), ["demo::old"], INTERNAL) == ["demo::old::Handle::engine -> ryo_language::types::Thing"]


def test_a_method_naming_a_type_the_allowed_module_exports_is_no_leak() -> None:
    assert leaks(method("ryo_language", True), ["demo::old"], INTERNAL) == []


def trait_impl(implemented_for: list[str], argument: list[str]) -> dict:
    """A rustdoc JSON with one trait implementation, `impl From<argument> for implemented_for`, where `implemented_for` is a type of
    the internal crate `ryo_core` and `argument` a type of the crate its path starts with."""
    crates = {"ryo_core": 1, "demo": 0, "core": 2}
    return {
        "external_crates": {"1": {"name": "ryo_core"}, "2": {"name": "core"}},
        "paths": {
            "20": {"crate_id": 1, "path": implemented_for},
            "21": {"crate_id": crates[argument[0]], "path": argument},
            "22": {"crate_id": 2, "path": ["core", "convert", "From"]},
        },
        "index": {
            "30": {
                "crate_id": 0,
                "visibility": "default",
                "inner": {
                    "impl": {
                        "is_synthetic": False,
                        "blanket_impl": None,
                        "trait": {"id": 22, "args": {"angle_bracketed": {"args": [{"type": {"resolved_path": {"id": 21}}}]}}},
                        "for": {"resolved_path": {"id": 20}},
                        "items": [],
                    }
                },
            }
        },
    }


HANDLES = ["ryo_core", "outbound", "language", "adapters", "use_case", "connection", "LanguageConnection"]


def test_a_trait_impl_for_an_internal_type_that_is_not_the_listed_one_is_a_leak() -> None:
    assert leaks(trait_impl(["ryo_core", "types", "Thing"], ["demo", "Language"]), [], INTERNAL) == ["an impl -> ryo_core::types::Thing"]


def test_the_listed_trait_impl_is_no_leak() -> None:
    assert sdk_api.ALLOWED_IMPLS == ("::".join(HANDLES),)
    assert leaks(trait_impl(HANDLES, ["demo", "Language"]), [], INTERNAL) == []


def test_the_listed_impl_naming_another_internal_type_leaks_that_type() -> None:
    assert leaks(trait_impl(HANDLES, ["ryo_core", "types", "Thing"]), [], INTERNAL) == ["an impl -> ryo_core::types::Thing"]


def test_a_product_that_names_no_api_passes_without_running_anything(tmp_path, monkeypatch) -> None:
    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **options: calls.append(argv))
    sdk_api.check(tmp_path)
    assert calls == []


# The manifests of the baseline the last check handed to semver-checks.
CHECKED: list[str] = []


def run_check(
    tmp_path: Path, monkeypatch, listing: str, rustdoc: dict, *, rustdoc_exit: int = 0, gated_listing: str = "", gated_rustdoc: dict | None = None
) -> list:
    """Runs the check against faked tools. `gated_listing` and `gated_rustdoc` are what the tools answer only when a feature is
    switched on, `--all-features`, as an item behind `#[cfg(feature = "fixtures")]` is."""
    (tmp_path / "crates/demo").mkdir(parents=True, exist_ok=True)
    (tmp_path / "crates/demo/Cargo.toml").write_text(MANIFEST)
    (tmp_path / "crates/demo/api.txt").write_text("pub fn demo::lint()\n")
    (tmp_path / "target/doc").mkdir(parents=True, exist_ok=True)
    (tmp_path / "target/doc/demo.json").write_text(json.dumps(rustdoc))
    calls = []
    baseline = []

    def fake(argv, **options):
        calls.append(argv)
        out = ""
        if "rustdoc" in argv and rustdoc_exit:
            raise subprocess.CalledProcessError(rustdoc_exit, argv)
        if "rustdoc" in argv and gated_rustdoc is not None and "--all-features" in argv:
            (tmp_path / "target/doc/demo.json").write_text(json.dumps(gated_rustdoc))
        if argv[:2] == ["git", "rev-parse"]:
            out = f"{tmp_path}\n"
        elif argv[:2] == ["cargo", "metadata"]:
            out = json.dumps(
                {
                    "target_directory": str(tmp_path / "target"),
                    **graph(INTERNAL, manifest_path=str(tmp_path / "crates/demo/sdk/Cargo.toml")),
                }
            )
        elif "public-api" in argv:
            out = listing + (gated_listing if "--all-features" in argv else "")
        elif argv[:2] == ["git", "archive"]:
            with tarfile.open(argv[argv.index("-o") + 1], "w") as tar:
                for name in ("crates/demo/Cargo.toml", "crates/demo/tastemaker/specs/Cargo.toml"):
                    entry = tarfile.TarInfo(name)
                    entry.size = 2
                    tar.addfile(entry, io.BytesIO(b"[]"))
        elif argv[:2] == ["cargo", "semver-checks"]:
            root = Path(argv[argv.index("--baseline-root") + 1])
            baseline.extend(sorted(str(each.relative_to(root)) for each in root.rglob("Cargo.toml")))
        return subprocess.CompletedProcess(argv, 0, stdout=out)

    monkeypatch.setattr(subprocess, "run", fake)
    sdk_api.check(tmp_path / "crates/demo")
    CHECKED.extend(baseline)
    return calls


def test_it_checks_the_leaks_the_listing_and_the_semver_baseline(tmp_path, monkeypatch) -> None:
    CHECKED.clear()
    calls = run_check(tmp_path, monkeypatch, "pub fn demo::lint()\n", document("lint", "serde"))
    # The SDK is named by its own manifest, and the baseline is the revision's workspace without its assets, so a fixture crate
    # of the same name in either is no ambiguity.
    assert calls[-1][:7] == ["cargo", "semver-checks", "--manifest-path", str(tmp_path / "crates/demo/sdk/Cargo.toml"), "-p", "demo-sdk", "--baseline-root"]
    assert calls[-1][7].endswith("/tree/crates/demo")
    assert ["git", "archive", "--format=tar", "-o", calls[-2][4], "abc123", "crates/demo"] == calls[-2]
    assert CHECKED == ["Cargo.toml"]
    # The rustdoc JSON the leaks are read from lists the `doc(hidden)` items too.
    assert any("rustdoc" in call and "--document-hidden-items" in call for call in calls)


def test_a_leak_fails_the_check(tmp_path, monkeypatch) -> None:
    with pytest.raises(RuntimeError, match="reaches an internal crate"):
        run_check(tmp_path, monkeypatch, "pub fn demo::lint()\n", document("lint", "ryo_core"))


def test_a_listing_that_is_not_the_recorded_one_fails_the_check(
    tmp_path, monkeypatch
) -> None:
    with pytest.raises(RuntimeError, match="not the one recorded"):
        run_check(
            tmp_path, monkeypatch, "pub fn demo::other()\n", document("lint", "serde")
        )
    evidence = tmp_path / "crates/demo/target/sdk-api"
    assert (evidence / "actual.txt").read_text() == "pub fn demo::other()\n"
    assert (evidence / "recorded.txt").read_text() == "pub fn demo::lint()\n"
    assert (tmp_path / "crates/demo/api.txt").read_text() == "pub fn demo::lint()\n"
    assert "-pub fn demo::lint()" in (evidence / "diff.txt").read_text()
    assert "+pub fn demo::other()" in (evidence / "diff.txt").read_text()
    metadata = json.loads((evidence / "metadata.json").read_text())
    assert metadata["actual_sha256"] == hashlib.sha256((evidence / "actual.txt").read_bytes()).hexdigest()
    assert metadata["comparison"] == "mismatch"
    assert metadata["nightly"] == "nightly-2026-08-01"


def test_a_feature_gated_item_is_on_the_recorded_surface(tmp_path, monkeypatch) -> None:
    with pytest.raises(RuntimeError, match="not the one recorded"):
        run_check(tmp_path, monkeypatch, "pub fn demo::lint()\n", document("lint", "serde"), gated_listing="pub mod demo::fixtures\n")
    assert "+pub mod demo::fixtures" in (tmp_path / "crates/demo/target/sdk-api/diff.txt").read_text()


def test_a_feature_gated_item_naming_an_internal_type_is_a_leak(tmp_path, monkeypatch) -> None:
    with pytest.raises(RuntimeError, match="demo::fixture -> ryo_core::types::Thing"):
        run_check(tmp_path, monkeypatch, "pub fn demo::lint()\n", document("lint", "serde"), gated_rustdoc=document("fixture", "ryo_core"))


def test_an_early_tool_failure_cannot_upload_a_stale_comparison(
    tmp_path, monkeypatch
) -> None:
    evidence = tmp_path / "crates/demo/target/sdk-api"
    evidence.mkdir(parents=True)
    for name in ("actual.txt", "recorded.txt", "diff.txt", "metadata.json"):
        (evidence / name).write_text("stale previous run")
    with pytest.raises(subprocess.CalledProcessError):
        run_check(
            tmp_path, monkeypatch, "unused", document("lint", "serde"), rustdoc_exit=1
        )
    assert list(evidence.iterdir()) == []


def test_the_top_level_is_asked_of_git_without_the_variables_a_hook_exports(tmp_path, monkeypatch) -> None:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    (tmp_path / "product").mkdir()
    monkeypatch.setenv("GIT_DIR", str(tmp_path / ".git"))
    assert sdk_api.toplevel(tmp_path / "product").resolve() == tmp_path.resolve()



def test_a_type_a_member_forwards_through_another_members_reexport_is_internal_and_leaks() -> None:
    metadata = graph(("ryo_core", "ryo_execution"), forwarded={"ryo_execution": ("ryo_execution_domain",)})
    internal = sdk_api.internal_crates(metadata, "demo-sdk")
    assert internal == ("ryo_core", "ryo_execution", "ryo_execution_domain")
    assert leaks(document("run", "ryo_execution_domain"), [], internal) == ["demo::run -> ryo_execution_domain::types::Thing"]
    assert leaks(document("run", "serde"), [], internal) == []
    assert leaks(document("run", "demo_bench"), [], internal) == []
