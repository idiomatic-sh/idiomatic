#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Installs every command-line tool the repository's checks run, each at the version declared for it, so the checks name bare tools.

    uv run --script tools/install.py            install what is missing or at another version

Where a version is declared:

- a Rust tool more than one product runs, or that no product declares, once, in `ROOT_CRATES` below; our own tools are installed at
  the version the root's own file names (`tastemaker = "x.y.z"` in `tastemaker.toml`, `asymptote = "x.y.z"` in `asymptote.toml`), the
  line the tool checks itself against on every run; `ryo` at the version written beside it below. Each is a published release, pinned
  apart from its product's workspace version, so a change that bumps a product still installs the release before it.
- a Rust tool only one product runs, in that product's `[workspace.metadata.bin]` (its `crates/*/Cargo.toml`):
  `name = { version = "x.y.z", locked = true, bins = [...] }`. A tool is
  declared in one place: one `ROOT_CRATES` holds, or another product's manifest declares, is an error.
- `cargo-dylint` and `dylint-link`, in `tools/lints/Cargo.toml` (`dylint_linting`): a library loads only into the dylint of its own version.
- `cargo-dist`, in the `dist-workspace.toml` of the npm registry, which cargo-dist reads itself (crates.io holds no release of it as new as
  the one pinned, so its own installer installs it).
- the nightly toolchain, in `crates/ryo/Cargo.toml` (`[workspace.metadata.api] nightly`).
- everything that is not Rust, in `TOOLS` below: it has no manifest here; the package managers ryo's tests compare against, which npm
  installs, in `NODE_TOOLS`.
- the `gh` extensions, in `GH_EXTENSIONS` below: `gh extension install <repository> --pin <tag>`.
- `cargo-binstall`, which installs the prebuilt Rust tools, in `BINSTALL` below: its official release archive for this machine, checked
  against the SHA-256 recorded there, never compiled.

A Rust tool is fetched prebuilt with `cargo binstall`, which may not compile it: no tool is compiled. A tool with no prebuilt release for a
machine is named in its `unavailable` (the machine's target) with the reason, and is not installed there: the groups that run it do not run
on that machine. Any other missing prebuilt fails the install. A tool whose release ships archives cargo-binstall does not find (a gzipped
executable, or a tar.gz or zip holding it), Rust or not, is in `ARCHIVES`, each archive's SHA-256 recorded. Our own published tools come prebuilt from the GitHub Release of their tag, the installed
command checked against the archive the release's `SHA256SUMS` names. A tool whose installed version is the declared one is left as it
is.
"""

from __future__ import annotations

import gzip
import hashlib
import io
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.request
import zipfile
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# cargo-binstall, from its release archive for this machine, each archive's SHA-256 as downloaded from the release.
BINSTALL = {
    "version": "1.23.0",
    "sha256": {
        "x86_64-unknown-linux-musl.tgz": "64bf954c68bb558431deeabecaec7687edd5541c2189ee263bb8bc18bc4fdf55",
        "aarch64-unknown-linux-musl.tgz": "ba9b7bf426c7b7375825cd3fa367c3f8a632ca7c6c546fdcb738114b167f4103",
        "x86_64-apple-darwin.zip": "5b4d6cd99651318e17a26ed9bab7505de0f496d88793a7735a0fc6e2079efe83",
        "aarch64-apple-darwin.zip": "0f679c0bc992c6b84fdcbb0f65492588447d26596ef387cb6cb1ba41ad8ceb33",
        "x86_64-pc-windows-msvc.zip": "f4641479477aca40387e88297e3813fab8e44a8d21f25faa44e0ff33e2bc1726",
        "aarch64-pc-windows-msvc.zip": "a3996cbd0c7fa2be599bd52eaf70ccd8c0dce4cce8d01345d486f47b0e0ee0f9",
    },
}
BINSTALL_URL = "https://github.com/cargo-bins/cargo-binstall/releases/download/v{version}/cargo-binstall-{asset}"


def pinned(tool: str, root: Path = ROOT) -> str:
    """The version of our own `tool` the repository needs: the `<tool> = "x.y.z"` line of the root `<tool>.toml`, which the tool
    itself checks on every run."""
    return tomllib.loads((root / f"{tool}.toml").read_text(encoding="utf-8"))[tool]


@dataclass(frozen=True)
class Crate:
    """A Rust tool: the crate, its version, and the options it is built with."""

    name: str
    version: str
    bins: tuple[str, ...] = ()
    default_features: bool = True
    features: tuple[str, ...] = ()
    # The machines (targets) it has no prebuilt release for, each with the reason: it is not installed there.
    unavailable: dict[str, str] = field(default_factory=dict)


# The Rust tools declared once for the whole repository: every tool more than one product's checks run, and the ones the release
# job and the root's checks run.
ROOT_CRATES = [
    Crate("idiomatic-sh-tastemaker", pinned("tastemaker"), bins=("tm",)),
    Crate("idiomatic-sh-asymptote", pinned("asymptote"), bins=("asymptote",)),
    Crate("idiomatic-sh-ryo", "0.1.1", bins=("ryo",)),
    Crate("cargo-deny", "0.20.2"),
    Crate("cargo-nextest", "0.9.146"),
    Crate("cargo-machete", "0.9.2"),
    Crate("cargo-hack", "0.6.45"),
    Crate("cargo-mutants", "27.1.0"),
    # Its release ships Linux x64 and macOS arm64 alone, the machines the `clippy` and `release-clippy` groups run on.
    Crate(
        "clippy-sarif",
        "0.8.0",
        unavailable={
            "aarch64-unknown-linux-gnu": "no aarch64 Linux archive in its release",
            "x86_64-pc-windows-msvc": "no Windows archive in its release",
        },
    ),
    Crate("cargo-llvm-cov", "0.9.1"),
    # `release-msrv` runs on Linux x64.
    Crate("cargo-msrv", "0.19.6", unavailable={"aarch64-unknown-linux-gnu": "no aarch64 Linux archive in its release"}),
    Crate("hyperfine", "1.20.0"),
    # Reclaims stale build directories; it works across workspaces.
    Crate("cargo-sweep", "0.8.0"),
    # Opens a product's release pull request (ci/release_pr/).
    Crate("release-plz", "0.4.2"),
]

# The tools that are not Rust: the command each installs, its package and version, and what it needs beside it.
TOOLS = {
    "yamllint": {"package": "yamllint", "version": "1.37.1"},
    "ec": {"package": "editorconfig-checker", "version": "3.11.1"},
    "typos": {"package": "typos", "version": "1.49.0"},
    "actionlint": {"package": "actionlint-py", "version": "1.7.12.24"},
    "shellcheck": {"package": "shellcheck-py", "version": "0.11.0.1"},
    "cz": {"package": "commitizen", "version": "4.17.0"},
    # The checkpoint's run.py runs in this environment too: it reads Rust through tree-sitter-rust, on CI's Python.
    "pytest": {"package": "pytest", "version": "9.1.1", "python": "3.12", "with": ["pyyaml==6.0.3", "tree-sitter==0.26.1", "tree-sitter-rust==0.24.2"]},
    "ruff": {"package": "ruff", "version": "0.16.5"},
    # The release job builds wheels with maturin.
    "maturin": {"package": "maturin", "version": "1.15.0"},
}

# The package managers ryo's replay tests ask what a JavaScript workspace resolves to, installed by npm: command -> version.
NODE_TOOLS = {"pnpm": "11.25.0", "bun": "1.4.4"}
# The gh extensions, by repository, each at the release tag it is pinned to: gh-stack opens the stacked pull requests.
GH_EXTENSIONS = {"github/gh-stack": "v0.2.1"}


def declared_crates(root: Path = ROOT, shared: list[Crate] = ROOT_CRATES) -> list[Crate]:
    """Every tool a product declares in `[workspace.metadata.bin]`; one `shared` holds or another product declares is an error."""
    found: dict[str, Crate] = {}
    declared_once = {crate.name for crate in shared}
    for manifest in sorted(root.glob("crates/*/Cargo.toml")):
        bins = tomllib.loads(manifest.read_text(encoding="utf-8")).get("workspace", {}).get("metadata", {}).get("bin", {})
        for name, entry in bins.items():
            entry = {"version": entry} if isinstance(entry, str) else entry
            crate = Crate(
                name=name,
                version=entry["version"],
                bins=tuple(entry.get("bins", ())),
                default_features=entry.get("default-features", True),
                features=tuple(entry.get("features", ())),
                unavailable=dict(entry.get("unavailable", {})),
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


def installed_gh_extensions() -> dict[str, str]:
    """The extensions `gh extension list` names: repository -> pinned tag."""
    listing = run(["gh", "extension", "list"], capture=True).stdout
    return {fields[1]: fields[2] for line in listing.splitlines() if len(fields := line.split("\t")) >= 3}


def install_gh_extensions(installed: dict[str, str]) -> list[str]:
    """Installs each of `GH_EXTENSIONS` not installed at its pinned tag, replacing one at another tag; returns the ones it installed."""
    done = []
    for repository, tag in GH_EXTENSIONS.items():
        if installed.get(repository) == tag:
            continue
        replace = ["--force"] if repository in installed else []
        run(["gh", "extension", "install", repository, "--pin", tag, *replace])
        done.append(f"{repository}@{tag}")
    return done


def installed_crates() -> dict[str, tuple[str, str]]:
    """The crates `cargo install` and `cargo binstall` left: crate -> (version, source)."""
    listing = run(["cargo", "install", "--list"], capture=True).stdout
    found = {}
    for line in listing.splitlines():
        matched = re.fullmatch(r"(\S+) v(\S+?)(?: \((.*)\))?:", line)
        if matched:
            found[matched[1]] = (matched[2], matched[3] or "")
    return found


def installed_python() -> dict[str, tuple[str, frozenset[str], str]]:
    """The packages `uv tool install` left: package -> (version, the requirements installed with it, its Python's minor version)."""
    listing = run(["uv", "tool", "list", "--show-with", "--show-python"], capture=True).stdout
    found = {}
    for line in listing.splitlines():
        matched = re.fullmatch(r"(\S+) v(\S+)(?: \[with: ([^]]*)\])? \[\w+ (\d+\.\d+)\S*\]", line)
        if matched:
            found[matched[1]] = (matched[2], frozenset(filter(None, (matched[3] or "").split(", "))), matched[4])
    return found


def stale_tool(tool: dict, installed: dict[str, tuple[str, frozenset[str], str]]) -> str | None:
    """Why the installed Python tool is not the declared one (its version, what is installed with it, or its Python); None where it is."""
    have = installed.get(tool["package"])
    if have is None:
        return "not installed"
    version, extras, python = have
    if version != tool["version"]:
        return f"installed {version}, declared {tool['version']}"
    if extras != frozenset(tool.get("with", [])):
        return f"installed with {', '.join(sorted(extras)) or 'nothing'}, declared with {', '.join(tool.get('with', [])) or 'nothing'}"
    if "python" in tool and python != tool["python"]:
        return f"installed on Python {python}, declared {tool['python']}"
    return None


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
    return None


def install_crate(crate: Crate, host: str) -> str:
    """Installs `crate` on `host` and says how: prebuilt, or not at all where it names `host` unavailable."""
    options = ([] if crate.default_features else ["--no-default-features"]) + (["--features", ",".join(crate.features)] if crate.features else [])
    if host in crate.unavailable:
        return f"not installed: {crate.unavailable[host]}"
    if options:
        sys.exit(f"{crate.name}: a prebuilt release has no features to choose")
    ours = crate.name.startswith(OURS)
    # Ours fetches the archive of this very target, the one `verify_release` checks.
    targets = ["--targets", host] if ours else []
    argv = ["cargo", "binstall", "--no-confirm", "--locked", "--force", "--disable-strategies", "compile", *targets, f"{crate.name}@{crate.version}"]
    if run(argv, check=False).returncode != 0:
        sys.exit(f"{crate.name} {crate.version} has no prebuilt release for {host}: name {host} in its `unavailable` with the reason")
    if ours:
        verify_release(crate, host, Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "bin")
    return "prebuilt"


# Our own packages, and the GitHub Release each version is published to: `<package>-v<version>`, one archive per target named in
# its `SHA256SUMS`.
OURS = "idiomatic-sh-"
RELEASE_URL = "https://github.com/idiomatic-sh/idiomatic/releases/download/{name}-v{version}/{file}"


def verify_release(crate: Crate, host: str, bin_dir: Path) -> None:
    """Stops the install unless each command cargo-binstall wrote for `crate` is the one in the release archive for `host` whose
    SHA-256 the release's `SHA256SUMS` names."""
    archive = f"{crate.name}-{host}" + (".zip" if host.endswith("-windows-msvc") else ".tar.gz")
    with urllib.request.urlopen(RELEASE_URL.format(name=crate.name, version=crate.version, file="SHA256SUMS")) as response:  # noqa: S310 - a fixed https URL
        sums = {file: digest for digest, file in (line.split() for line in response.read().decode().splitlines() if line.strip())}
    if archive not in sums:
        sys.exit(f"{crate.name} {crate.version}: its SHA256SUMS names no {archive}")
    data = fetched(RELEASE_URL.format(name=crate.name, version=crate.version, file=archive), sums[archive])
    for command in crate.bins:
        name = command + (".exe" if os.name == "nt" else "")
        if archive.endswith(".zip"):
            expected = zipfile.ZipFile(io.BytesIO(data)).read(name)
        else:
            member = tarfile.open(fileobj=io.BytesIO(data), mode="r:gz").extractfile(name)
            if member is None:
                sys.exit(f"{archive} holds no {name}")
            expected = member.read()
        if (bin_dir / name).read_bytes() != expected:
            sys.exit(f"{bin_dir / name} is not the {name} of {archive}, whose SHA-256 SHA256SUMS names")


# The tools whose releases ship one gzipped executable per machine, which cargo-binstall does not find: the command, its version, the
# archive for each target, and each archive's SHA-256 as downloaded from the release.
ARCHIVES = {
    "taplo": {
        "version": "0.10.0",
        "url": "https://github.com/tamasfe/taplo/releases/download/{version}/taplo-{asset}.gz",
        "assets": {
            "x86_64-unknown-linux-gnu": ("linux-x86_64", "8fe196b894ccf9072f98d4e1013a180306e17d244830b03986ee5e8eabeb6156"),
            "aarch64-unknown-linux-gnu": ("linux-aarch64", "033681d01eec8376c3fd38fa3703c79316f5e14bb013d859943b60a07bccdcc3"),
            "aarch64-apple-darwin": ("darwin-aarch64", "713734314c3e71894b9e77513c5349835eefbd52908445a0d73b0c7dc469347d"),
            "x86_64-apple-darwin": ("darwin-x86_64", "898122cde3a0b1cd1cbc2d52d3624f23338218c91b5ddb71518236a4c2c10ef2"),
            "x86_64-pc-windows-msvc": ("windows-x86_64", "550fdc955343f8a196447a05346ecf8827e391726e4ded577bb00a0a36cf220c"),
            "aarch64-pc-windows-msvc": ("windows-aarch64", "c16a9a1248bdd746fde657b0196bfc47c8bede1e50ae73b7d9d33088a6682180"),
        },
    },
    # The secret scanner of the root's lint group. Its release ships each executable in a tar.gz (a zip on Windows), beside the
    # LICENSE and README; each SHA-256 is the one its `gitleaks_<version>_checksums.txt` names.
    "gitleaks": {
        "version": "8.30.1",
        "url": "https://github.com/gitleaks/gitleaks/releases/download/v{version}/gitleaks_{version}_{asset}",
        "assets": {
            "x86_64-unknown-linux-gnu": ("linux_x64.tar.gz", "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"),
            "aarch64-unknown-linux-gnu": ("linux_arm64.tar.gz", "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080"),
            "aarch64-apple-darwin": ("darwin_arm64.tar.gz", "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5"),
            "x86_64-apple-darwin": ("darwin_x64.tar.gz", "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709"),
            "x86_64-pc-windows-msvc": ("windows_x64.zip", "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e"),
            "aarch64-pc-windows-msvc": ("windows_arm64.zip", "b95f5e4f5c425cedca7ee203d9afd29597e692c4924a12ed42f970537c72cc0f"),
        },
    },
}


def fetched(url: str, sha256: str) -> bytes:
    """The file at `url`, once its SHA-256 is `sha256`; the install stops on any other."""
    print("+ fetch", url, flush=True)
    with urllib.request.urlopen(url) as response:  # noqa: S310 - a fixed https URL
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != sha256:
        sys.exit(f"{url}: SHA-256 {digest}, not the recorded {sha256}")
    return data


def unpacked(data: bytes, asset: str, name: str) -> bytes:
    """The executable `name` from a release archive: the member of that name of a `.tar.gz` or a `.zip`, or the whole of a plain
    `.gz`."""
    if asset.endswith(".tar.gz"):
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            member = archive.extractfile(name)
            if member is None:
                sys.exit(f"{asset}: {name} is not a file")
            return member.read()
    if asset.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            return archive.read(name)
    return gzip.decompress(data)


def install_archive(command: str, host: str, bin_dir: Path) -> str:
    """Puts `command`'s release executable for `host` in `bin_dir` unless the one there answers its version."""
    tool = ARCHIVES[command]
    name = command + (".exe" if os.name == "nt" else "")
    found = shutil.which(name, path=str(bin_dir))
    if found and tool["version"] in run([found, "--version"], check=False, capture=True).stdout.split():
        return "installed"
    if host not in tool["assets"]:
        sys.exit(f"{command} {tool['version']} has no archive recorded for {host}")
    asset, sha256 = tool["assets"][host]
    data = unpacked(fetched(tool["url"].format(version=tool["version"], asset=asset), sha256), asset, name)
    bin_dir.mkdir(parents=True, exist_ok=True)
    (bin_dir / name).write_bytes(data)
    (bin_dir / name).chmod(0o755)
    return "prebuilt archive"


def host_target() -> str:
    """This machine's target triple, as rustc names it."""
    said = run(["rustc", "-vV"], capture=True).stdout
    return next(line.removeprefix("host: ") for line in said.splitlines() if line.startswith("host: "))


def binstall_asset(host: str) -> str:
    """The cargo-binstall archive for `host`: Linux's static musl build on either libc, the platform's own elsewhere."""
    arch = host.split("-", 1)[0]
    if "-linux-" in host:
        return f"{arch}-unknown-linux-musl.tgz"
    if host.endswith("-apple-darwin"):
        return f"{arch}-apple-darwin.zip"
    if host.endswith("-pc-windows-msvc"):
        return f"{arch}-pc-windows-msvc.zip"
    sys.exit(f"no cargo-binstall archive is recorded for {host}")


def binstall_installed() -> str:
    """The version `cargo binstall -V` answers; empty where there is none."""
    found = run(["cargo", "binstall", "-V"], check=False, capture=True)
    return found.stdout.strip() if found.returncode == 0 else ""


def install_binstall(host: str, bin_dir: Path) -> None:
    """Puts cargo-binstall's release archive for `host` in `bin_dir`, once its SHA-256 is the one recorded in `BINSTALL`."""
    asset = binstall_asset(host)
    url = BINSTALL_URL.format(version=BINSTALL["version"], asset=asset)
    archive = fetched(url, BINSTALL["sha256"][asset])
    name = "cargo-binstall.exe" if os.name == "nt" else "cargo-binstall"
    if asset.endswith(".zip"):
        data = zipfile.ZipFile(io.BytesIO(archive)).read(name)
    else:
        member = tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz").extractfile(name)
        if member is None:
            sys.exit(f"{url} holds no {name}")
        data = member.read()
    bin_dir.mkdir(parents=True, exist_ok=True)
    (bin_dir / name).write_bytes(data)
    (bin_dir / name).chmod(0o755)


def main(argv: list[str]) -> int:
    if argv:
        print(__doc__)
        return 2
    crates = [*ROOT_CRATES, *declared_crates()]
    dylint = dylint_version()
    # `release-dylint` runs on Linux x64.
    windows = {"x86_64-pc-windows-msvc": "no Windows archive in its release"}
    crates += [Crate("cargo-dylint", dylint, unavailable=windows), Crate("dylint-link", dylint, unavailable=windows)]
    host = host_target()
    bin_dir = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "bin"
    if binstall_installed() != BINSTALL["version"]:
        install_binstall(host, bin_dir)
    installed = installed_crates()
    how = {crate.name: install_crate(crate, host) for crate in crates if stale_crate(crate, installed)}
    how |= {command: install_archive(command, host, bin_dir) for command in ARCHIVES}
    for name, done in how.items():
        print(f"{name}: {done}", flush=True)
    have = installed_python()
    for command, tool in TOOLS.items():
        if stale_tool(tool, have):
            extra = [part for each in tool.get("with", []) for part in ("--with", each)]
            python = ["--python", tool["python"]] if "python" in tool else []
            run(["uv", "tool", "install", "--force", *python, *extra, f"{tool['package']}=={tool['version']}"])
    stale = [f"{command}@{version}" for command, version in NODE_TOOLS.items() if installed_node(command) != version]
    if stale:
        npm = shutil.which("npm")
        if npm is None:
            sys.exit(f"npm installs {', '.join(stale)}: install Node.js first")
        run([npm, "install", "--global", *stale])
    if shutil.which("gh") is None:
        sys.exit(f"gh installs {', '.join(GH_EXTENSIONS)}: install the GitHub CLI first")
    install_gh_extensions(installed_gh_extensions())
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
