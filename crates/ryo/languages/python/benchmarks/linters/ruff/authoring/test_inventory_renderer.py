"""The public inventory stays concise without changing its complete result set."""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import tarfile
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
        self.assertIn("**397 PASS / 16 FAIL / 413 pinned defaults**", rendered)
        self.assertIn("no combined replay is claimed", rendered)
        self.assertEqual(json.loads(receipt.read_text()), original)

    def test_ruf041_proof_preserves_original_sources_and_scope_controls(self):
        from authoring.target_supplement import verified_rows
        receipt = renderer.LATEST_FULL_RESULT
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        target = receipt.parent / "ruf041-target-supplement"
        rows = verified_rows(renderer.RUFF, receipt.parent, previous, target)
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("RUF041", 43, 36, 7)])
        controls = json.loads((target / "controls.json").read_text())["controls"]
        self.assertEqual([(item["target"], item["case_count"]) for item in controls],
                         [("py310", 43), ("py312", 43)])
        cases = tomllib.loads((renderer.RUFF / "rules/RUF041/cases.toml").read_text())["cases"]
        positives = [case for case in cases if "unrelated-generic-literal-positive" in case["name"]]
        self.assertEqual(len(positives), 2)
        self.assertTrue(all(any(finding["rule"] == "RUF041" for finding in case["expected"])
                            for case in positives))
        with tarfile.open(target / "raw-receipts.tar.gz", "r:gz") as archive:
            source = archive.extractfile("history/RUF041/profiles/RUF041/repro/rules/RUF041/cases.toml")
            original = tomllib.loads(source.read().decode())["cases"]
        self.assertEqual([case["files"] for case in cases[:41]],
                         [case["files"] for case in original])

    def test_tc004_proof_retains_full_fixtures_and_both_target_controls(self):
        from authoring.target_supplement import verified_rows
        receipt = renderer.LATEST_FULL_RESULT
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        target = receipt.parent / "tc004-target-supplement"
        rows = verified_rows(renderer.RUFF, receipt.parent, previous, target)
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("TC004", 41, 39, 2)])
        controls = json.loads((target / "controls.json").read_text())["controls"]
        self.assertEqual([(item["target"], item["case_count"]) for item in controls],
                         [("py312", 41), ("py310", 41)])
        cases = tomllib.loads((renderer.RUFF / "rules/TC004/cases.toml").read_text())["cases"]
        self.assertEqual(sum(case["name"].startswith("upstream-full-") for case in cases), 17)
        self.assertEqual({case["name"] for case in cases if "reference-discrepancy" in case},
                         {"runtime-fallback", "shadowed-true"})

    def test_tc007_proof_preserves_scope_negatives_and_exact_differences(self):
        from authoring.target_supplement import verified_rows
        receipt = renderer.LATEST_FULL_RESULT
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        rows = verified_rows(renderer.RUFF, receipt.parent, previous,
                             receipt.parent / "tc007-target-supplement")
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("TC007", 32, 30, 2)])
        cases = tomllib.loads((renderer.RUFF / "rules/TC007/cases.toml").read_text())["cases"]
        by_name = {case["name"]: case for case in cases}
        for name in ("annotation-only-name", "forward-runtime-class",
                     "function-local-shadow", "function-local-builtin-shadow"):
            self.assertEqual(by_name[name]["expected"], [])
            self.assertNotIn("reference-discrepancy", by_name[name])
        self.assertTrue(by_name["valid-class-scope-unavailable"]["expected"])

    def test_natural_order_proof_retains_both_complete_target_controls(self):
        receipt = renderer.LATEST_FULL_RESULT
        directory = receipt.parent / "ruf022-ruf023-target-supplement"
        proof = json.loads((directory / "proof.json").read_text())
        self.assertEqual(proof["requirements"], {"RUF022": ["py310", "py37"], "RUF023": ["py310", "py37"]})
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous, directory)
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("RUF022", 33, 33, 0), ("RUF023", 31, 31, 0)])

    def test_stub_syntax_proof_keeps_all_original_cases_and_only_two_exact_discrepancies(self):
        receipt = renderer.LATEST_FULL_RESULT
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous,
                             receipt.parent / "pyi019-pyi034-target-supplement")
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("PYI019", 139, 138, 1), ("PYI034", 72, 71, 1)])

    def test_binding_validity_proof_preserves_selected_findings_and_complete_fixtures(self):
        receipt = renderer.LATEST_FULL_RESULT
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous,
                             receipt.parent / "f811-f841-target-supplement")
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("F811", 120, 111, 9), ("F841", 106, 77, 29)])

    def test_pep695_proof_checks_both_targets_without_dropping_unsupported_sources(self):
        receipt = renderer.LATEST_FULL_RESULT
        directory = receipt.parent / "up046-up047-target-supplement"
        proof = json.loads((directory / "proof.json").read_text())
        self.assertEqual(proof["requirements"], {"UP046": ["py312", "py310"], "UP047": ["py312", "py310"]})
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous, directory)
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("UP046", 14, 14, 0), ("UP047", 16, 16, 0)])
        counter_syntax = 0
        for rule in ["UP046", "UP047"]:
            default = tomllib.loads((renderer.RUFF / "rules" / rule / "cases.toml").read_text())["cases"]
            counter = tomllib.loads((renderer.RUFF / "profiles" / (rule + "-py310") / "cases.toml").read_text())["cases"]
            self.assertEqual({case["name"]: case["files"] for case in default},
                             {case["name"]: case["files"] for case in counter})
            self.assertFalse(any(finding["rule"] == rule for case in counter for finding in case["expected"]))
            counter_syntax += sum(finding["rule"] == "invalid-syntax" for case in counter for finding in case["expected"])
        self.assertEqual(counter_syntax, 11)

    def test_never_union_proof_preserves_all_sources_and_documents_scope_policy(self):
        receipt = renderer.LATEST_FULL_RESULT
        directory = receipt.parent / "ruf020-target-supplement"
        proof = json.loads((directory / "proof.json").read_text())
        self.assertEqual(proof["requirements"], {"RUF020": ["py310", "py312"]})
        previous = {row["rule_id"]: row for row in json.loads(receipt.read_text())["rules"]}
        from authoring.target_supplement import verified_rows
        rows = verified_rows(renderer.RUFF, receipt.parent, previous, directory)
        self.assertEqual([(row["rule_id"], row["case_count"], row["raw_exact"], row["documented_discrepancies"])
                          for row in rows], [("RUF020", 42, 33, 9)])
        default = tomllib.loads((renderer.RUFF / "rules/RUF020/cases.toml").read_text())["cases"]
        counter = tomllib.loads((renderer.RUFF / "profiles/RUF020-py312/cases.toml").read_text())["cases"]
        self.assertEqual({case["name"]: case["files"] for case in default},
                         {case["name"]: case["files"] for case in counter})
        for cases in [default, counter]:
            documented = [case for case in cases if "reference-discrepancy" in case]
            self.assertEqual(len(documented), 9)
            scope = next(case for case in documented if case["name"] == "ruf020-forward-module-value-overrides-local")
            self.assertIn("scope-policy difference", scope["reference-discrepancy"]["reason"])
            self.assertEqual(scope["reference-discrepancy"]["expected"], [])
            self.assertEqual(scope["expected"], [{"rule": "RUF020", "file": "case.py", "start": 97, "end": 102}])
        self.assertEqual(sum(finding["rule"] == "invalid-syntax" for case in default for finding in case["expected"]), 15)
        self.assertFalse(any(finding["rule"] == "invalid-syntax" for case in counter for finding in case["expected"]))
