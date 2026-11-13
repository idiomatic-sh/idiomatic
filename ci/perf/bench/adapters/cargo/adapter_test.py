"""What the cargo adapter of `bench` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
import hashlib
import json
from pathlib import Path

import pytest

from ci.perf.bench.adapters.cargo.adapter import (BENCH_DIRECTORY, FIXTURE_COPY, FIXTURE_HOOK, FIXTURE_MODULE, FIXTURE_HELPER, CargoBench, LanguageBench,
                                                  cargo_bench, decode, declared_metrics, measure_languages, measures, registry)
from ci.perf.bench.interface_test import conforms
from ci.perf.use_case import Refused

DOCUMENT = '{"ruff": {"cold-milliseconds": {"value": 831, "lower_value": 794, "upper_value": 893}, "peak-megabytes": {"value": 546}}}'


def fixture_setup(head, tree):
    """A historical driver and a distinct, benchmark-owned setup helper supplied by head."""
    helper = head / FIXTURE_HELPER
    helper.parent.mkdir(parents=True, exist_ok=True)
    helper.write_text("// benchmark-owned scratch normalization helper\n")
    driver = tree / BENCH_DIRECTORY / "use_case.rs"
    driver.parent.mkdir(parents=True, exist_ok=True)
    original = "/// The core's warm and cold `check`.\nfn target() {\n" + FIXTURE_COPY + "}\n"
    driver.write_text(original)
    return driver, original, helper


def test_the_cargo_adapter_implements_the_port() -> None:
    """It is a `bench` adapter."""
    conforms(CargoBench)


def test_a_bench_runs_as_the_products_asymptote_toml_runs_it() -> None:
    assert cargo_bench("commands") == ["cargo", "bench", "--locked", "--workspace", "--bench", "commands"]
    assert cargo_bench("distribution") == ["cargo", "bench", "--locked", "--workspace", "--all-features", "--bench", "distribution"]


def test_each_measure_is_its_benchmark_and_measure_with_the_value_the_bench_reports() -> None:
    assert measures(DOCUMENT) == {"ruff/cold-milliseconds": 831.0, "ruff/peak-megabytes": 546.0}


def test_both_trees_build_with_the_checkout_s_toolchain_and_the_document_is_kept(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    calls = []

    def run(argv, **kwargs):
        calls.append((kwargs["cwd"], kwargs["env"]["RUSTUP_TOOLCHAIN"]))
        return subprocess.CompletedProcess(argv, 0, DOCUMENT, "")

    monkeypatch.setattr(subprocess, "run", run)
    bench = CargoBench(tmp_path, tmp_path / "out")
    assert bench.measure(tmp_path / "base", "ryo", "lint", "base-1") == measures(DOCUMENT)
    assert calls == [(tmp_path / "base/crates/ryo", "1.97.1")]
    assert (tmp_path / "out/base-1-ryo-lint.json").read_text() == DOCUMENT


def test_failed_bench_keeps_stdout_stderr_and_command_for_the_failure_artifact(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    fixture_setup(tmp_path, tmp_path / "head")

    def run(argv, **kwargs):
        return subprocess.CompletedProcess(argv, 101, "compiler json diagnostics", "cargo failed")

    monkeypatch.setattr(subprocess, "run", run)
    bench = CargoBench(tmp_path, tmp_path / "out")
    try:
        bench.measure(tmp_path / "head", "tastemaker", "commands", "head-1")
    except subprocess.CalledProcessError as error:
        assert error.returncode == 101
        assert error.stdout == "compiler json diagnostics"
        assert error.stderr == "cargo failed"
    else:
        raise AssertionError("failed cargo benchmark must remain a failure")
    assert (tmp_path / "out/head-1-tastemaker-commands.stdout.txt").read_text() == "compiler json diagnostics"
    assert (tmp_path / "out/head-1-tastemaker-commands.stderr.txt").read_text() == "cargo failed"
    assert '"status": 101' in (tmp_path / "out/head-1-tastemaker-commands.command.json").read_text()


def test_historical_benchmark_setup_is_explicit_and_fully_restored(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    tree = tmp_path / "base"
    driver, original, helper = fixture_setup(tmp_path, tree)
    manifest = tree / "crates/ryo/tastemaker.toml"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text('tastemaker = "0.1.0"\n')

    def run(argv, **kwargs):
        assert FIXTURE_HOOK in driver.read_text()
        assert FIXTURE_MODULE in driver.read_text()
        assert (tree / FIXTURE_HELPER).read_bytes() == helper.read_bytes()
        assert manifest.read_text() == 'tastemaker = "0.1.0"\n', "the adapter never rewrites a production manifest"
        return subprocess.CompletedProcess(argv, 0, DOCUMENT, "BENCH_FIXTURE_VERSION recorded\n")

    monkeypatch.setattr(subprocess, "run", run)
    assert CargoBench(tmp_path, None).measure(tree, "tastemaker", "commands", "base-1") == measures(DOCUMENT)
    assert driver.read_text() == original
    assert not (tree / FIXTURE_HELPER).exists()
    record = tmp_path / "target/benchmarks/speed/base-1-tastemaker-commands.command.json"
    evidence = json.loads(record.read_text())["benchmark_setup"]
    assert evidence["execution"] == "audited-benchmark-setup-overlay"
    assert evidence["original_driver_sha256"] == hashlib.sha256(original.encode()).hexdigest()
    assert evidence["original_driver_sha256"] != evidence["effective_driver_sha256"]
    assert evidence["helper_sha256"] == hashlib.sha256(helper.read_bytes()).hexdigest()
    assert evidence["helper_source"] == helper.read_text()
    assert FIXTURE_HOOK.splitlines()[0] in evidence["driver_patch"]


def test_current_benchmark_uses_the_same_setup_without_an_extra_hook(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    driver, original, helper = fixture_setup(tmp_path, tmp_path)
    current = FIXTURE_MODULE + original.replace(FIXTURE_COPY, FIXTURE_COPY + FIXTURE_HOOK)
    driver.write_text(current)

    def run(argv, **kwargs):
        assert driver.read_text() == current
        return subprocess.CompletedProcess(argv, 0, DOCUMENT, "")

    monkeypatch.setattr(subprocess, "run", run)
    CargoBench(tmp_path, tmp_path / "out").measure(tmp_path, "tastemaker", "commands", "head-1")
    assert driver.read_text() == current
    assert helper.exists()
    evidence = json.loads((tmp_path / "out/head-1-tastemaker-commands.command.json").read_text())["benchmark_setup"]
    assert evidence["driver_patch"] == ""
    assert evidence["original_driver_sha256"] == evidence["effective_driver_sha256"]


def test_exception_restores_the_original_driver_and_existing_helper(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    tree = tmp_path / "base"
    driver, original, _ = fixture_setup(tmp_path, tree)
    old_helper = tree / FIXTURE_HELPER
    old_helper.parent.mkdir(parents=True, exist_ok=True)
    old_helper.write_text("// original historical helper\n")

    def run(argv, **kwargs):
        raise OSError("cargo cannot start")

    monkeypatch.setattr(subprocess, "run", run)
    with pytest.raises(OSError, match="cargo cannot start"):
        CargoBench(tmp_path, None).measure(tree, "tastemaker", "commands", "base-1")
    assert driver.read_text() == original
    assert old_helper.read_text() == "// original historical helper\n"


def test_unknown_historical_setup_is_refused_without_mutation(monkeypatch, tmp_path) -> None:
    (tmp_path / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.97.1"\n')
    driver, _, _ = fixture_setup(tmp_path, tmp_path / "base")
    driver.write_text("unknown benchmark driver\n")

    def run(argv, **kwargs):
        raise AssertionError("unsupported setup must not launch cargo")

    monkeypatch.setattr(subprocess, "run", run)
    with pytest.raises(Refused, match="unsupported historical"):
        CargoBench(tmp_path, None).measure(tmp_path / "base", "tastemaker", "commands", "base-1")
    assert driver.read_text() == "unknown benchmark driver\n"


# Pinned native table and existing registry contracts, with no Cargo/runtime workload execution.

TABLE = """provider                       fastest │ slowest │ median │ mean │ samples │ iters
╰─ generated_every_relation               │         │        │      │         │
   ├─ 2500                         1 µs  │ 3 µs    │ 2 µs   │ 2 µs │ 100     │ 100
   ╰─ 10000                        4 ms  │ 6 ms    │ 5 ms   │ 5 ms │ 100     │ 100
"""
HISTORICAL_TABLE = TABLE.replace("provider                       fastest", "performance                       fastest", 1)
METADATA = '\n[package.metadata.ryo.performance]\nschema="divan-timing/v1"\nworkloads={generated_every_relation=[2500,10000]}\nmetrics=["median-ns"]\n'
SOURCE = b'// unchanged established parser benchmark workload\n'


def tree(root: Path, language: str = "python", *, metadata: str = METADATA, historical: bool = False) -> Path:
    workspace = root / "crates/ryo"
    directory = workspace / "languages" / language
    target = "performance" if historical else "provider"
    source_path = "benches/performance.rs" if historical else "benchmarks/provider/bench.rs"
    (directory / source_path).parent.mkdir(parents=True, exist_ok=True)
    (workspace / "Cargo.toml").write_text('[workspace]\nmembers=["languages/*"]\n')
    (workspace / "Cargo.lock").write_text('[[package]]\nname="divan"\nversion="0.1.21"\n')
    (directory / "Cargo.toml").write_text(f'[package]\nname="idiomatic-sh-ryo-{language}"\n[[bench]]\nname="{target}"\npath="{source_path}"\nharness=false\n' + metadata)
    (directory / source_path).write_bytes(SOURCE)
    return root


def spec(tmp_path) -> LanguageBench:
    return registry(tree(tmp_path))[0]


def test_native_timing_units_and_explicit_declared_leaf_paths(tmp_path) -> None:
    expected = {"python/generated_every_relation/2500/median-ns": 2000.0,
                "python/generated_every_relation/10000/median-ns": 5_000_000.0}
    assert decode(TABLE, spec(tmp_path)) == expected
    assert declared_metrics(tmp_path) == set(expected)


@pytest.mark.parametrize("document", [
    "", TABLE.replace("   ╰─ 10000", "   ╰─ 10001"),
    TABLE.replace("   ╰─ 10000                        4 ms  │ 6 ms    │ 5 ms   │ 5 ms │ 100     │ 100\n", ""),
    TABLE + "   ╰─ 2500                         1 µs  │ 3 µs    │ 2 µs   │ 2 µs │ 100     │ 100\n",
    TABLE.replace("2 µs   │ 2 µs", "NaN ns │ 2 µs"), TABLE.replace("2 µs   │ 2 µs", "0 ns   │ 2 µs"),
    TABLE.replace("2 µs", "2 ticks"), TABLE.replace("100     │ 100", "0       │ 100"),
    TABLE.replace("╰─ generated_every_relation", "╰─ missing_workload"),
    TABLE + "ignored unexpected output\n", TABLE.replace("median", "cpu-ms"),
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
    manifest.write_text(original.replace('name="provider"', 'name="other"'))
    with pytest.raises(Refused, match="explicit provider target"):
        registry(root)
    manifest.write_text(original)
    (root / "crates/ryo/Cargo.lock").write_text('[[package]]\nname="divan"\nversion="0.1.22"\n')
    with pytest.raises(Refused, match="pinned Divan"):
        registry(root)
    tree(root)
    (root / "crates/ryo/Cargo.toml").write_text('[workspace]\nmembers=[]\n')
    with pytest.raises(Refused, match="absent from the Cargo workspace"):
        registry(root)


def test_current_registry_rejects_old_and_ambiguous_bench_targets(tmp_path) -> None:
    root = tree(tmp_path)
    manifest = root / "crates/ryo/languages/python/Cargo.toml"
    current = manifest.read_text()
    manifest.write_text(current.replace('name="provider"\npath="benchmarks/provider/bench.rs"',
                                        'name="performance"\npath="benches/performance.rs"'))
    with pytest.raises(Refused, match="explicit provider target"):
        registry(root)
    manifest.write_text(current)
    legacy = root / "crates/ryo/languages/python/benches/performance.rs"
    legacy.parent.mkdir(parents=True, exist_ok=True)
    legacy.write_bytes(SOURCE)
    with pytest.raises(Refused, match="legacy provider bench remains"):
        registry(root)
    legacy.unlink()
    manifest.write_text(current + '\n[[bench]]\nname="other"\npath="benchmarks/misc/bench.rs"\nharness=false\n')
    with pytest.raises(Refused, match="misplaced or ambiguous benchmark target"):
        registry(root)
    manifest.write_text(current + '\n[[bench]]\nname="performance"\npath="benches/performance.rs"\nharness=false\n')
    with pytest.raises(Refused, match="explicit provider target"):
        registry(root)


def test_historical_metadata_fallback_requires_the_same_existing_target_and_source(tmp_path) -> None:
    head = tree(tmp_path / "head")
    base = tree(tmp_path / "base", metadata="", historical=True)
    required = {entry.name: entry for entry in registry(head)}
    assert registry(base, required)[0].legacy
    (base / "crates/ryo/languages/python/benches/performance.rs").write_bytes(b"changed workload")
    with pytest.raises(Refused, match="baseline-required"):
        registry(base, required)
    tree(base, metadata="", historical=True)
    tree(base, "json")
    with pytest.raises(Refused, match="inventories differ"):
        registry(base, required)


def test_native_measurement_keeps_raw_command_and_exact_compatibility_provenance(monkeypatch, tmp_path) -> None:
    head = tree(tmp_path / "head")
    base = tree(tmp_path / "base", metadata="", historical=True)
    calls = []
    def run(command, **options):
        calls.append((command, options))
        return subprocess.CompletedProcess(command, 0, HISTORICAL_TABLE, "Timer precision: 1 ns\n")
    monkeypatch.setattr(subprocess, "run", run)
    measured = measure_languages(head, base, tmp_path / "out", "base-1", "1.97.1")
    assert set(measured) == declared_metrics(head)
    command, options = calls[0]
    assert command == ["cargo", "bench", "--locked", "--package", "idiomatic-sh-ryo-python", "--bench", "performance", "--", "--color", "never"]
    assert options["env"]["RUSTUP_TOOLCHAIN"] == "1.97.1"
    evidence = json.loads((tmp_path / "out/base-1-ryo-languages-python.command.json").read_text())
    assert evidence["baseline_registry"] == "head-required/identical-established-source"
    assert evidence["source_path"] == "benches/performance.rs"
    assert evidence["target"] == "performance"
    assert evidence["required_metrics"] == sorted(measured)
    assert (tmp_path / "out/base-1-ryo-languages-python.stdout.txt").read_text() == HISTORICAL_TABLE
    assert not any("cpu" in key or "rss" in key for key in measured)


def test_current_measurement_uses_provider_target_without_changing_workload_keys(monkeypatch, tmp_path) -> None:
    head = tree(tmp_path / "head")
    calls = []
    def run(command, **options):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, TABLE, "")
    monkeypatch.setattr(subprocess, "run", run)
    measured = measure_languages(head, head, tmp_path / "out", "head-1", "1.97.1")
    assert calls == [["cargo", "bench", "--locked", "--package", "idiomatic-sh-ryo-python",
                      "--bench", "provider", "--", "--color", "never"]]
    assert set(measured) == declared_metrics(head)
    evidence = json.loads((tmp_path / "out/head-1-ryo-languages-python.command.json").read_text())
    assert evidence["target"] == "provider"
    assert evidence["source_path"] == "benchmarks/provider/bench.rs"


def test_failed_native_process_retains_raw_failure_and_never_returns_metrics(monkeypatch, tmp_path) -> None:
    head = tree(tmp_path / "head")
    monkeypatch.setattr(subprocess, "run", lambda command, **_: subprocess.CompletedProcess(command, 101, "partial", "growth assertion failed"))
    with pytest.raises(subprocess.CalledProcessError):
        measure_languages(head, head, tmp_path / "out", "head-1", "1.97.1")
    assert (tmp_path / "out/head-1-ryo-languages-python.stdout.txt").read_text() == "partial"
    assert not (tmp_path / "out/head-1-ryo-languages.metrics.json").exists()


@pytest.mark.parametrize("changed", ["Cargo.lock", "Cargo.toml", "languages/python/Cargo.toml", "languages/python/benchmarks/provider/bench.rs"])
def test_input_mutation_during_measurement_is_refused(monkeypatch, tmp_path, changed) -> None:
    head = tree(tmp_path / "head")
    def run(command, **_):
        path = head / "crates/ryo" / changed
        path.write_bytes(path.read_bytes() + b"\n# changed during measurement\n")
        return subprocess.CompletedProcess(command, 0, TABLE, "")
    monkeypatch.setattr(subprocess, "run", run)
    with pytest.raises(Refused, match="changed during measurement"):
        measure_languages(head, head, tmp_path / "out", "head-1", "1.97.1")
    assert not (tmp_path / "out/head-1-ryo-languages.metrics.json").exists()
