#!/usr/bin/env python3
"""Forward Cargo JSON while printing compiler errors to stderr for retained CI diagnostics.

The clippy tools run with `--cap-lints warn`, so a denied lint is a warning here. The level a lint had before the cap is in the
compiler's own level note, which rustc attaches to the first diagnostic of each lint and level source in a package: "`-D x`
implied by `-D clippy::correctness`", "requested on the command line with `-D x`", "`#[deny(x)]` on by default", or "the lint
level is defined here" on the attribute's span. The tool forwards a lint diagnostic when its package has shown the lint at deny or
forbid, and also, with one line on stderr, when no level for it has been read (a note in a shape this tool does not know fails
the lock visibly instead of dropping the finding); in both cases only when its primary span is a file of the workspace, not a
toolchain source. Compiler errors and every non-diagnostic message pass unchanged. Asymptote's lock then judges the denied
findings; warnings stay outside it."""

from __future__ import annotations

import json
import re
import sys
from typing import TextIO

# The level of the first flag or attribute in a level note: `-D x`, `-W x implied by -W group`, `#[deny(x)]` on by default.
LEVEL = re.compile(r"(?<![\w-])(-D|-F|-W|-A|--force-warn)\s|#\[(deny|forbid|warn|allow|expect)\(")
FLAGS = {"-D": "deny", "-F": "forbid", "-W": "warn", "-A": "allow", "--force-warn": "warn"}
DEFINED_HERE = "the lint level is defined here"
NOTE_SHAPES = ("implied by", "requested on the command line", "on by default")


def level_of(diagnostic: dict) -> str | None:
    """The lint level the diagnostic's level note gives, read before the cap; None when it carries no note this tool can read."""
    for child in diagnostic.get("children") or []:
        if not isinstance(child, dict) or child.get("level") != "note":
            continue
        note = str(child.get("message", ""))
        if note == DEFINED_HERE:
            texts = [str(part.get("text", "")) for span in child.get("spans") or [] if isinstance(span, dict)
                     for part in span.get("text") or [] if isinstance(part, dict)]
            note = " ".join(texts)
        elif not any(shape in note for shape in NOTE_SHAPES):
            continue
        found = LEVEL.search(note)
        if found:
            return FLAGS[found.group(1)] if found.group(1) else found.group(2)
    return None


def in_workspace(diagnostic: dict) -> bool:
    """Whether the diagnostic's primary span is a file of the workspace: a relative path, not a toolchain source."""
    spans = [span for span in diagnostic.get("spans") or [] if isinstance(span, dict) and span.get("is_primary")]
    return bool(spans) and all(not str(span.get("file_name", "")).startswith(("/", "..")) for span in spans)


def forward(messages: TextIO, cargo_json: TextIO, diagnostics: TextIO) -> None:
    """Pass every Cargo message through but the lint diagnostics whose package has not denied their lint (or outside the
    workspace), rendering compiler errors on the separate diagnostic stream."""
    seen: dict[tuple[object, str], set[str]] = {}
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
        if not isinstance(code, str):
            continue
        levels = seen.setdefault((message.get("package_id"), code), set())
        level = level_of(diagnostic)
        if level is not None:
            levels.add(level)
        if not in_workspace(diagnostic):
            continue
        if not levels:
            cargo_json.write(line)
            diagnostics.write(f"cargo_diagnostics.py: forwarded {code}, whose lint level the compiler's notes do not give\n")
            diagnostics.flush()
        elif levels & {"deny", "forbid"}:
            cargo_json.write(line)
    cargo_json.flush()


def self_test() -> None:
    """Hold the stream filter to one rendered error, a dropped warning that is no lint, and an unchanged rest of the stream."""
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
        {"reason": "compiler-message", "message": {"level": "warning", "message": "1 warning emitted"}}
    ) + "\n"
    other = json.dumps({"reason": "build-finished", "success": False}) + "\n"
    source = StringIO(error + warning + other)
    forwarded = StringIO()
    rendered = StringIO()
    forward(source, forwarded, rendered)
    assert forwarded.getvalue() == error + other
    assert rendered.getvalue() == "error[E0001]: synthetic failure\n"


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    elif sys.argv[1:]:
        raise SystemExit("usage: cargo_diagnostics.py [--self-test]")
    else:
        forward(sys.stdin, sys.stdout, sys.stderr)
