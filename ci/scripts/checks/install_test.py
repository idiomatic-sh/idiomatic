"""What `tools/install.py` promises: each tool is declared once, and every command a check runs bare is one it installs."""

from __future__ import annotations

import importlib.util
import json
import os
import shlex
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
# setup-python supplies the bare Python command used by the benchmark verifier.
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


def test_the_own_tools_are_their_published_releases_pinned_apart_from_their_workspaces() -> None:
    crates = {crate.name: crate for crate in [*install.ROOT_CRATES, *install.declared_crates()]}
    assert (crates["idiomatic-sh-tastemaker"].version, crates["idiomatic-sh-tastemaker"].bins) == (install.pinned("tastemaker"), ("tm",))
    assert (crates["idiomatic-sh-ryo"].version, crates["idiomatic-sh-ryo"].bins) == ("0.1.1", ("ryo",))
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
    for manifest_path in [ROOT / "asymptote.toml", *sorted(ROOT.glob("crates/*/asymptote.toml"))]:
        text = " ".join(" ".join(tool.get(key, [])) for tool in tomllib.loads(manifest_path.read_text())["tools"] for key in ("run", "fix"))
        for pinned in ("==", "--from", "uvx", "cargo bin"):
            assert pinned not in text, f"{manifest_path.relative_to(ROOT)} names {pinned!r} in a run or fix line"


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


def test_an_archive_s_executable_is_its_member_of_that_name_or_the_whole_of_a_gzip() -> None:
    import gzip
    import io
    import tarfile
    import zipfile

    packed = io.BytesIO()
    with tarfile.open(fileobj=packed, mode="w:gz") as archive:
        for member, body in (("LICENSE", b"license"), ("gitleaks", b"scanner")):
            info = tarfile.TarInfo(member)
            info.size = len(body)
            archive.addfile(info, io.BytesIO(body))
    assert install.unpacked(packed.getvalue(), "linux_x64.tar.gz", "gitleaks") == b"scanner"
    zipped = io.BytesIO()
    with zipfile.ZipFile(zipped, "w") as archive:
        archive.writestr("README.md", b"readme")
        archive.writestr("gitleaks.exe", b"scanner")
    assert install.unpacked(zipped.getvalue(), "windows_x64.zip", "gitleaks.exe") == b"scanner"
    assert install.unpacked(gzip.compress(b"taplo"), "linux-x86_64", "taplo") == b"taplo"
    with pytest.raises(KeyError):
        install.unpacked(packed.getvalue(), "linux_x64.tar.gz", "other")


def test_an_own_tool_is_kept_only_when_it_is_the_command_of_the_release_archive_its_sha256sums_names(tmp_path, monkeypatch) -> None:
    import hashlib
    import io
    import tarfile

    packed = io.BytesIO()
    with tarfile.open(fileobj=packed, mode="w:gz") as archive:
        info = tarfile.TarInfo("ryo")
        info.size = 7
        archive.addfile(info, io.BytesIO(b"release"))
    data = packed.getvalue()
    name = "idiomatic-sh-ryo-x86_64-unknown-linux-gnu.tar.gz"
    files = {"SHA256SUMS": f"{hashlib.sha256(data).hexdigest()}  {name}\n".encode(), name: data}
    monkeypatch.setattr(install.urllib.request, "urlopen", lambda url: io.BytesIO(files[url.rsplit("/", 1)[1]]))
    monkeypatch.setattr(install.os, "name", "posix")
    crate = install.Crate("idiomatic-sh-ryo", "0.1.1", bins=("ryo",))
    (tmp_path / "ryo").write_bytes(b"release")
    install.verify_release(crate, "x86_64-unknown-linux-gnu", tmp_path)
    (tmp_path / "ryo").write_bytes(b"another")
    with pytest.raises(SystemExit):
        install.verify_release(crate, "x86_64-unknown-linux-gnu", tmp_path)
    files[name] = data + b"tampered"
    (tmp_path / "ryo").write_bytes(b"release")
    with pytest.raises(SystemExit):
        install.verify_release(crate, "x86_64-unknown-linux-gnu", tmp_path)


def test_benchmark_collector_selects_only_one_matching_executable(tmp_path) -> None:
    """The benchmark tool runs the collector its build names: `seal_cargo_artifact.py` takes the one release executable Cargo's
    compiler-artifact records name, and refuses a build that names none or two."""
    tools = tomllib.loads((ROOT / "crates/ryo/asymptote.toml").read_text())["tools"]
    command = next(tool["run"][-1] for tool in tools if "ryo-benchmark-collector --message-format=json" in tool["run"][-1])
    (sealing,) = [line for line in command.splitlines() if "seal_cargo_artifact.py" in line and "ryo-benchmark-collector" in line]
    script, _build, *selection, sealed = shlex.split(sealing)[1:]
    assert script == "../../.github/scripts/seal_cargo_artifact.py"
    assert selection == ["ryo-benchmark-collector", "bin", "/crates/benchmark/src/main.rs", "false"]
    assert f'"$PWD/{sealed}"' in command
    collector = tmp_path / "collector"
    collector.write_text("#!/bin/sh\nexit 0\n")
    collector.chmod(0o755)
    matching = {
        "reason": "compiler-artifact",
        "target": {"name": "ryo-benchmark-collector", "kind": ["bin"], "src_path": "/checkout/crates/ryo/crates/benchmark/src/main.rs"},
        "profile": {"test": False},
        "executable": str(collector),
    }
    cases = (
        ([matching], True),
        ([{**matching, "reason": "build-finished"}], False),
        ([{**matching, "target": {**matching["target"], "name": "other"}}], False),
        ([matching, matching], False),
    )
    for number, (records, accepted) in enumerate(cases):
        build = tmp_path / f"build-{number}.jsonl"
        build.write_text("\n".join(json.dumps(record) for record in records) + "\n")
        out = tmp_path / f"sealed-{number}"
        run = subprocess.run([sys.executable, "-I", str(ROOT / ".github/scripts/seal_cargo_artifact.py"), str(build), *selection, str(out)], capture_output=True, text=True, check=False)
        # Two artifacts must not become one executable, and no artifact must not pass.
        assert (run.returncode == 0 and out.is_file() and os.access(out, os.X_OK)) == accepted, run.stderr
