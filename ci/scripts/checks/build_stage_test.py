"""Build artifacts and mandatory quality remain independent CI verdicts."""
from pathlib import Path
import subprocess
import tomllib

import pytest

ROOT = Path(__file__).resolve().parents[3]
PRODUCTS = ("ryo", "tastemaker", "asymptote")


def tool(product, identity):
    data = tomllib.loads((ROOT / "crates" / product / "asymptote.toml").read_text())
    return next(item for item in data["tools"] if item["id"] == identity)


@pytest.mark.parametrize("product", PRODUCTS)
def test_compilation_preserves_diagnostics_and_failure_before_following_stages(tmp_path, product):
    command = tool(product, "build")["run"]
    assert command[:2] == ["bash", "-c"]
    fake = 'cargo() { printf "%s\\n" "$*" >> cargo-calls; printf "compiler stdout\\n"; printf "error: exact denied diagnostic\\n" >&2; return 101; }; '
    result = subprocess.run([*command[:2], fake + command[2]], cwd=tmp_path, capture_output=True, text=True)
    assert result.returncode == 101
    assert (tmp_path / "cargo-calls").read_text().splitlines() == ["check --workspace --all-targets --all-features --locked"]
    diagnostic = (tmp_path / "target/checks/build/compile.log").read_text()
    assert "compiler stdout" in diagnostic
    assert "error: exact denied diagnostic" in diagnostic
    assert not (tmp_path / "target/checks/build/test.log").exists()
    assert not (tmp_path / "target/checks/build/doc.log").exists()


@pytest.mark.parametrize("product", PRODUCTS)
def test_style_failure_has_its_own_mandatory_quality_group(product):
    build = tool(product, "build")
    quality = tool(product, "cargo-clippy")
    assert "cargo clippy" not in build["run"][2]
    assert quality["group"] == "clippy"
    assert "cargo clippy" in " ".join(quality["run"])
    workflow = (ROOT / ".github/workflows/check.yml").read_text()
    assert "{ runner: ubuntu-26.04, group: clippy }" in workflow


@pytest.mark.parametrize("status", [0, 101])
def test_rust_memory_guard_retains_actual_output_and_exit(tmp_path, status):
    data = tomllib.loads((ROOT / "asymptote.toml").read_text())
    command = next(item for item in data["tools"] if item["id"] == "ryo-rust-peak")["run"]
    fake = f'cargo() {{ printf "%s\\n" "$*" > cargo-calls; printf "peak stdout\\n"; printf "peak stderr\\n" >&2; return {status}; }}; '
    result = subprocess.run([*command[:2], fake + command[2]], cwd=tmp_path, capture_output=True, text=True)
    assert result.returncode == status
    assert (tmp_path / "cargo-calls").read_text().strip() == (
        "nextest run --manifest-path crates/ryo/Cargo.toml --package idiomatic-sh-ryo --locked -E test(/^peak::/)"
    )
    output = tmp_path / "target/benchmarks/speed"
    assert (output / "ryo-rust-peak.stdout.log").read_text() == result.stdout == "peak stdout\n"
    assert (output / "ryo-rust-peak.stderr.log").read_text() == result.stderr == "peak stderr\n"
    assert (output / "ryo-rust-peak.status").read_text() == f"{status}\n"
    workflow = (ROOT / ".github/workflows/check.yml").read_text()
    assert "if: always() && steps.speed_check.outcome != 'skipped'" in workflow


def test_ci_tests_runs_the_workspace_templates_pytest_line():
    data = tomllib.loads((ROOT / "asymptote.toml").read_text())
    command = next(item for item in data["tools"] if item["id"] == "ci-tests")["run"]
    assert command == ["pytest", "--quiet", "-p", "no:cacheprovider", "--import-mode=importlib", "-o", "pythonpath=.",
                       "-o", "python_files=*_test.py", "-o", "python_classes=", "-o", "norecursedirs=.* __pycache__", "ci"]


@pytest.mark.parametrize("status", [0, 101])
def test_failed_tool_stdout_is_retained_without_changing_the_command(tmp_path, status):
    data = tomllib.loads((ROOT / "crates/ryo/asymptote.toml").read_text())
    expected = ["cargo", "test", "--workspace", "--locked", "--all-features", "--test", "bdd"]
    output = tmp_path / "target/checks/bdd"
    command = next(item for item in data["tools"] if item["id"] == "test-bdd")["run"]
    fake = f'{expected[0]}() {{ printf "%s\\0" "$@" > child-arguments; printf "underlying failed scenario\\n"; printf "child stderr\\n" >&2; return {status}; }}; '
    result = subprocess.run([*command[:2], fake + command[2]], cwd=tmp_path, capture_output=True, text=True)
    assert result.returncode == status
    assert (tmp_path / "child-arguments").read_bytes().split(b"\0")[:-1] == [arg.encode() for arg in expected[1:]]
    assert (output / "stdout.log").read_text() == result.stdout == "underlying failed scenario\n"
    assert (output / "stderr.log").read_text() == result.stderr == "child stderr\n"
    assert (output / "status").read_text() == f"{status}\n"
