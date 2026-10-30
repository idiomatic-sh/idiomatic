"""What the `sdk_api` script promises, proven without running the tools it wraps."""

from __future__ import annotations

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
    assert leaks(document("lint", "ryo_core"), []) == ["demo::lint -> ryo_core::types::Thing"]
    assert leaks(document("lint", "idiomatic_sh_ryo_language"), []) == ["demo::lint -> idiomatic_sh_ryo_language::types::Thing"]


def test_a_type_of_another_crate_is_no_leak() -> None:
    assert leaks(document("lint", "serde"), []) == []


def test_an_allowed_path_is_not_reported() -> None:
    assert leaks(document("old", "ryo_core"), ["demo::old"]) == []


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
    assert leaks(method("ryo_language", False), ["demo::old"]) == ["demo::old::Handle::engine -> ryo_language::types::Thing"]


def test_a_method_naming_a_type_the_allowed_module_exports_is_no_leak() -> None:
    assert leaks(method("ryo_language", True), ["demo::old"]) == []


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


HANDLES = ["ryo_core", "outbound", "language", "types", "LanguageHandles"]


def test_a_trait_impl_for_an_internal_type_that_is_not_the_listed_one_is_a_leak() -> None:
    assert leaks(trait_impl(["ryo_core", "types", "Thing"], ["demo", "Language"]), []) == ["an impl -> ryo_core::types::Thing"]


def test_the_listed_trait_impl_is_no_leak() -> None:
    assert sdk_api.ALLOWED_IMPLS == ("::".join(HANDLES),)
    assert leaks(trait_impl(HANDLES, ["demo", "Language"]), []) == []


def test_the_listed_impl_naming_another_internal_type_leaks_that_type() -> None:
    assert leaks(trait_impl(HANDLES, ["ryo_core", "types", "Thing"]), []) == ["an impl -> ryo_core::types::Thing"]


def test_a_product_that_names_no_api_passes_without_running_anything(tmp_path, monkeypatch) -> None:
    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **options: calls.append(argv))
    sdk_api.check(tmp_path)
    assert calls == []


# The manifests of the baseline the last check handed to semver-checks.
CHECKED: list[str] = []


def run_check(tmp_path: Path, monkeypatch, listing: str, rustdoc: dict) -> list:
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
        if argv[:2] == ["git", "rev-parse"]:
            out = f"{tmp_path}\n"
        elif argv[:2] == ["cargo", "metadata"]:
            out = json.dumps(
                {
                    "target_directory": str(tmp_path / "target"),
                    "packages": [
                        {"name": "demo-sdk", "manifest_path": str(tmp_path / "crates/demo/sdk/Cargo.toml")},
                    ],
                }
            )
        elif "public-api" in argv:
            out = listing
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


def test_a_listing_that_is_not_the_recorded_one_fails_the_check(tmp_path, monkeypatch) -> None:
    with pytest.raises(RuntimeError, match="not the one recorded"):
        run_check(tmp_path, monkeypatch, "pub fn demo::other()\n", document("lint", "serde"))


def test_the_top_level_is_asked_of_git_without_the_variables_a_hook_exports(tmp_path, monkeypatch) -> None:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    (tmp_path / "product").mkdir()
    monkeypatch.setenv("GIT_DIR", str(tmp_path / ".git"))
    assert sdk_api.toplevel(tmp_path / "product").resolve() == tmp_path.resolve()
