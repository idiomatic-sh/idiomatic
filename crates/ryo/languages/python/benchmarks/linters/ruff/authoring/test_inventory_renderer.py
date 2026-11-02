"""The public inventory stays concise without changing its complete result set."""
import hashlib
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
        receipt = renderer.RESULTS / "full-inventory-current/summary.json"
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
                self.assertIn("results/full-inventory-current/summary.json", reason)
                self.assertNotIn("BenchmarkFinding", reason)
                self.assertNotIn("expected-vs-candidate", reason)
        self.assertEqual(hashlib.sha256(receipt.read_bytes()).hexdigest(), before)
        self.assertIn("expected-vs-candidate", receipt.read_text())
