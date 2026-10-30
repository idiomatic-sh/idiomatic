"""What `tools/install.py` promises: each tool is declared once, and every command a check runs bare is one it installs."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("install", ROOT / "tools/install.py")
install = importlib.util.module_from_spec(SPEC)
# A dataclass looks its module up by name while it is defined.
sys.modules["install"] = install
SPEC.loader.exec_module(install)

# Commands the machine provides before any tool is installed.
SYSTEM = {"cargo", "bash", "git", "uv", "sh", "jq"}


def manifest(path: Path, tools: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"[workspace.metadata.bin]\n{tools}\n")


def test_a_tool_two_products_declare_alike_is_refused(tmp_path) -> None:
    manifest(tmp_path / "crates/b/Cargo.toml", 'cargo-deny = { version = "0.20.2", locked = true }')
    manifest(tmp_path / "crates/a/Cargo.toml", 'cargo-deny = { version = "0.20.2", locked = true }')
    with pytest.raises(SystemExit):
        install.declared_crates(tmp_path, shared=[])


def test_a_tool_the_root_declares_is_refused_in_a_product(tmp_path) -> None:
    manifest(tmp_path / "crates/a/Cargo.toml", 'cargo-deny = { version = "0.20.2", locked = true }')
    with pytest.raises(SystemExit):
        install.declared_crates(tmp_path, shared=[install.Crate("cargo-deny", "0.20.2")])


def test_every_tool_is_declared_in_one_place() -> None:
    names = [crate.name for crate in [*install.ROOT_CRATES, *install.declared_crates()]]
    assert len(names) == len(set(names)), sorted(names)


def test_a_path_is_read_relative_to_the_manifest_that_declares_it(tmp_path) -> None:
    manifest(tmp_path / "crates/a/Cargo.toml", 'tool = { version = "0.1.0", path = "cli", bins = ["t"] }')
    (crate,) = install.declared_crates(tmp_path, shared=[])
    assert crate.path == (tmp_path / "crates/a/cli").resolve()


def test_the_installed_crates_are_read_from_cargo(monkeypatch) -> None:
    listing = "cargo-deny v0.20.2:\n    cargo-deny\ntm v0.1.0 (/work/crates/tastemaker/cli):\n    tm\n"
    monkeypatch.setattr(install, "run", lambda *a, **k: subprocess.CompletedProcess(a, 0, listing, ""))
    assert install.installed_crates() == {"cargo-deny": ("0.20.2", ""), "tm": ("0.1.0", "/work/crates/tastemaker/cli")}


def test_a_crate_is_stale_when_missing_or_at_another_version() -> None:
    crate = install.Crate("cargo-deny", "0.20.2")
    assert install.stale_crate(crate, {}) == "not installed"
    assert install.stale_crate(crate, {"cargo-deny": ("0.19.0", "")}) == "installed 0.19.0, declared 0.20.2"
    assert install.stale_crate(crate, {"cargo-deny": ("0.20.2", "")}) is None


def test_a_caret_pin_is_met_by_any_version_it_allows() -> None:
    assert install.meets("0.1.7", "^0.1") and install.meets("0.1.0", "0.1")
    assert not install.meets("0.2.0", "^0.1") and not install.meets("0.0.9", "^0.1")
    assert install.meets("1.4.2", "^1.2") and not install.meets("2.0.0", "^1.2")
    assert install.meets("0.2.3", "0.2.3") and not install.meets("0.2.4", "0.2.3")
    crate = install.Crate("idiomatic-sh-asymptote", "^0.1")
    assert install.stale_crate(crate, {"idiomatic-sh-asymptote": ("0.1.9", "")}) is None
    assert install.stale_crate(crate, {"idiomatic-sh-asymptote": ("0.2.0", "")}) == "installed 0.2.0, declared ^0.1"


def test_asymptote_is_the_version_the_root_asymptote_toml_pins(tmp_path) -> None:
    (tmp_path / "asymptote.toml").write_text('asymptote = "9.9.9"\nmembers = ["crates/*"]\n')
    assert install.pinned("asymptote", tmp_path) == "9.9.9"
    (crate,) = [crate for crate in install.ROOT_CRATES if crate.name == "idiomatic-sh-asymptote"]
    assert (crate.version, crate.bins) == (install.pinned("asymptote"), ("asymptote",))
    assert "idiomatic-sh-asymptote" not in {crate.name for crate in install.declared_crates()}


def test_the_own_tools_of_the_repository_are_built_from_their_paths() -> None:
    crates = {crate.name: crate for crate in [*install.ROOT_CRATES, *install.declared_crates()]}
    assert crates["idiomatic-sh-tastemaker"].path == (ROOT / "crates/tastemaker/cli").resolve()
    assert crates["idiomatic-sh-asymptote"].path == (ROOT / "crates/asymptote/cli").resolve()
    assert crates["idiomatic-sh-ryo"].path == (ROOT / "crates/ryo/cli").resolve()
    assert (crates["idiomatic-sh-ryo"].bins, crates["idiomatic-sh-ryo"].version) == (("ryo",), install.workspace_version("ryo"))
    assert crates["taplo-cli"].bins == ("taplo",)


def test_the_dylint_and_dist_pins_are_read_from_the_files_that_hold_them() -> None:
    assert install.dylint_version() == tomllib.loads((ROOT / "tools/lints/Cargo.toml").read_text())["dependencies"]["dylint_linting"].removeprefix("=")
    assert install.dist_version().count(".") == 2


def commands(tools: list[dict]) -> set[str]:
    return {tool[key][0] for tool in tools for key in ("run", "fix") if key in tool}


def test_every_command_a_check_runs_bare_is_installed_by_the_script() -> None:
    provided = set(install.TOOLS) | {
        bin for crate in [*install.ROOT_CRATES, *install.declared_crates()] for bin in (crate.bins or (crate.name,))
    }
    for manifest_path in [ROOT / "asymptote.toml", *sorted(ROOT.glob("crates/*/asymptote.toml"))]:
        tools = tomllib.loads(manifest_path.read_text())["tools"]
        unknown = commands(tools) - provided - SYSTEM
        assert not unknown, f"{manifest_path.relative_to(ROOT)} runs {sorted(unknown)}, which nothing installs"


def test_no_check_names_a_version_or_a_launcher() -> None:
    for manifest_path in [ROOT / "asymptote.toml", *sorted(ROOT.glob("crates/*/asymptote.toml"))]:
        text = " ".join(" ".join(tool.get(key, [])) for tool in tomllib.loads(manifest_path.read_text())["tools"] for key in ("run", "fix"))
        for pinned in ("==", "--from", "uvx", "cargo bin"):
            assert pinned not in text, f"{manifest_path.relative_to(ROOT)} names {pinned!r} in a run or fix line"
