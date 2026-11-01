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
    assert "{ os: ubuntu-26.04, group: clippy }" in workflow
