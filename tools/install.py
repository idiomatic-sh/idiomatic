#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Installs every command-line tool the repository's checks run, each at the version declared for it, so the checks name bare tools.

    uv run --script tools/install.py            install what is missing or at another version

Where a version is declared:

- a Rust tool more than one product runs, or that no product declares, once, in `ROOT_CRATES` below; our own tools carry a `path`
  and are built from this checkout, at the version the root's own file names (`tastemaker = "x.y.z"` in `tastemaker.toml`,
  `asymptote = "x.y.z"` in `asymptote.toml`), the line the tool checks itself against on every run; `ryo` is built at the version of its
  Cargo workspace (`crates/ryo/Cargo.toml`).
- a Rust tool only one product runs, in that product's `[workspace.metadata.bin]` (its `crates/*/Cargo.toml`):
  `name = { version = "x.y.z", locked = true, bins = [...] }`, and `path = "..."` (relative to that manifest) for our own tools. A tool is
  declared in one place: one `ROOT_CRATES` holds, or another product's manifest declares, is an error.
- `cargo-dylint` and `dylint-link`, in `tools/lints/Cargo.toml` (`dylint_linting`): a library loads only into the dylint of its own version.
- `cargo-dist`, in the `dist-workspace.toml` of the npm registry, which cargo-dist reads itself (crates.io holds no release of it as new as
  the one pinned, so its own installer installs it).
- the nightly toolchain, in `crates/ryo/Cargo.toml` (`[workspace.metadata.api] nightly`).
- everything that is not Rust, in `TOOLS` below: it has no manifest here; the package managers ryo's tests compare against, which npm
  installs, in `NODE_TOOLS`.
- `cargo-binstall`, which installs the prebuilt Rust tools, in `BOOTSTRAP` below: nothing can install it.

A Rust tool is fetched prebuilt with `cargo binstall` and built with `cargo install` where there is no prebuilt one for this machine. A tool
installed from a path is installed again when its version differs or a source file is newer than the installed command.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Installed by `cargo install`: nothing can install it.
BOOTSTRAP = {"cargo-binstall": "1.23.0"}


def pinned(tool: str, root: Path = ROOT) -> str:
    """The version of our own `tool` the repository needs: the `<tool> = "x.y.z"` line of the root `<tool>.toml`, which the tool
    itself checks on every run."""
    return tomllib.loads((root / f"{tool}.toml").read_text(encoding="utf-8"))[tool]


def workspace_version(product: str, root: Path = ROOT) -> str:
    """The version of our own `product`'s command line: the `version` its Cargo workspace gives every crate."""
    return tomllib.loads((root / f"crates/{product}/Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]


@dataclass(frozen=True)
class Crate:
    """A Rust tool: the crate, its version, where it is built from when it is ours, and the options it is built with."""

    name: str
    version: str
    path: Path | None = None
    bins: tuple[str, ...] = ()
    default_features: bool = True
    features: tuple[str, ...] = ()


# The Rust tools declared once for the whole repository: every tool more than one product's checks run, and the ones the release
# job and the root's checks run.
ROOT_CRATES = [
    # path until the first release, then the version alone.
    Crate("idiomatic-sh-tastemaker", pinned("tastemaker"), path=ROOT / "crates/tastemaker/cli", bins=("tm",)),
    Crate("idiomatic-sh-asymptote", pinned("asymptote"), path=ROOT / "crates/asymptote/cli", bins=("asymptote",)),
    Crate("idiomatic-sh-ryo", workspace_version("ryo"), path=ROOT / "crates/ryo/cli", bins=("ryo",)),
    Crate("cargo-deny", "0.20.2"),
    Crate("cargo-nextest", "0.9.146"),
    Crate("cargo-machete", "0.9.2"),
    Crate("cargo-hack", "0.6.45"),
    Crate("cargo-mutants", "27.1.0"),
    Crate("clippy-sarif", "0.8.0"),
    Crate("cargo-llvm-cov", "0.9.1"),
    Crate("cargo-msrv", "0.19.6"),
    Crate("taplo-cli", "0.10.0", bins=("taplo",)),
    Crate("hyperfine", "1.20.0"),
    # Reclaims stale build directories; it works across workspaces.
    Crate("cargo-sweep", "0.8.0"),
]

# The tools that are not Rust: the command each installs, its package and version, and what it needs beside it.
TOOLS = {
    "yamllint": {"package": "yamllint", "version": "1.37.1"},
    "ec": {"package": "editorconfig-checker", "version": "3.11.1"},
    "typos": {"package": "typos", "version": "1.49.0"},
    "actionlint": {"package": "actionlint-py", "version": "1.7.12.24"},
    "shellcheck": {"package": "shellcheck-py", "version": "0.11.0.1"},
    "cz": {"package": "commitizen", "version": "4.17.0"},
    "pytest": {"package": "pytest", "version": "9.1.1", "with": ["pyyaml==6.0.3"]},
    "ruff": {"package": "ruff", "version": "0.16.5"},
    # The release job builds wheels with maturin.
    "maturin": {"package": "maturin", "version": "1.15.0"},
}

# The package managers ryo's replay tests ask what a JavaScript workspace resolves to, installed by npm: command -> version.
NODE_TOOLS = {"pnpm": "11.25.0", "bun": "1.4.4"}

# Directories a path-installed tool's sources are not read from.
IGNORED = {"target", ".git", "node_modules"}


def declared_crates(root: Path = ROOT, shared: list[Crate] = ROOT_CRATES) -> list[Crate]:
    """Every tool a product declares in `[workspace.metadata.bin]`; one `shared` holds or another product declares is an error."""
    found: dict[str, Crate] = {}
    declared_once = {crate.name for crate in shared}
    for manifest in sorted(root.glob("crates/*/Cargo.toml")):
        bins = tomllib.loads(manifest.read_text(encoding="utf-8")).get("workspace", {}).get("metadata", {}).get("bin", {})
        for name, entry in bins.items():
            entry = {"version": entry} if isinstance(entry, str) else entry
            path = (manifest.parent / entry["path"]).resolve() if "path" in entry else None
            crate = Crate(
                name=name,
                version=entry["version"],
                path=path,
                bins=tuple(entry.get("bins", ())),
                default_features=entry.get("default-features", True),
                features=tuple(entry.get("features", ())),
            )
            if name in declared_once:
                sys.exit(f"{manifest}: {name} is declared once for the repository, in tools/install.py's ROOT_CRATES")
            if name in found:
                sys.exit(f"{manifest}: {name} is declared by another product too: declare it once, in tools/install.py's ROOT_CRATES")
            found[name] = crate
    return list(found.values())


def dylint_version(root: Path = ROOT) -> str:
    """The version of `dylint_linting` the lints are built against."""
    manifest = tomllib.loads((root / "tools/lints/Cargo.toml").read_text(encoding="utf-8"))
    return manifest["dependencies"]["dylint_linting"].removeprefix("=")


DIST_WORKSPACE = "crates/asymptote/distribution/registries/npm/dist-workspace.toml"
DIST_INSTALLER = "https://github.com/axodotdev/cargo-dist/releases/download/v{version}/cargo-dist-installer.{extension}"


def dist_version(root: Path = ROOT) -> str:
    """The version of cargo-dist the npm registry's `dist-workspace.toml` pins."""
    return tomllib.loads((root / DIST_WORKSPACE).read_text(encoding="utf-8"))["dist"]["cargo-dist-version"]


def dist_installed() -> str:
    """The version `cargo dist --version` answers; empty where there is none."""
    found = run(["cargo", "dist", "--version"], check=False, capture=True)
    return found.stdout.split()[-1] if found.returncode == 0 and found.stdout.split() else ""


def install_dist(version: str) -> None:
    """Runs cargo-dist's own installer for `version`, which writes `dist` and its alias `cargo-dist`, the command `cargo dist` runs."""
    if os.name == "nt":
        url = DIST_INSTALLER.format(version=version, extension="ps1")
        run(["powershell", "-ExecutionPolicy", "ByPass", "-c", f"irm {url} | iex"])
        return
    url = DIST_INSTALLER.format(version=version, extension="sh")
    run(["sh", "-c", f"curl --proto '=https' --tlsv1.2 -LsSf {url} | sh"])


def nightly(root: Path = ROOT) -> str:
    """The nightly toolchain that writes rustdoc's JSON for the `sdk-api` tool."""
    return tomllib.loads((root / "crates/ryo/Cargo.toml").read_text(encoding="utf-8"))["workspace"]["metadata"]["api"]["nightly"]


def run(argv: list[str], check: bool = True, capture: bool = False) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(argv), flush=True)
    return subprocess.run(argv, check=check, text=True, encoding="utf-8", capture_output=capture, stdin=subprocess.DEVNULL)


def installed_node(command: str) -> str:
    """The version `command --version` answers; empty where there is none."""
    found = shutil.which(command)
    if found is None:
        return ""
    answered = run([found, "--version"], check=False, capture=True)
    return answered.stdout.strip() if answered.returncode == 0 else ""


def installed_crates() -> dict[str, tuple[str, str]]:
    """The crates `cargo install` and `cargo binstall` left: crate -> (version, source)."""
    listing = run(["cargo", "install", "--list"], capture=True).stdout
    found = {}
    for line in listing.splitlines():
        matched = re.fullmatch(r"(\S+) v(\S+?)(?: \((.*)\))?:", line)
        if matched:
            found[matched[1]] = (matched[2], matched[3] or "")
    return found


def installed_python() -> dict[str, str]:
    """The packages `uv tool install` left: package -> version."""
    listing = run(["uv", "tool", "list"], capture=True).stdout
    return {m[1]: m[2] for line in listing.splitlines() if (m := re.fullmatch(r"(\S+) v(\S+)", line))}


def source_roots(crate: Crate) -> list[Path]:
    """The directories of the packages a path-installed crate is built from: itself and the path dependencies it reaches, wherever they are."""
    manifest = crate.path / "Cargo.toml"
    listing = run(["cargo", "metadata", "--format-version", "1", "--locked", "--manifest-path", str(manifest)], capture=True).stdout
    packages = json.loads(listing)["packages"]
    return sorted({Path(package["manifest_path"]).parent for package in packages if package["source"] is None})


def newest(paths: list[Path]) -> float:
    """The latest modification time of a source file under any of `paths`."""
    latest = 0.0
    for path in paths:
        for directory, names, files in os.walk(path):
            names[:] = [name for name in names if name not in IGNORED]
            latest = max([latest, *(os.stat(Path(directory) / file).st_mtime for file in files)])
    return latest


def command_time(crate: Crate) -> float:
    """When the command a path-installed crate left was written; 0 where there is none."""
    home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    times = [(home / "bin" / bin).stat().st_mtime for bin in crate.bins or (crate.name,) if (home / "bin" / bin).exists()]
    return min(times, default=0.0)


def meets(installed: str, declared: str) -> bool:
    """Whether the `installed` version is the one `declared` asks for: a full `x.y.z` is that version exactly; `^0.1` or `0.1` is Cargo's
    caret requirement, met by any 0.1.x, compared as numbers."""
    if re.fullmatch(r"\d+\.\d+\.\d+", declared):
        return installed == declared
    wanted = re.fullmatch(r"\^?(\d+(?:\.\d+){0,2})", declared)
    have = re.match(r"(\d+)\.(\d+)\.(\d+)", installed)
    if wanted is None or have is None:
        return installed == declared
    parts = [int(part) for part in wanted.group(1).split(".")]
    floor = parts + [0] * (3 - len(parts))
    pivot = next((place for place, part in enumerate(parts) if part), len(parts) - 1)
    ceiling = [*floor[:pivot], floor[pivot] + 1, *[0] * (2 - pivot)]
    return floor <= [int(part) for part in have.groups()] < ceiling


def stale_crate(crate: Crate, installed: dict[str, tuple[str, str]]) -> str | None:
    """Why the installed crate is not the declared one; None where it is."""
    have = installed.get(crate.name)
    if have is None:
        return "not installed"
    if not meets(have[0], crate.version):
        return f"installed {have[0]}, declared {crate.version}"
    if crate.path is not None and (str(crate.path) not in have[1] or newest(source_roots(crate)) > command_time(crate)):
        return f"built from {have[1] or 'a registry'}, declared from {crate.path} (sources newer)"
    return None


def install_crate(crate: Crate) -> None:
    options = ([] if crate.default_features else ["--no-default-features"]) + (["--features", ",".join(crate.features)] if crate.features else [])
    if crate.path is not None:
        run(["cargo", "install", "--locked", "--force", *options, "--path", str(crate.path)])
        return
    if not options and run(["cargo", "binstall", "--no-confirm", "--locked", "--force", f"{crate.name}@{crate.version}"], check=False).returncode == 0:
        return
    run(["cargo", "install", "--locked", "--force", *options, "--version", crate.version, crate.name])


def main(argv: list[str]) -> int:
    if argv:
        print(__doc__)
        return 2
    crates = [*ROOT_CRATES, *declared_crates()]
    dylint = dylint_version()
    crates += [Crate("cargo-dylint", dylint), Crate("dylint-link", dylint)]
    installed = installed_crates()
    for name, version in BOOTSTRAP.items():
        if installed.get(name, ("",))[0] != version:
            run(["cargo", "install", "--locked", "--force", "--version", version, name])
    installed = installed_crates()
    for crate in crates:
        if stale_crate(crate, installed):
            install_crate(crate)
    have = installed_python()
    for command, tool in TOOLS.items():
        why = None if have.get(tool["package"]) == tool["version"] else f"installed {have.get(tool['package'], 'nothing')}, declared {tool['version']}"
        if why:
            extra = [part for each in tool.get("with", []) for part in ("--with", each)]
            run(["uv", "tool", "install", "--force", *extra, f"{tool['package']}=={tool['version']}"])
    stale = [f"{command}@{version}" for command, version in NODE_TOOLS.items() if installed_node(command) != version]
    if stale:
        npm = shutil.which("npm")
        if npm is None:
            sys.exit(f"npm installs {', '.join(stale)}: install Node.js first")
        run([npm, "install", "--global", *stale])
    version = dist_version()
    if dist_installed() != version:
        install_dist(version)
    toolchains = run(["rustup", "toolchain", "list"], capture=True).stdout
    wanted = nightly()
    if not toolchains.count(wanted):
        run(["rustup", "toolchain", "install", wanted, "--profile", "minimal"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
