"""Build the product benchmark from the checked-out engine and Tastemaker's release tag."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def source_identity(name):
    if name == "candidate":
        paths = ["HEAD:crates/ryo", "HEAD:crates/tastemaker/benchmarks/project"]
        inputs = ["candidate.rs.in"]
    else:
        pin = tomllib.loads((ROOT / "tastemaker.toml").read_text())["tastemaker"]
        paths = [f"idiomatic-sh-tastemaker-v{pin}^{{commit}}", "HEAD:crates/ryo/cli/benches/readers/shipping.rs"]
        inputs = ["capture.rs.in", "reference.rs.in"]
    objects = subprocess.check_output(["git", "rev-parse", *paths], cwd=ROOT, text=True).splitlines()
    files = [HERE / "build.py", *(HERE / "worker" / f for f in inputs)]
    return objects + [hashlib.sha256(p.read_bytes()).hexdigest() for p in files]


def verify_build(dest, name):
    stamp = json.loads((dest / (name + ".json")).read_bytes())
    if stamp["sources"] != source_identity(name):
        raise ValueError(f"{name} worker is stale; rebuild with build.py")
    for binary, expected in stamp["binaries"].items():
        with (dest / binary).open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != expected:
                raise ValueError(f"changed benchmark executable: {binary}")


def archive(revision, dest, paths):
    with tempfile.TemporaryFile() as tar:
        subprocess.run(["git", "archive", revision, *paths], cwd=ROOT, stdout=tar, check=True)
        tar.seek(0)
        subprocess.run(["tar", "xf", "-"], cwd=dest, stdin=tar, check=True)


def build(manifest, target, dest, arguments, products):
    command = ["cargo", *arguments, "--manifest-path", manifest, "--target-dir", target,
               "--offline", "--message-format=json-render-diagnostics"]
    with tempfile.TemporaryFile(mode="w+") as messages:
        subprocess.run(command, cwd=ROOT, stdout=messages, check=True)
        messages.seek(0)
        found = set()
        for line in messages:
            artifact = json.loads(line)
            name = artifact.get("target", {}).get("name")
            exe = artifact.get("executable")
            if exe and name in products:
                shutil.copy2(exe, dest / products[name])
                found.add(name)
        if found != set(products):
            raise RuntimeError(f"missing Cargo artifacts: {set(products) - found}")


def insert(path, needle, replacement):
    source = path.read_text()
    if source.count(needle) != 1:
        raise ValueError(f"{path} does not have the expected benchmark interface: {needle}")
    path.write_text(source.replace(needle, replacement))


def reference(stage, target, dest, tag):
    archive(tag, stage, ["crates/ryo", "crates/tastemaker"])
    tm = stage / "crates/tastemaker"
    cli = tm / "cli"
    adapter = tm / "crates/core/src/outbound/analysis/adapters/ryo"
    shutil.copyfile(HERE / "worker/capture.rs.in", adapter / "capture.rs")
    insert(adapter / "adapter.rs", "use ryo::linter::HostKind;", '#[path = "capture.rs"]\nmod capture;\nuse ryo::linter::HostKind;')
    needle = "let sdk = schemas::request(request)?;"
    insert(adapter / "adapter.rs", needle, needle + "\n        let stamp = capture::record(&sdk, &self.languages);")
    insert(adapter / "adapter.rs", ".map(report)", ".inspect(|r| capture::answer(stamp, r))\n            .map(report)")
    insert(tm / "crates/core/Cargo.toml", "[dependencies]", "[dependencies]\nserde_json.workspace = true\nsha2.workspace = true")
    insert(cli / "Cargo.toml", "[dependencies]", '[[bin]]\nname = "tm-case-reference"\npath = "src/case_reference.rs"\n\n[dependencies]\nsha2.workspace = true')
    # Reuse the reader harness's typed host cells, parameter conversion and row encoding.
    codec = (ROOT / "crates/ryo/cli/benches/readers/shipping.rs").read_text().split("pub(super) fn run(")[0]
    # The frozen oracle supplies complete SDK cohorts in its own request envelope.
    imports, declarations = codec.split("#[derive(serde::Deserialize)]\nstruct Request {", 1)
    _, hosted = declarations.split("#[derive(serde::Deserialize)]\nstruct Hosted {", 1)
    codec = imports.replace("path::{Path, PathBuf}", "path::PathBuf") + "#[derive(serde::Deserialize)]\nstruct Hosted {" + hosted
    allocator = (cli / "src/main.rs").read_text().split("fn main()")[0]
    (cli / "src/case_reference.rs").write_text(allocator + codec.replace("//!", "//") + (HERE / "worker/reference.rs.in").read_text())
    build(tm / "Cargo.toml", target, dest,
          ["build", "--profile", "bench", "-p", "idiomatic-sh-tastemaker", "--bin", "tm", "--bin", "tm-case-reference"],
          {"tm": "tm-observer", "tm-case-reference": "tm-case-reference"})


def candidate(stage, target, dest):
    archive("HEAD", stage, ["crates/ryo", "crates/tastemaker/benchmarks/project"])
    engine = stage / "crates/ryo"
    manifest = engine / "cli/Cargo.toml"
    bench = engine / "cli/benches/readers"
    source = (bench / "bench.rs").read_text()
    if source.count("fn main()") != 1:
        raise ValueError("reader benchmark entry point changed")
    (bench / "tm_check.rs").write_text(source.split("fn main()")[0] + (HERE / "worker/candidate.rs.in").read_text())
    with manifest.open("a") as f:
        f.write('\n[[bench]]\nname = "tm-check-reader"\npath = "benches/readers/tm_check.rs"\nharness = false\n')
    build(engine / "Cargo.toml", target, dest, ["bench", "--no-run", "--locked", "-p", "idiomatic-sh-ryo", "--bench", "tm-check-reader"],
          {"tm-check-reader": "tm-check-reader"})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--target-dir", type=Path, default=os.environ.get("CARGO_TARGET_DIR"))
    parser.add_argument("--only", choices=["reference", "candidate"])
    args = parser.parse_args()
    dest = args.output.resolve()
    dest.mkdir(parents=True, exist_ok=True)
    target = Path(args.target_dir or dest / "target").resolve()
    version = tomllib.loads((ROOT / "tastemaker.toml").read_text())["tastemaker"]
    tag = f"idiomatic-sh-tastemaker-v{version}"
    for name in ("reference", "candidate"):
        if args.only and name != args.only:
            continue
        with tempfile.TemporaryDirectory(prefix=name + "-", dir=dest) as scratch:
            if name == "reference":
                reference(Path(scratch), target, dest, tag)
            else:
                candidate(Path(scratch), target, dest)
        names = ("tm-observer", "tm-case-reference") if name == "reference" else ("tm-check-reader",)
        binaries = {}
        for binary in names:
            with (dest / binary).open("rb") as stream:
                binaries[binary] = hashlib.file_digest(stream, "sha256").hexdigest()
        (dest / (name + ".json")).write_text(json.dumps(dict(sources=source_identity(name), binaries=binaries)) + "\n")
