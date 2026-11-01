"""What the cargo adapter of `bench` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess

from ci.perf.bench.adapters.cargo.adapter import CargoBench, cargo_bench, measures
from ci.perf.bench.interface_test import conforms

DOCUMENT = '{"ruff": {"cold-milliseconds": {"value": 831, "lower_value": 794, "upper_value": 893}, "peak-megabytes": {"value": 546}}}'


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
