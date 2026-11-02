"""Pinned native table and existing registry contracts, with no Cargo/runtime workload execution."""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from ci.perf.bench.adapters.cargo.language import LanguageBench, decode, declared_metrics, measure_languages, registry
from ci.perf.use_case import Refused

DOCUMENT = """performance                       fastest │ slowest │ median │ mean │ samples │ iters
╰─ generated_every_relation               │         │        │      │         │
   ├─ 2500                         1 µs  │ 3 µs    │ 2 µs   │ 2 µs │ 100     │ 100
   ╰─ 10000                        4 ms  │ 6 ms    │ 5 ms   │ 5 ms │ 100     │ 100
"""
METADATA = '\n[package.metadata.ryo.performance]\nschema="divan-timing/v1"\nworkloads={generated_every_relation=[2500,10000]}\nmetrics=["median-ns"]\n'
SOURCE = b'// unchanged established parser benchmark workload\n'


def tree(root: Path, language: str = "python", *, metadata: str = METADATA) -> Path:
    workspace = root / "crates/ryo"
    directory = workspace / "languages" / language
    (directory / "benches").mkdir(parents=True, exist_ok=True)
    (workspace / "Cargo.toml").write_text('[workspace]\nmembers=["languages/*"]\n')
    (workspace / "Cargo.lock").write_text('[[package]]\nname="divan"\nversion="0.1.21"\n')
    (directory / "Cargo.toml").write_text(f'[package]\nname="idiomatic-sh-ryo-{language}"\n[[bench]]\nname="performance"\npath="benches/performance.rs"\nharness=false\n' + metadata)
    (directory / "benches/performance.rs").write_bytes(SOURCE)
    return root


def spec(tmp_path) -> LanguageBench:
    return registry(tree(tmp_path))[0]


def test_native_timing_units_and_explicit_declared_leaf_paths(tmp_path) -> None:
    expected = {"python/generated_every_relation/2500/median-ns": 2000.0,
                "python/generated_every_relation/10000/median-ns": 5_000_000.0}
    assert decode(DOCUMENT, spec(tmp_path)) == expected
    assert declared_metrics(tmp_path) == set(expected)


@pytest.mark.parametrize("document", [
    "", DOCUMENT.replace("   ╰─ 10000", "   ╰─ 10001"),
    DOCUMENT.replace("   ╰─ 10000                        4 ms  │ 6 ms    │ 5 ms   │ 5 ms │ 100     │ 100\n", ""),
    DOCUMENT + "   ╰─ 2500                         1 µs  │ 3 µs    │ 2 µs   │ 2 µs │ 100     │ 100\n",
    DOCUMENT.replace("2 µs   │ 2 µs", "NaN ns │ 2 µs"), DOCUMENT.replace("2 µs   │ 2 µs", "0 ns   │ 2 µs"),
    DOCUMENT.replace("2 µs", "2 ticks"), DOCUMENT.replace("100     │ 100", "0       │ 100"),
    DOCUMENT.replace("╰─ generated_every_relation", "╰─ missing_workload"),
    DOCUMENT + "ignored unexpected output\n", DOCUMENT.replace("median", "cpu-ms"),
])
def test_missing_extra_duplicate_invalid_or_unrecognised_native_results_refuse(document, tmp_path) -> None:
    with pytest.raises(Refused):
        decode(document, spec(tmp_path))


@pytest.mark.parametrize("metadata", [
    "", METADATA.replace('"divan-timing/v1"', '"other"'), METADATA.replace('[2500,10000]', '[]'),
    METADATA.replace('[2500,10000]', '[2500,2500]'), METADATA.replace('[2500,10000]', '[true,10000]'),
    METADATA.replace('["median-ns"]', '["cpu-ms"]'), METADATA.replace('workloads={generated_every_relation=[2500,10000]}', 'workloads={}'),
])
def test_declared_workload_registry_is_required_and_validated(metadata, tmp_path) -> None:
    with pytest.raises(Refused):
        registry(tree(tmp_path, metadata=metadata))


def test_missing_target_unpinned_decoder_and_unregistered_language_refuse(tmp_path) -> None:
    root = tree(tmp_path)
    manifest = root / "crates/ryo/languages/python/Cargo.toml"
    original = manifest.read_text()
    manifest.write_text(original.replace('name="performance"', 'name="other"'))
    with pytest.raises(Refused, match="explicit performance target"):
        registry(root)
    manifest.write_text(original)
    (root / "crates/ryo/Cargo.lock").write_text('[[package]]\nname="divan"\nversion="0.1.22"\n')
    with pytest.raises(Refused, match="pinned Divan"):
        registry(root)
    tree(root)
    (root / "crates/ryo/Cargo.toml").write_text('[workspace]\nmembers=[]\n')
    with pytest.raises(Refused, match="absent from the Cargo workspace"):
        registry(root)


def test_historical_metadata_fallback_requires_the_same_existing_target_and_source(tmp_path) -> None:
    head = tree(tmp_path / "head")
    base = tree(tmp_path / "base", metadata="")
    required = {entry.name: entry for entry in registry(head)}
    assert registry(base, required)[0].legacy
    (base / "crates/ryo/languages/python/benches/performance.rs").write_bytes(b"changed workload")
    with pytest.raises(Refused, match="baseline-required"):
        registry(base, required)
    tree(base)
    tree(base, "json")
    with pytest.raises(Refused, match="inventories differ"):
        registry(base, required)


def test_native_measurement_keeps_raw_command_and_exact_compatibility_provenance(monkeypatch, tmp_path) -> None:
    head = tree(tmp_path / "head")
    base = tree(tmp_path / "base", metadata="")
    calls = []
    def run(command, **options):
        calls.append((command, options))
        return subprocess.CompletedProcess(command, 0, DOCUMENT, "Timer precision: 1 ns\n")
    monkeypatch.setattr(subprocess, "run", run)
    measured = measure_languages(head, base, tmp_path / "out", "base-1", "1.97.1")
    assert set(measured) == declared_metrics(head)
    command, options = calls[0]
    assert command == ["cargo", "bench", "--locked", "--package", "idiomatic-sh-ryo-python", "--bench", "performance", "--", "--color", "never"]
    assert options["env"]["RUSTUP_TOOLCHAIN"] == "1.97.1"
    evidence = json.loads((tmp_path / "out/base-1-ryo-languages-python.command.json").read_text())
    assert evidence["baseline_registry"] == "head-required/identical-established-source"
    assert evidence["required_metrics"] == sorted(measured)
    assert (tmp_path / "out/base-1-ryo-languages-python.stdout.txt").read_text() == DOCUMENT
    assert not any("cpu" in key or "rss" in key for key in measured)


def test_failed_native_process_retains_raw_failure_and_never_returns_metrics(monkeypatch, tmp_path) -> None:
    head = tree(tmp_path / "head")
    monkeypatch.setattr(subprocess, "run", lambda command, **_: subprocess.CompletedProcess(command, 101, "partial", "growth assertion failed"))
    with pytest.raises(subprocess.CalledProcessError):
        measure_languages(head, head, tmp_path / "out", "head-1", "1.97.1")
    assert (tmp_path / "out/head-1-ryo-languages-python.stdout.txt").read_text() == "partial"
    assert not (tmp_path / "out/head-1-ryo-languages.metrics.json").exists()


@pytest.mark.parametrize("changed", ["Cargo.lock", "Cargo.toml", "languages/python/Cargo.toml", "languages/python/benches/performance.rs"])
def test_input_mutation_during_measurement_is_refused(monkeypatch, tmp_path, changed) -> None:
    head = tree(tmp_path / "head")
    def run(command, **_):
        path = head / "crates/ryo" / changed
        path.write_bytes(path.read_bytes() + b"\n# changed during measurement\n")
        return subprocess.CompletedProcess(command, 0, DOCUMENT, "")
    monkeypatch.setattr(subprocess, "run", run)
    with pytest.raises(Refused, match="changed during measurement"):
        measure_languages(head, head, tmp_path / "out", "head-1", "1.97.1")
    assert not (tmp_path / "out/head-1-ryo-languages.metrics.json").exists()
