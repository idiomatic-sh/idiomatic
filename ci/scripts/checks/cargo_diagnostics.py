#!/usr/bin/env python3
"""Forward Cargo JSON while printing compiler errors to stderr for retained CI diagnostics.

The clippy tools run with `--cap-lints warn`, so a denied lint is a warning here: the tool forwards a lint diagnostic only when
its code is set to `deny` or `forbid` in the workspace's own lint tables (`[workspace.lints.rust]` and `[workspace.lints.clippy]`
of the Cargo.toml in the working directory, read at run time: one definition), and only when its primary span is a file of the
workspace, not a toolchain source. Compiler errors and every non-diagnostic message pass unchanged. Asymptote's lock then judges
the denied findings; warnings stay outside it."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path
from typing import TextIO


def denied_codes(manifest: Path) -> set[str]:
    """The lint codes the workspace's Cargo.toml sets to deny or forbid: rust lints by name, clippy lints as `clippy::<name>`."""
    tables = tomllib.loads(manifest.read_text(encoding="utf-8")).get("workspace", {}).get("lints", {})
    found = set()
    for tool, prefix in (("rust", ""), ("clippy", "clippy::")):
        for name, value in tables.get(tool, {}).items():
            level = value if isinstance(value, str) else value.get("level") if isinstance(value, dict) else None
            if level in ("deny", "forbid"):
                found.add(prefix + name)
    return found


def in_workspace(diagnostic: dict) -> bool:
    """Whether the diagnostic's primary span is a file of the workspace: a relative path, not a toolchain source."""
    spans = [span for span in diagnostic.get("spans") or [] if isinstance(span, dict) and span.get("is_primary")]
    return bool(spans) and all(not str(span.get("file_name", "")).startswith(("/", "..")) for span in spans)


def forward(messages: TextIO, cargo_json: TextIO, diagnostics: TextIO, denied: set[str] | None = None) -> None:
    """Pass every Cargo message through but the lint diagnostics outside the denied set (or outside the workspace), rendering
    compiler errors on the separate diagnostic stream. With no denied set every message passes."""
    for line in messages:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            cargo_json.write(line)
            continue
        if not isinstance(message, dict) or message.get("reason") != "compiler-message":
            cargo_json.write(line)
            continue
        diagnostic = message.get("message")
        if not isinstance(diagnostic, dict):
            cargo_json.write(line)
            continue
        if diagnostic.get("level") == "error":
            cargo_json.write(line)
            rendered = diagnostic.get("rendered")
            diagnostics.write(rendered if isinstance(rendered, str) else f"{diagnostic.get('message', 'compiler error')}\n")
            diagnostics.flush()
            continue
        code = (diagnostic.get("code") or {}).get("code") if isinstance(diagnostic.get("code"), dict) else None
        if denied is None or (code in denied and in_workspace(diagnostic)):
            cargo_json.write(line)
    cargo_json.flush()


def self_test() -> None:
    """Hold the stream filter to one rendered error and an unchanged Cargo stream."""
    from io import StringIO

    error = json.dumps(
        {
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "message": "synthetic failure",
                "rendered": "error[E0001]: synthetic failure\n",
            },
        }
    ) + "\n"
    warning = json.dumps(
        {"reason": "compiler-message", "message": {"level": "warning", "message": "not an error"}}
    ) + "\n"
    other = json.dumps({"reason": "build-finished", "success": False}) + "\n"
    source = StringIO(error + warning + other)
    forwarded = StringIO()
    rendered = StringIO()
    forward(source, forwarded, rendered)
    assert forwarded.getvalue() == error + warning + other
    assert rendered.getvalue() == "error[E0001]: synthetic failure\n"


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    elif sys.argv[1:]:
        raise SystemExit("usage: cargo_diagnostics.py [--self-test]")
    else:
        forward(sys.stdin, sys.stdout, sys.stderr, denied_codes(Path("Cargo.toml")))
