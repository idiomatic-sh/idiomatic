"""The public inventory stays concise without changing its complete result set."""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import unittest

from authoring import generate_rule_inventory as renderer


class InventoryRendererTests(unittest.TestCase):
    def test_failure_categories_do_not_embed_verifier_payloads(self):
        cases = [
            ("missing implementation", "Missing implementation"),
            ("rule registration: missing settings", "Registration failed"),
            ("verifier timeout after 120 seconds", "Timed out"),
            ('candidate failed: expected a let in query source at byte 0', "Query rejected"),
            ("verifier exited 1: 38 cases checked; failures:\n" + "BenchmarkFinding " * 10000, "Fixture mismatch"),
            ("recorded PASS has changed or missing current source/cases", "Stale source proof"),
            ("interrupted before execution", "Not run"),
        ]
        for reason, expected in cases:
            with self.subTest(expected=expected):
                self.assertEqual(renderer.failure_category(reason), expected)

    def test_snapshot_has_every_pinned_id_and_receipt_links_without_changing_receipts(self):
        receipt = renderer.LATEST_FULL_RESULT
        before = hashlib.sha256(receipt.read_bytes()).hexdigest()
        rendered = renderer.render_full_inventory(receipt)
        self.assertEqual(rendered, (renderer.RUFF / "RULES.md").read_text())
        expected = tomllib.loads((renderer.RUFF / "benchmark.toml").read_text())["defaults"]
        rows = re.findall(r"^\| ([A-Z]+[0-9]+) \| (PASS|FAIL) \| (.*) \|$", rendered, re.MULTILINE)
        self.assertEqual([rule for rule, _, _ in rows], expected)
        self.assertEqual(len(rows), 413)
        for _, status, reason in rows:
            if status == "FAIL":
                self.assertLess(len(reason), 140)
                target = re.fullmatch(r"\[[^]]+\]\((results/ci-80be-full-inventory-2026-11-03/summary\.json)\)", reason)
                self.assertIsNotNone(target)
                self.assertTrue((renderer.RUFF / target.group(1)).is_file())
                self.assertNotIn("BenchmarkFinding", reason)
                self.assertNotIn("expected-vs-candidate", reason)
        self.assertEqual(hashlib.sha256(receipt.read_bytes()).hexdigest(), before)
        self.assertIn("expected-vs-candidate", receipt.read_text())

    def test_recovery_keeps_the_original_full_run_and_uses_executed_source_bound_proof(self):
        receipt = renderer.LATEST_FULL_RESULT
        original = json.loads(receipt.read_text())
        self.assertEqual(original["counts"], {"PASS": 383, "FAIL": 30})
        previous = {row["rule_id"]: row for row in original["rules"]}
        self.assertEqual(previous["PLE0115"]["status"], "FAIL")
        self.assertEqual(previous["PLE0117"]["status"], "FAIL")
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous,
                             receipt.parent / "ple0115-ple0117-target-supplement")
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("PLE0115", 7, 2, 5), ("PLE0117", 26, 22, 4)])
        rendered = renderer.render_full_inventory(receipt)
        self.assertIn("**385 PASS / 28 FAIL / 413 pinned defaults**", rendered)
        self.assertIn("no combined replay is claimed", rendered)
        self.assertEqual(json.loads(receipt.read_text()), original)
