#!/usr/bin/env python3
"""Forward Cargo JSON while printing compiler errors to stderr for retained CI diagnostics."""

from __future__ import annotations

import json
import sys
from typing import TextIO


def forward(messages: TextIO, cargo_json: TextIO, diagnostics: TextIO) -> None:
    """Pass every Cargo message through, rendering compiler errors on the separate diagnostic stream."""
    for line in messages:
        cargo_json.write(line)
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(message, dict):
            continue
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message.get("message")
        if not isinstance(diagnostic, dict) or diagnostic.get("level") != "error":
            continue
        rendered = diagnostic.get("rendered")
        diagnostics.write(
            rendered
            if isinstance(rendered, str)
            else f"{diagnostic.get('message', 'compiler error')}\n"
        )
        diagnostics.flush()
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
        forward(sys.stdin, sys.stdout, sys.stderr)
