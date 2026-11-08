"""Ruff parity comparison tests; neither ryo nor Ruff is run."""
from pathlib import Path
import tempfile
import unittest

import json

from authoring.ruff_parity import Positions, accepted_problems, compare, rows_text, ruff_command

SOURCE = "x = 'é'\r\nfor items in items:\n    pass\n"


def request(root):
    return {"root": str(root), "files": ["case.py"], "selected_rules": ["B020", "F401"], "parser_configuration": "python-target=3.10"}


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
        rows, counts = compare(ryo, [ruff_finding(self.root, "B020", 2, 5, 2, 10)], request(self.root), Positions(self.root))
        self.assertEqual([(row["rule"], row["class"], row["matched"], row["end_differs"]) for row in rows], [("B020", "identical", 1, 0), ("F401", "identical", 0, 0)])
        self.assertEqual(counts["totals"]["matched"], 1)

    def test_unmatched_findings_are_missing_or_extra_and_other_rules_are_outside(self):
        ryo = {"diagnostics": [{"rule": "B020", "file": "case.py", "start": self.loop, "end": self.loop + 5},
                               {"rule": "invalid-syntax", "file": "case.py", "start": 0, "end": 1}],
               "skipped": [{"path": "case.py", "reason": "syntax-rejected"}]}
        rows, counts = compare(ryo, [ruff_finding(self.root, "F401", 1, 5, 1, 8), ruff_finding(self.root, "E999", 1, 1, 1, 2)], request(self.root), Positions(self.root))
        self.assertEqual([(row["rule"], row["class"], row["missing"], row["extra"]) for row in rows], [("B020", "ryo-extra", 0, 1), ("F401", "ryo-missing", 1, 0)])
        self.assertEqual(rows[1]["missing_samples"], [{"file": "case.py", "line": 1, "column": 5, "end": [1, 8], "ruff_message": "m"}])
        self.assertEqual(counts["outside_selection"], {"ryo": {"invalid-syntax": 1}, "ruff": {"E999": 1}})
        self.assertEqual(counts["ryo_skipped"], {"rows": 1, "reasons": {"syntax-rejected": 1}, "files": ["case.py"]})
        self.assertEqual(counts["rules_by_class"], {"ryo-missing": ["F401"], "ryo-extra": ["B020"], "both": []})

    def test_end_position_difference_is_counted_on_a_matched_pair(self):
        ryo = {"diagnostics": [{"rule": "B020", "file": "case.py", "start": self.loop, "end": self.loop + 4}], "skipped": []}
        rows, _ = compare(ryo, [ruff_finding(self.root, "B020", 2, 5, 2, 10)], request(self.root), Positions(self.root))
        self.assertEqual((rows[0]["matched"], rows[0]["end_differs"]), (1, 1))
        self.assertEqual(accepted_problems(rows, {"differences": []}), ["B020: 0 missing, 0 extra, 1 end positions differ; not accepted"])

    def test_ruff_position_beyond_the_line_is_refused(self):
        with self.assertRaises(ValueError):
            compare({"diagnostics": [], "skipped": []}, [ruff_finding(self.root, "B020", 1, 40, 1, 41)], request(self.root), Positions(self.root))

    def test_only_exactly_accepted_differences_pass(self):
        rows = [{"rule": "B020", "missing": 0, "extra": 3, "end_differs": 0, "differences": 3}, {"rule": "F401", "missing": 0, "extra": 0, "end_differs": 0, "differences": 0}]
        exact = {"differences": [{"rule": "B020", "missing": 0, "extra": 3, "reason": "r"}]}
        self.assertEqual(accepted_problems(rows, exact), [])
        self.assertEqual(accepted_problems(rows, {"differences": []}), ["B020: 0 missing, 3 extra, 0 end positions differ; not accepted"])
        drifted = {"differences": [{"rule": "B020", "missing": 0, "extra": 2, "reason": "r"}]}
        self.assertEqual(accepted_problems(rows, drifted), ["B020: 0 missing, 3 extra, 0 end positions differ; accepted 0 missing, 2 extra"])
        stale = {"differences": [*exact["differences"], {"rule": "F401", "missing": 1, "extra": 0, "reason": "r"}, {"rule": "UP032", "missing": 1, "extra": 0, "reason": "r"}]}
        self.assertEqual(accepted_problems(rows, stale), ["F401: 0 missing, 0 extra, 0 end positions differ; accepted 1 missing, 0 extra", "UP032: accepted but not selected"])

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
