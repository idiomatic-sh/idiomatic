#!/usr/bin/env python3
"""check-quotes.py [CONTRACTS_DIR]: every `  @REV:PATH:LINE` quote in every sheet.md and api.txt must match that line of
the code in /workspaces/idiomatic-public-benchmark at that revision. Prints one line per failure; exit 1 on any."""
import os, re, subprocess, sys
ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = "/workspaces/idiomatic-public-benchmark"
QUOTE = re.compile(r"^(?P<text>.*?\S)\s{2,}@(?P<rev>[0-9a-f]{7,40}):(?P<path>[^\s:]+):(?P<line>\d+)\s*(?:\[[^\]]*\])?\s*$")
cache, fails, checked = {}, 0, 0
def lines(rev, path):
    key = (rev, path)
    if key not in cache:
        out = subprocess.run(["git", "-C", REPO, "show", f"{rev}:{path}"], capture_output=True, text=True)
        cache[key] = out.stdout.split("\n") if out.returncode == 0 else None
    return cache[key]
norm = lambda s: re.sub(r"\s+", " ", s.strip().strip("`").strip())
for dirpath, _, files in os.walk(ROOT):
    if "/inputs" in dirpath or "/briefs" in dirpath: continue
    for name in files:
        if name not in ("sheet.md", "api.txt"): continue
        f = os.path.join(dirpath, name)
        for n, raw in enumerate(open(f, errors="replace"), 1):
            m = QUOTE.match(raw.rstrip("\n"))
            if not m: continue
            text = norm(re.sub(r"^\s*([-*>]|\d+\.)\s+", "", m["text"]))
            src = lines(m["rev"], m["path"]); checked += 1
            rel = os.path.relpath(f, ROOT)
            if src is None:
                print(f"{rel}:{n}: no such file {m['rev']}:{m['path']}"); fails += 1; continue
            i = int(m["line"]) - 1
            if i >= len(src):
                print(f"{rel}:{n}: line {m['line']} past end of {m['path']}"); fails += 1; continue
            code = norm(src[i])
            if not (text in code or code in text or (len(code) > 8 and text.startswith(code[: max(8, len(code) // 2)]))):
                print(f"{rel}:{n}: quote does not match {m['rev']}:{m['path']}:{m['line']}\n    quoted: {text[:140]}\n    code:   {code[:140]}"); fails += 1
print(f"check-quotes: {checked} quotes checked, {fails} failures", file=sys.stderr)
sys.exit(1 if fails else 0)
