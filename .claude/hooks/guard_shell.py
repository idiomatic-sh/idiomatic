"""PreToolUse hook for Bash: blocks the shell habits that have cost this repository work."""

import json
import re
import shlex
import sys

LOCK = "/workspaces/.cargo-build.lock"
TARGET = "/workspaces/idiomatic-benchmark-guards/target"
FIXTURE_FIX = "cli/tests/replay/imports.rs"
CARGO_BUILDS = {"build", "test", "bench", "clippy", "run", "check", "doc"}
SPLIT = re.compile(r"\n|;|&&|\|\||\||(?<![>&])&(?![>&])")
COMMENT = re.compile(r"(^|\s)#.*$", re.MULTILINE)


def tokens(segment):
    try:
        found = shlex.split(segment)
    except ValueError:
        found = segment.split()
    while found and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", found[0]):
        found = found[1:]
    return found


def stash(words):
    if len(words) < 2 or words[0] != "git":
        return None
    rest = [w for w in words[1:] if not w.startswith("-")] if words[1].startswith("-") else words[1:]
    if not rest or rest[0] != "stash":
        return None
    if len(rest) == 1 or rest[1] == "pop":
        return "bare git stash/stash pop shares one stack across worktrees; commit, or use git stash push -u -m"
    return None


def removal(words):
    if not words or words[0] != "rm":
        return None
    flags = [w for w in words[1:] if w.startswith("-")]
    if not any(re.search(r"[rR]", f) or f == "--recursive" for f in flags):
        return None
    for target in (w for w in words[1:] if not w.startswith("-")):
        if "*" in target or "?" in target:
            return "recursive rm with a glob; delete only exact paths you created"
    return None


def cargo_index(words):
    """Index of `cargo` when the segment runs it directly, behind an optional flock."""
    if not words:
        return None
    if words[0] == "flock":
        for i, w in enumerate(words):
            if w == "cargo":
                return i
        return None
    return 0 if words[0] == "cargo" else None


def cargo_rules(words, before):
    at = cargo_index(words)
    if at is None:
        return None
    rest = [w for w in words[at + 1:] if not w.startswith("+")]
    if "--target-dir" in rest or any(w.startswith("--target-dir=") for w in rest):
        for i, w in enumerate(rest):
            value = w.split("=", 1)[1] if w.startswith("--target-dir=") else (rest[i + 1] if w == "--target-dir" and i + 1 < len(rest) else None)
            if (w == "--target-dir" or w.startswith("--target-dir=")) and value != TARGET:
                return "cargo --target-dir must be " + TARGET
    sub = next((w for w in rest if not w.startswith("-")), "")
    if sub in CARGO_BUILDS and LOCK not in " ".join(words[:at]) and LOCK not in before:
        return "cargo " + sub + " must run as: flock " + LOCK + " cargo " + sub
    return None


def tm_fix(words, command):
    if not words or words[0].rsplit("/", 1)[-1] != "tm":
        return None
    if "check" in words and "--fix" in words and FIXTURE_FIX not in command:
        return "tm check --fix is allowed only for the fixture " + FIXTURE_FIX
    return None


def reason(command):
    clean = COMMENT.sub("", command)
    if re.search(r"\b(while|until|for)\b", clean) and re.search(r"\b(pgrep|pkill)\b[^;&|\n]*\s-[A-Za-z]*f", clean):
        return "self-matching wait loop (pgrep/pkill -f in a loop); use pid files or pgrep -x"
    before = ""
    for segment in SPLIT.split(clean):
        words = tokens(segment)
        for found in (stash(words), removal(words), cargo_rules(words, before), tm_fix(words, clean)):
            if found:
                return found
        before += segment + " "
    return None


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    command = (data.get("tool_input") or {}).get("command") or ""
    found = reason(command)
    if found:
        print("Blocked: " + found, file=sys.stderr)
        return 2
    return 0


sys.exit(main())
