"""What `cargo_diagnostics.py` forwards to clippy-sarif: compiler errors, and a lint only when the compiler's own level note
says the lint is denied in its package (or the level cannot be read) and its primary span is a workspace file.

The note texts are rustc's, captured from clippy-driver 1.97.1 with `--cap-lints warn` and `--error-format=json`."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from io import StringIO
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("cargo_diagnostics", Path(__file__).with_name("cargo_diagnostics.py"))
diagnostics = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(diagnostics)

VALUES = "path+file:///w/crates/ryo/crates/values#idiomatic-sh-ryo-values@0.1.0"
CORE = "path+file:///w/crates/ryo/crates/core#idiomatic-sh-ryo-core@0.1.0"


def lint(code: str, path: str, note: str | None = None, *, package: str = VALUES, level: str = "warning",
         attribute: str | None = None) -> str:
    """One cargo `compiler-message` line: a lint diagnostic, with rustc's level note as its child when given (and, for "the lint
    level is defined here", the attribute's text on the note's span)."""
    children = []
    if note is not None:
        spans = [] if attribute is None else [{"file_name": path, "is_primary": True, "text": [{"text": attribute}]}]
        children.append({"message": note, "code": None, "level": "note", "spans": spans, "children": [], "rendered": None})
    message = {"level": level, "message": code, "code": {"code": code}, "children": children,
               "spans": [{"file_name": path, "is_primary": True}]}
    return json.dumps({"reason": "compiler-message", "package_id": package, "message": message}) + "\n"


def run(source: str) -> tuple[str, str]:
    forwarded, rendered = StringIO(), StringIO()
    diagnostics.forward(StringIO(source), forwarded, rendered)
    return forwarded.getvalue(), rendered.getvalue()


def test_a_lint_denied_through_its_group_is_forwarded(tmp_path) -> None:
    """Through the script as the clippy tools run it, in a workspace whose Cargo.toml denies the group: the member is forwarded."""
    (tmp_path / "Cargo.toml").write_text('[workspace.lints.clippy]\ncorrectness = { level = "deny", priority = -1 }\n'
                                         'pedantic = { level = "warn", priority = -1 }\n', encoding="utf-8")
    denied = lint("clippy::absurd_extreme_comparisons", "crates/values/src/a.rs",
                  "`-D clippy::absurd-extreme-comparisons` implied by `-D clippy::correctness`")
    warned = lint("clippy::cast_lossless", "crates/values/src/a.rs", "`-W clippy::cast-lossless` implied by `-W clippy::pedantic`")
    script = Path(__file__).with_name("cargo_diagnostics.py")
    done = subprocess.run([sys.executable, "-I", str(script)], input=denied + warned, cwd=tmp_path, capture_output=True,
                          text=True, check=False)
    assert (done.returncode, done.stdout) == (0, denied)


def test_a_denied_lint_is_forwarded_and_a_warned_lint_an_error_or_a_toolchain_path_is_handled_as_before() -> None:
    kept = lint("clippy::as_conversions", "crates/values/src/a.rs", "requested on the command line with `-D clippy::as-conversions`")
    attribute = lint("clippy::cast_possible_truncation", "crates/values/src/b.rs", "the lint level is defined here",
                     attribute="#[deny(clippy::cast_possible_truncation)]")
    error = lint("E0308", "crates/values/src/a.rs", level="error")
    finished = json.dumps({"reason": "build-finished", "success": True}) + "\n"
    source = (
        kept
        + attribute
        + lint("missing_docs", "crates/values/src/a.rs", "requested on the command line with `-W missing-docs`")
        + lint("clippy::as_conversions", "/usr/local/rustup/toolchains/x/lib/rustlib/src/rust/library/core/src/macros/mod.rs")
        + lint("clippy::as_conversions", "../elsewhere/src/b.rs")
        + error
        + finished
    )
    assert run(source) == (kept + attribute + error + finished, "E0308\n")


def test_a_default_deny_lint_is_forwarded_and_a_default_warn_lint_is_dropped() -> None:
    denied = lint("arithmetic_overflow", "crates/values/src/a.rs", "`#[deny(arithmetic_overflow)]` on by default")
    warned = lint("unused_comparisons", "crates/values/src/a.rs", "`#[warn(unused_comparisons)]` on by default")
    assert run(denied + warned)[0] == denied


def test_a_later_diagnostic_without_a_note_takes_its_lints_level_in_that_package() -> None:
    denied = lint("clippy::as_conversions", "crates/values/src/a.rs", "requested on the command line with `-D clippy::as-conversions`")
    denied_again = lint("clippy::as_conversions", "crates/values/src/c.rs")
    warned = lint("clippy::must_use_candidate", "crates/values/src/a.rs",
                  "`-W clippy::must-use-candidate` implied by `-W clippy::pedantic`")
    warned_again = lint("clippy::must_use_candidate", "crates/values/src/c.rs")
    elsewhere = lint("clippy::as_conversions", "crates/core/src/a.rs", "requested on the command line with `-W clippy::as-conversions`",
                     package=CORE)
    elsewhere_again = lint("clippy::as_conversions", "crates/core/src/b.rs", package=CORE)
    forwarded = run(denied + denied_again + warned + warned_again + elsewhere + elsewhere_again)[0]
    assert forwarded == denied + denied_again


def test_a_diagnostic_whose_level_cannot_be_read_is_forwarded() -> None:
    unread = lint("clippy::as_conversions", "crates/values/src/a.rs", "a level note in a shape this script does not know")
    unnoted = lint("clippy::indexing_slicing", "crates/values/src/a.rs")
    forwarded, rendered = run(unread + unnoted)
    assert forwarded == unread + unnoted
    assert rendered.splitlines() == [
        "cargo_diagnostics.py: forwarded clippy::as_conversions, whose lint level the compiler's notes do not give",
        "cargo_diagnostics.py: forwarded clippy::indexing_slicing, whose lint level the compiler's notes do not give",
    ]
