"""Ruff parity comparison tests; neither ryo nor Ruff is run."""
import json
from pathlib import Path
import tempfile
import unittest

from authoring.ruff_parity import Positions, accepted_problems, compare, corpus_digest, load_accepted, rows_text, ruff_command

SOURCE = "x = 'é'\r\nfor items in items:\n    pass\n"


def request(root):
    return {"root": str(root), "files": ["case.py"], "selected_rules": ["B020", "F401"], "parser_configuration": "python-target=3.10",
            "context_manifest": {"boundary": str(root), "logical_root": ".", "source_files": {"case.py": "0" * 64}, "auxiliary_files": {}, "missing": []}}


def accepted_text(request, *differences):
    return json.dumps({"description": "d", "corpus": corpus_digest(request), "differences": list(differences)})


def finding(kind, line, column):
    return {"kind": kind, "file": "case.py", "line": line, "column": column}


def ruff_finding(root, code, row, column, end_row, end_column):
    return {"filename": str(Path(root) / "case.py"), "code": code, "location": {"row": row, "column": column},
            "end_location": {"row": end_row, "column": end_column}, "message": "m"}


class RuffParityTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        (self.root / "case.py").write_bytes(SOURCE.encode("utf-8"))
        self.loop = SOURCE.encode("utf-8").index(b"items")

    def tearDown(self):
        self.directory.cleanup()

    def test_matching_findings_are_identical_across_unicode_and_crlf_lines(self):
        ryo = {"diagnostics": [{"rule": "B020", "file": "case.py", "start": self.loop, "end": self.loop + 5}], "skipped": []}
        rows, counts, _ = compare(ryo, [ruff_finding(self.root, "B020", 2, 5, 2, 10)], request(self.root), Positions(self.root))
        self.assertEqual([(row["rule"], row["class"], row["matched"], row["end_differs"]) for row in rows], [("B020", "identical", 1, 0), ("F401", "identical", 0, 0)])
        self.assertEqual(counts["totals"]["matched"], 1)

    def test_unmatched_findings_are_missing_or_extra_and_other_rules_are_outside(self):
        ryo = {"diagnostics": [{"rule": "B020", "file": "case.py", "start": self.loop, "end": self.loop + 5},
                               {"rule": "invalid-syntax", "file": "case.py", "start": 0, "end": 1}],
               "skipped": [{"path": "case.py", "reason": "syntax-rejected"}]}
        rows, counts, differing = compare(ryo, [ruff_finding(self.root, "F401", 1, 5, 1, 8), ruff_finding(self.root, "E999", 1, 1, 1, 2)], request(self.root), Positions(self.root))
        self.assertEqual([(row["rule"], row["class"], row["missing"], row["extra"]) for row in rows], [("B020", "ryo-extra", 0, 1), ("F401", "ryo-missing", 1, 0)])
        self.assertEqual(rows[1]["missing_samples"], [{"file": "case.py", "line": 1, "column": 5, "end": [1, 8], "ruff_message": "m"}])
        self.assertEqual(counts["outside_selection"], {"ryo": {"invalid-syntax": 1}, "ruff": {"E999": 1}})
        self.assertEqual(counts["ryo_skipped"], {"rows": 1, "reasons": {"syntax-rejected": 1}, "files": ["case.py"]})
        self.assertEqual(counts["rules_by_class"], {"ryo-missing": ["F401"], "ryo-extra": ["B020"], "both": []})
        self.assertEqual(differing, {"B020": [("extra", "case.py", 2, 5)], "F401": [("missing", "case.py", 1, 5)]})

    def test_end_position_difference_is_counted_on_a_matched_pair(self):
        ryo = {"diagnostics": [{"rule": "B020", "file": "case.py", "start": self.loop, "end": self.loop + 4}], "skipped": []}
        rows, _, differing = compare(ryo, [ruff_finding(self.root, "B020", 2, 5, 2, 10)], request(self.root), Positions(self.root))
        self.assertEqual((rows[0]["matched"], rows[0]["end_differs"]), (1, 1))
        self.assertEqual(accepted_problems(rows, differing, {}), ["B020: 1 end positions differ"])

    def test_ruff_position_beyond_the_line_is_refused(self):
        with self.assertRaises(ValueError):
            compare({"diagnostics": [], "skipped": []}, [ruff_finding(self.root, "B020", 1, 40, 1, 41)], request(self.root), Positions(self.root))

    def rows(self, differing):
        return [{"rule": rule, "end_differs": 0} for rule in differing], differing

    def test_only_exactly_pinned_findings_pass(self):
        rows, differing = self.rows({"B020": [("extra", "case.py", 2, 5)], "F401": []})
        exact = load_accepted(accepted_text(request(self.root), {"rule": "B020", "reason": "r", "findings": [finding("extra", 2, 5)]}), request(self.root))
        self.assertEqual(accepted_problems(rows, differing, exact), [])
        self.assertEqual(accepted_problems(rows, differing, {}), ["B020: extra case.py:2:5 not accepted"])

    def test_a_same_count_difference_at_another_location_fails(self):
        rows, differing = self.rows({"B020": [("extra", "case.py", 3, 1)], "F401": []})
        swapped = load_accepted(accepted_text(request(self.root), {"rule": "B020", "reason": "r", "findings": [finding("extra", 2, 5)]}), request(self.root))
        self.assertEqual(accepted_problems(rows, differing, swapped), ["B020: extra case.py:3:1 not accepted", "B020: accepted extra case.py:2:5 no longer occurs"])

    def test_stale_and_unselected_acceptances_fail(self):
        rows, differing = self.rows({"B020": [], "F401": []})
        stale = load_accepted(accepted_text(request(self.root), {"rule": "F401", "reason": "r", "findings": [finding("missing", 1, 5)]},
                                            {"rule": "UP032", "reason": "r", "findings": [finding("missing", 1, 1)]}), request(self.root))
        self.assertEqual(accepted_problems(rows, differing, stale), ["F401: accepted missing case.py:1:5 no longer occurs", "UP032: accepted but not selected"])

    def test_a_rule_named_twice_is_refused(self):
        twice = accepted_text(request(self.root), {"rule": "B020", "reason": "r", "findings": [finding("extra", 9, 9)]},
                              {"rule": "B020", "reason": "r", "findings": [finding("extra", 2, 5)]})
        with self.assertRaisesRegex(ValueError, "B020 twice"):
            load_accepted(twice, request(self.root))

    def test_acceptances_for_another_corpus_are_refused(self):
        other = {**request(self.root), "files": ["case.py", "other.py"]}
        text = accepted_text(other, {"rule": "B020", "reason": "r", "findings": [finding("extra", 2, 5)]})
        with self.assertRaisesRegex(ValueError, "corpus"):
            load_accepted(text, request(self.root))
        moved = {**request(self.root), "root": "/elsewhere", "context_manifest": {**request(self.root)["context_manifest"], "boundary": "/elsewhere"}}
        self.assertEqual(corpus_digest(moved), corpus_digest(request(self.root)))

    def test_rule_rows_keep_their_order_with_identical_runs_on_one_line(self):
        rows = [{"rule": rule, "class": kind, "end_differs": 0} for rule, kind in (("A", "identical"), ("B", "identical"), ("C", "ryo-extra"), ("D", "identical"))]
        text = rows_text(rows)
        self.assertEqual(json.loads(text), rows)
        self.assertEqual(len(text.splitlines()), 5)

    def test_ruff_runs_isolated_over_the_request_target_rules_and_files(self):
        self.assertEqual(ruff_command("ruff", request(self.root)), ["ruff", "check", "--isolated", "--no-cache", "--output-format", "json", "--select", "B020,F401", "--target-version", "py310", "case.py"])
        with self.assertRaises(ValueError):
            ruff_command("ruff", {**request(self.root), "parser_configuration": "python-target=2.7"})


if __name__ == "__main__":
    unittest.main()
