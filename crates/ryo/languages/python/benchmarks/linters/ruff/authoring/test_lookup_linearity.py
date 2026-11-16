"""The name lookup of ordinary reads costs time proportional to the file, not to its square.

A file of N pairs `x = i; print(x)` has N bindings of one name and N reads of it. A lookup that joins every read to every
earlier binding grows 3.5 to 6 times per doubling and holds gigabytes; one ordered pass doubles. The F401 profile runs
the lookup for every read, so its worker time at 1,000, 2,000 and 4,000 pairs must grow by at most 2.2 each doubling
(CPU seconds of the worker, the best of two runs, so a busy machine does not decide it).
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

from verify_profiles import RUFF_DIR, profile_modules, write_staged_profile

DOUBLING_LIMIT = 2.2
SIZES = (1000, 2000, 4000)
CONTROL = "import os\n"


def cases(pairs: int) -> str:
    body = "".join(f"x = {index}\nprint(x)\n" for index in range(pairs))
    return (
        '[[cases]]\nname="control"\ncomparison="full"\n'
        f"files={{\"case.py\"={json.dumps(CONTROL)}}}\n"
        'expected=[{rule="F401", file="case.py", start=7, end=9}]\n\n'
        '[[cases]]\nname="pairs"\ncomparison="full"\n'
        f"files={{\"case.py\"={json.dumps(body)}}}\nexpected=[]\n"
    )


class LookupLinearityTest(unittest.TestCase):
    def stage(self, root: Path, pairs: int) -> None:
        source = RUFF_DIR / "profiles/F401/benchmark.toml"
        profile = tomllib.loads(source.read_text(encoding="utf-8"))
        registry = profile_modules(profile, RUFF_DIR)
        for relative in registry.values():
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(RUFF_DIR / relative, destination)
        write_staged_profile(
            source, root / "benchmark.toml", registry,
            "python-target=3.10", {"F401": "accepted-syntax"}, module_root=root,
        )
        rule = root / "rules/F401"
        rule.mkdir(parents=True)
        shutil.copyfile(RUFF_DIR / "rules/F401/query.ryoql", rule / "query.ryoql")
        (rule / "cases.toml").write_text(cases(pairs), encoding="utf-8")

    def worker_seconds(self, worker: str, ruff: str, pairs: int) -> float:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.stage(root, pairs)
            before = os.times()
            result = subprocess.run([worker, "verify-cases", ruff, str(root)], capture_output=True, text=True, cwd=RUFF_DIR)
            after = os.times()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return (after.children_user - before.children_user) + (after.children_system - before.children_system)

    def test_worker_time_doubles_with_the_file(self) -> None:
        # The sealed worker and the pinned Ruff are this test's input, named by the environment; an absent one fails it.
        named = {name: os.environ.get(name) for name in ("RYO_PREFLIGHT_WORKER", "RYO_PREFLIGHT_RUFF")}
        absent = [name for name, value in named.items() if not value or not Path(value).is_file()]
        self.assertEqual(absent, [], "the sealed worker and pinned Ruff must be named and present")
        worker, ruff = named["RYO_PREFLIGHT_WORKER"], named["RYO_PREFLIGHT_RUFF"]
        seconds = [min(self.worker_seconds(worker, ruff, pairs) for _ in range(2)) for pairs in SIZES]
        ratios = [later / earlier for earlier, later in zip(seconds, seconds[1:])]
        self.assertTrue(
            all(ratio <= DOUBLING_LIMIT for ratio in ratios),
            f"worker seconds at {SIZES} pairs are {seconds}; each doubling must cost at most {DOUBLING_LIMIT}x, got {ratios}",
        )


if __name__ == "__main__":
    unittest.main()
