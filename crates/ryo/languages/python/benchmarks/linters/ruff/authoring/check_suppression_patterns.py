r"""Reject double-escaped regex classes in literal Ruff suppression patterns.

RyoQL passes ``\s`` through to the regex engine as one backslash plus ``s``.
Writing ``\\s`` in a query string instead matches a literal backslash and
breaks common ``# noqa`` patterns. This guard is for source authoring, before
the benchmark worker runs.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


LITERAL_PATTERN = re.compile(r'\b(inline-pattern|range-pattern)\s*=\s*"((?:\\.|[^"\\])*)"')
REGEX_CLASSES = "sSdDwWbBpP"


def violations(source: str) -> list[str]:
    problems = []
    for match in LITERAL_PATTERN.finditer(source):
        field, value = match.groups()
        for name in REGEX_CLASSES:
            if "\\\\" + name in value:
                line = source.count("\n", 0, match.start()) + 1
                problems.append(f"line {line}: {field} double-escapes \\{name}")
    return problems


def self_test() -> None:
    assert violations(r'inline-pattern="^\s+"') == []
    assert violations(r'range-pattern="(?:^|,)RUF033(?:,|$)"') == []
    assert violations(r'inline-pattern="^\\s+"') == [
        r"line 1: inline-pattern double-escapes \s"
    ]
    assert violations(r'range-pattern="\\bRUF033\\b"') == [
        r"line 1: range-pattern double-escapes \b"
    ]


def main(args: list[str]) -> int:
    if args == ["--self-test"]:
        self_test()
        print("suppression pattern guard: self-test passed")
        return 0
    if not args:
        print("usage: check_suppression_patterns.py QUERY.ryoql [...]", file=sys.stderr)
        return 2
    failed = False
    for raw in args:
        path = Path(raw)
        for problem in violations(path.read_text()):
            print(f"{path}: {problem}", file=sys.stderr)
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
