"""The captured Ubuntu collector artifact remains selectable as a release binary."""

from __future__ import annotations

import json
from pathlib import Path
import unittest

from seal_cargo_artifact import select_artifact


CAPTURE = Path(__file__).parent / "fixtures" / "collector-artifact.jsonl"
BENCH_CAPTURE = Path(__file__).parent / "fixtures" / "ruff-bench-artifact.jsonl"
SOURCE_SUFFIX = "/crates/benchmark/src/main.rs"


class CargoArtifactTest(unittest.TestCase):
    def test_release_binary_has_test_eligible_target(self) -> None:
        line = CAPTURE.read_text().strip()
        captured = json.loads(line)
        self.assertIs(captured["target"]["test"], True)
        self.assertIs(captured["profile"]["test"], False)
        selected = select_artifact([line], "ryo-benchmark-collector", "bin", SOURCE_SUFFIX,
                                   False)
        self.assertEqual(str(selected), captured["executable"])

    def test_bench_no_run_has_test_profile_and_non_test_target(self) -> None:
        line = BENCH_CAPTURE.read_text().strip()
        captured = json.loads(line)
        self.assertIs(captured["target"]["test"], False)
        self.assertIs(captured["profile"]["test"], True)
        selected = select_artifact([line], "ruff", "bench",
                                   "/languages/python/benchmarks/linters/ruff/bench.rs", True)
        self.assertEqual(str(selected), captured["executable"])

    def test_test_profile_is_refused(self) -> None:
        captured = json.loads(CAPTURE.read_text())
        captured["profile"]["test"] = True
        with self.assertRaisesRegex(ValueError, "found 0"):
            select_artifact([json.dumps(captured)], "ryo-benchmark-collector", "bin",
                            SOURCE_SUFFIX, False)


if __name__ == "__main__":
    unittest.main()
