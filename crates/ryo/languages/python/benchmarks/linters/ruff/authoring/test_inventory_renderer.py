"""The public inventory stays concise without changing its complete result set."""
import hashlib
import json
from pathlib import Path
import re
import tempfile
import tomllib
import unittest

from authoring import generate_rule_inventory as renderer
from authoring import inventory_sources


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

    def test_render_lists_every_pinned_id_with_short_receipt_links_and_demotes_stale_proof(self):
        receipt = renderer.LATEST_FULL_RESULT
        before = hashlib.sha256(receipt.read_bytes()).hexdigest()
        summary = json.loads(receipt.read_text())
        expected = tomllib.loads((renderer.RUFF / "benchmark.toml").read_text())["defaults"]
        base_modules = tomllib.loads((renderer.RUFF / "benchmark.toml").read_text()).get("query-modules", {})
        # Re-seal every row's source identities from the live tree, so only the planted rows differ.
        for row in summary["rules"]:
            instance = row["instance"]
            query, _label = inventory_sources.source(row["rule_id"])
            settings = inventory_sources.REPOSITORY / instance["settings_source"]
            registry = {**base_modules, **tomllib.loads(settings.read_text()).get("query-modules", {})}
            instance.update(
                query_sha256=inventory_sources.sha(query),
                cases_sha256=inventory_sources.sha(renderer.RUFF / "rules" / row["rule_id"] / "cases.toml"),
                settings_source_sha256=inventory_sources.sha(settings),
                module_sha256={logical: inventory_sources.sha(renderer.RUFF / registry[logical])
                               for logical in instance["module_sha256"] if logical in registry})
        failed, stale = summary["rules"][0], summary["rules"][1]
        failed.pop("failure_receipt", None)
        failed.update(status="FAIL", reason="verifier exited 1: 38 cases checked; failures:\n" + "BenchmarkFinding " * 10000)
        stale["instance"]["query_sha256"] = "0" * 64
        with tempfile.TemporaryDirectory(prefix="ryo-inventory-render-") as temporary:
            fixture = Path(temporary) / "summary.json"
            fixture.write_text(json.dumps(summary))
            rendered = renderer.render_full_inventory(fixture)
            rows = re.findall(r"^\| ([A-Z]+[0-9]+) \| (PASS|FAIL) \| (.*) \|$", rendered, re.MULTILINE)
            self.assertEqual([rule for rule, _, _ in rows], expected)
            self.assertEqual(sum(status == "PASS" for _, status, _ in rows), 411)
            by_rule = {rule: (status, reason) for rule, status, reason in rows}
            self.assertEqual(by_rule[failed["rule_id"]], ("FAIL", f"[Fixture mismatch]({fixture.resolve().as_posix()})"))
            self.assertEqual(by_rule[stale["rule_id"]], ("FAIL", f"[Stale source proof]({fixture.resolve().as_posix()})"))
            for _, status, reason in rows:
                self.assertLess(len(reason), 140)
                self.assertNotIn("BenchmarkFinding", reason)
        self.assertEqual(hashlib.sha256(receipt.read_bytes()).hexdigest(), before)
