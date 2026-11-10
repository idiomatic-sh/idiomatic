"""What `cargo_diagnostics.py` forwards to clippy-sarif: compiler errors, and a lint only when the workspace denies its code and
its primary span is a workspace file."""

from __future__ import annotations

import importlib.util
import json
from io import StringIO
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("cargo_diagnostics", Path(__file__).with_name("cargo_diagnostics.py"))
diagnostics = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(diagnostics)


def lint(code: str, path: str, level: str = "warning") -> str:
    message = {"level": level, "message": code, "code": {"code": code}, "spans": [{"file_name": path, "is_primary": True}]}
    return json.dumps({"reason": "compiler-message", "message": message}) + "\n"


def test_a_denied_code_is_forwarded_and_a_warn_code_or_a_toolchain_path_is_dropped(tmp_path) -> None:
    manifest = tmp_path / "Cargo.toml"
    manifest.write_text(
        '[workspace.lints.rust]\nunused_must_use = "deny"\nmissing_docs = "warn"\n'
        '[workspace.lints.clippy]\nas_conversions = "deny"\npedantic = { level = "warn", priority = -1 }\n'
        'correctness = { level = "deny", priority = -1 }\n',
        encoding="utf-8",
    )
    denied = diagnostics.denied_codes(manifest)
    assert denied == {"unused_must_use", "clippy::as_conversions", "clippy::correctness"}
    kept = lint("clippy::as_conversions", "crates/values/src/a.rs")
    error = lint("E0308", "crates/values/src/a.rs", level="error")
    finished = json.dumps({"reason": "build-finished", "success": True}) + "\n"
    source = StringIO(
        kept
        + lint("missing_docs", "crates/values/src/a.rs")
        + lint("clippy::as_conversions", "/usr/local/rustup/toolchains/x/lib/rustlib/src/rust/library/core/src/macros/mod.rs")
        + lint("unused_must_use", "../elsewhere/src/b.rs")
        + error
        + finished
    )
    forwarded, rendered = StringIO(), StringIO()
    diagnostics.forward(source, forwarded, rendered, denied)
    assert forwarded.getvalue() == kept + error + finished
    assert rendered.getvalue() == "E0308\n"
