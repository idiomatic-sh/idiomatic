"""What `tools/install.py` promises: each tool is declared once, and every command a check runs bare is one it installs."""

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
from pathlib import Path

import pytest
import tomllib

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("install", ROOT / "tools/install.py")
install = importlib.util.module_from_spec(SPEC)
# A dataclass looks its module up by name while it is defined.
sys.modules["install"] = install
SPEC.loader.exec_module(install)

# Commands the machine provides before any tool is installed; setup-python supplies `python` for the Ruff verification commands.
SYSTEM = {"cargo", "bash", "git", "uv", "sh", "jq", "python"}


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
    assert install.ARCHIVES["taplo"]["version"] == "0.10.0"


def test_the_dylint_and_dist_pins_are_read_from_the_files_that_hold_them() -> None:
    assert install.dylint_version() == tomllib.loads((ROOT / "tools/lints/Cargo.toml").read_text())["dependencies"]["dylint_linting"].removeprefix("=")
    assert install.dist_version().count(".") == 2


def commands(tools: list[dict]) -> set[str]:
    return {tool[key][0] for tool in tools for key in ("run", "fix") if key in tool}


def test_every_command_a_check_runs_bare_is_installed_by_the_script() -> None:
    provided = set(install.TOOLS) | set(install.ARCHIVES) | {
        bin for crate in [*install.ROOT_CRATES, *install.declared_crates()] for bin in (crate.bins or (crate.name,))
    }
    for manifest_path in [ROOT / "asymptote.toml", *sorted(ROOT.glob("crates/*/asymptote.toml"))]:
        tools = tomllib.loads(manifest_path.read_text())["tools"]
        unknown = commands(tools) - provided - SYSTEM
        assert not unknown, f"{manifest_path.relative_to(ROOT)} runs {sorted(unknown)}, which nothing installs"


def test_no_check_names_a_version_or_a_launcher() -> None:
    # Ruff's correctness and measurement commands use jq's `==` operator; only a version-like pin is forbidden.
    forbidden = {
        "pinned package version": re.compile(r"==\s*\d+\.\d+"),
        "uvx launcher": re.compile(r"\buvx\b"),
        "cargo bin launcher": re.compile(r"\bcargo\s+bin\b"),
        "cargo install --from": re.compile(r"--from\b"),
    }
    for manifest_path in [ROOT / "asymptote.toml", *sorted(ROOT.glob("crates/*/asymptote.toml"))]:
        text = " ".join(" ".join(tool.get(key, [])) for tool in tomllib.loads(manifest_path.read_text())["tools"] for key in ("run", "fix"))
        for description, pattern in forbidden.items():
            assert not pattern.search(text), f"{manifest_path.relative_to(ROOT)} names {description} in a run or fix line"
    jq_comparison = 'jq \'select(.reason == "compiler-artifact")\''
    assert not forbidden["pinned package version"].search(jq_comparison)


def test_a_tool_s_unavailable_machines_are_read_from_its_manifest(tmp_path) -> None:
    manifest(tmp_path / "crates/a/Cargo.toml", 'tool = { version = "0.1.0", unavailable = { x86_64-unknown-linux-gnu = "no Linux archive" } }')
    (crate,) = install.declared_crates(tmp_path, shared=[])
    assert crate.unavailable == {"x86_64-unknown-linux-gnu": "no Linux archive"}


def recorded(monkeypatch, returncode: int) -> list[list[str]]:
    calls: list[list[str]] = []

    def run(argv, check=True, capture=False):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, returncode, "", "")

    monkeypatch.setattr(install, "run", run)
    return calls


def test_a_tool_is_installed_prebuilt_and_never_compiled(monkeypatch) -> None:
    calls = recorded(monkeypatch, 0)
    assert install.install_crate(install.Crate("cargo-deny", "0.20.2"), "x86_64-unknown-linux-gnu") == "prebuilt"
    (argv,) = calls
    assert argv[:2] == ["cargo", "binstall"] and argv[-1] == "cargo-deny@0.20.2"
    assert argv[argv.index("--disable-strategies") + 1] == "compile"


def test_a_missing_prebuilt_fails_the_install(monkeypatch) -> None:
    recorded(monkeypatch, 1)
    with pytest.raises(SystemExit):
        install.install_crate(install.Crate("cargo-deny", "0.20.2"), "x86_64-unknown-linux-gnu")


def test_a_tool_is_not_installed_on_a_machine_it_names_unavailable(monkeypatch) -> None:
    calls = recorded(monkeypatch, 0)
    crate = install.Crate("cargo-msrv", "0.19.6", unavailable={"aarch64-unknown-linux-gnu": "no aarch64 Linux archive"})
    assert install.install_crate(crate, "aarch64-unknown-linux-gnu") == "not installed: no aarch64 Linux archive"
    assert calls == []
    assert install.install_crate(crate, "x86_64-unknown-linux-gnu") == "prebuilt"


def test_no_tool_is_compiled_from_a_registry() -> None:
    source = (ROOT / "tools/install.py").read_text(encoding="utf-8")
    assert '"cargo", "install", "--locked", "--force", *options, "--version"' not in source


def test_every_unavailable_machine_gives_its_reason() -> None:
    for crate in [*install.ROOT_CRATES, *install.declared_crates()]:
        assert all(reason.strip() for reason in crate.unavailable.values()), crate.name


def test_every_machine_the_checks_run_on_has_a_recorded_archive() -> None:
    for host in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "aarch64-apple-darwin", "x86_64-pc-windows-msvc"]:
        assert len(install.BINSTALL["sha256"][install.binstall_asset(host)]) == 64
        for tool in install.ARCHIVES.values():
            assert len(tool["assets"][host][1]) == 64


def test_windows_binary_names_use_exe_once(tmp_path, monkeypatch) -> None:
    monkeypatch.setenv("CARGO_HOME", str(tmp_path))
    crate = install.Crate("own-tool", "1.0.0", bins=("tm", "helper.exe"))
    assert install.command_paths(crate, windows=True) == (tmp_path / "bin/tm.exe", tmp_path / "bin/helper.exe")
    assert install.command_paths(crate, windows=False) == (tmp_path / "bin/tm", tmp_path / "bin/helper.exe")


def test_all_declared_binaries_are_required_even_when_one_is_installed(tmp_path, monkeypatch) -> None:
    first, second = tmp_path / "tm.exe", tmp_path / "helper.exe"
    first.write_bytes(b"first executable")
    monkeypatch.setattr(install, "command_paths", lambda crate: (first, second))
    crate = install.Crate("own-tool", "1.0.0", bins=("tm", "helper"))
    assert install.command_time(crate) == 0.0
    second.write_bytes(b"second executable")
    assert install.command_time(crate) == min(first.stat().st_mtime, second.stat().st_mtime)


def test_path_source_fingerprint_uses_content_not_checkout_timestamps(tmp_path, monkeypatch) -> None:
    workspace = tmp_path / "workspace"
    package = workspace / "tool"
    package.mkdir(parents=True)
    source = package / "src/main.rs"
    source.parent.mkdir()
    source.write_text("fn main() {}\n")
    crate = install.Crate("own-tool", "1.0.0", path=package)
    monkeypatch.setattr(install, "source_roots", lambda _crate: [package])
    monkeypatch.setattr(install, "cargo_identity", lambda: "cargo 1.90.0 test identity\n")
    monkeypatch.setattr(install, "rustc_identity", lambda: "rustc 1.90.0 test identity\n")

    original = install.path_source_fingerprint(crate)
    source.touch()
    assert install.path_source_fingerprint(crate) == original
    (package / "target").mkdir()
    (package / "target/generated.rs").write_text("generated")
    assert install.path_source_fingerprint(crate) == original
    for path, content in (
        (workspace / "Cargo.toml", "[workspace]\nresolver = \"3\"\n"),
        (workspace / "Cargo.lock", "# dependency graph\n"),
        (workspace / "rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
        (workspace / ".cargo/config.toml", "[build]\ntarget-dir = \"target\"\n"),
    ):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        assert install.path_source_fingerprint(crate) != original
        path.unlink()
    changed_features = install.Crate("own-tool", "1.0.0", path=package, features=("extra",))
    assert install.path_source_fingerprint(changed_features) != original
    monkeypatch.setattr(install, "rustc_identity", lambda: "rustc 1.91.0 test identity\n")
    assert install.path_source_fingerprint(crate) != original
    monkeypatch.setattr(install, "rustc_identity", lambda: "rustc 1.90.0 test identity\n")
    monkeypatch.setattr(install, "cargo_identity", lambda: "cargo 1.91.0 test identity\n")
    assert install.path_source_fingerprint(crate) != original
    monkeypatch.setattr(install, "cargo_identity", lambda: "cargo 1.90.0 test identity\n")
    source.write_text("fn main() { println!(\"changed\"); }\n")
    assert install.path_source_fingerprint(crate) != original


def test_path_install_stamp_rebuilds_on_source_change_or_missing_binary(tmp_path, monkeypatch) -> None:
    package = tmp_path / "workspace" / "tool"
    package.mkdir(parents=True)
    source = package / "src/main.rs"
    source.parent.mkdir()
    source.write_text("fn main() {}\n")
    binary = tmp_path / "cargo/bin/own-tool"
    binary.parent.mkdir(parents=True)
    binary.write_text("installed")
    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo"))
    monkeypatch.setattr(install, "command_paths", lambda _crate: (binary,))
    monkeypatch.setattr(install, "source_roots", lambda _crate: [package])
    monkeypatch.setattr(install, "cargo_identity", lambda: "cargo test identity\n")
    monkeypatch.setattr(install, "rustc_identity", lambda: "rustc test identity\n")
    crate = install.Crate("own-tool", "1.0.0", path=package)
    installed = {"own-tool": ("1.0.0", str(package))}

    install.record_path_install(crate, install.path_source_fingerprint(crate))
    assert install.stale_crate(crate, installed) is None
    source.write_text("fn main() { println!(\"changed\"); }\n")
    assert "source fingerprint differs" in install.stale_crate(crate, installed)
    install.record_path_install(crate, install.path_source_fingerprint(crate))
    binary.unlink()
    assert "command is missing" in install.stale_crate(crate, installed)


def test_failed_path_install_does_not_write_success_stamp(tmp_path, monkeypatch) -> None:
    package = tmp_path / "workspace" / "tool"
    package.mkdir(parents=True)
    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo"))
    monkeypatch.setattr(install, "source_roots", lambda _crate: [package])
    monkeypatch.setattr(install, "cargo_identity", lambda: "cargo test identity\n")
    monkeypatch.setattr(install, "rustc_identity", lambda: "rustc test identity\n")

    def fail(argv, **_kwargs):
        raise subprocess.CalledProcessError(1, argv)

    monkeypatch.setattr(install, "run", fail)
    crate = install.Crate("own-tool", "1.0.0", path=package)
    with pytest.raises(subprocess.CalledProcessError):
        install.install_crate(crate, "x86_64-unknown-linux-gnu")
    assert not install.path_stamp(crate).exists()
