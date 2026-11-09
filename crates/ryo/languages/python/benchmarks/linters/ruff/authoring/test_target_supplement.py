"""Receipt-policy tests; no rule subprocess is simulated."""
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import unittest

from authoring.target_supplement import verified_rows, verified_failed_rows

RUFF = Path(__file__).resolve().parents[1]
CURRENT = "results/full-inventory-current"
SUPPLEMENT = "up035-up036-target-supplement"
PREVIOUS = {rule: {"status": "FAIL"} for rule in ["UP035", "UP036"]}
# Every query module a control's staged profile may import: the fragments the controls bind by digest.
FRAGMENTS = sorted(str(path.relative_to(RUFF)) for path in (RUFF / "fragments").glob("*.ryoql"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class TargetEvidenceTests(unittest.TestCase):
    def fixture(self, directory):
        root = Path(directory) / "ruff"
        current = root / CURRENT
        target = current / SUPPLEMENT
        shutil.copytree(RUFF / CURRENT / SUPPLEMENT, target)
        proof = json.loads((target / "proof.json").read_text())
        controls = json.loads((target / "controls.json").read_text())
        needed = set(FRAGMENTS)
        for item in controls["controls"]:
            needed.update([item["cases_file"], item["settings_source"], f'rules/{item["rule"]}/query.ryoql'])
            evidence = (RUFF / item["settings_source"]).parent / "evidence"
            if evidence.is_dir():
                needed.update(str(path.relative_to(RUFF)) for path in evidence.rglob("*") if path.is_file())
        needed.update(CURRENT + "/" + name for name in proof["prior_summaries_sha256"])
        for relative in needed:
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(RUFF / relative, destination)
        return root, current, target, proof, controls

    def test_all_actual_archived_targets_validate(self):
        self.assertEqual([row["rule_id"] for row in verified_rows(RUFF, RUFF / CURRENT, PREVIOUS, RUFF / CURRENT / SUPPLEMENT)], ["UP035", "UP036"])

    def test_missing_countertarget_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root, current, target, proof, controls = self.fixture(directory)
            controls["controls"].pop()
            (target / "controls.json").write_text(json.dumps(controls))
            proof["controls_sha256"] = sha(target / "controls.json")
            (target / "proof.json").write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, "missing, duplicate or unknown"):
                verified_rows(root, current, PREVIOUS, target)

    def test_failed_native_case_cannot_hide_behind_rehashed_passing_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            root, current, target, proof, controls = self.fixture(directory)
            item = controls["controls"][0]
            archive, replacement = target / "raw-receipts.tar.gz", target / "replacement.tar.gz"
            with tarfile.open(archive, "r:gz") as source:
                records = [json.loads(line) for line in source.extractfile(item["case_results_path"]).read().splitlines()]
                next(row for row in records if row["side"] == "candidate")["case_passed"] = False
                payload = ("\n".join(json.dumps(row) for row in records) + "\n").encode()
                item["case_results_sha256"] = hashlib.sha256(payload).hexdigest()
                summary = json.loads(source.extractfile(item["summary_path"]).read())
                summary["rules"][0]["case_results_sha256"] = item["case_results_sha256"]
                summary_payload = json.dumps(summary).encode()
                item["summary_sha256"] = hashlib.sha256(summary_payload).hexdigest()
                changes = {item["case_results_path"]: payload, item["summary_path"]: summary_payload}
                with tarfile.open(replacement, "w:gz") as output:
                    for member in source.getmembers():
                        metadata = copy.copy(member)
                        if member.isfile():
                            data = changes.get(member.name)
                            if data is None:
                                data = source.extractfile(member).read()
                            metadata.size = len(data)
                            output.addfile(metadata, io.BytesIO(data))
                        else:
                            output.addfile(metadata)
            replacement.replace(archive)
            (target / "controls.json").write_text(json.dumps(controls))
            proof["controls_sha256"] = sha(target / "controls.json")
            proof["raw_receipts_sha256"] = sha(archive)
            (target / "proof.json").write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, "execution is incomplete or failed"):
                verified_rows(root, current, PREVIOUS, target)


class OptionalModuleAndFailureEvidenceTests(unittest.TestCase):
    def direct_fixture(self, directory):
        root = Path(directory) / "ruff"
        current = root / CURRENT
        target = current / "ple0116-target-supplement"
        shutil.copytree(RUFF / CURRENT / "ple0116-target-supplement", target)
        proof = json.loads((target / "proof.json").read_text())
        controls = json.loads((target / "controls.json").read_text())
        needed = {"rules/PLE0116/query.ryoql", "rules/PLE0116/cases.toml",
                  "profiles/PLE0116/benchmark.toml", *FRAGMENTS}
        needed.update(CURRENT + "/" + name for name in proof["prior_summaries_sha256"])
        for relative in needed:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(RUFF / relative, path)
        return root, current, target, proof, controls

    def test_actual_direct_negative_target_validates(self):
        target = RUFF / CURRENT / "ple0116-target-supplement"
        rows = verified_rows(RUFF, RUFF / CURRENT, {"PLE0116": {"status": "FAIL"}}, target)
        self.assertEqual([(row["status"], row["case_count"], row["raw_exact"]) for row in rows],
                         [("PASS", 12, 12)])

    def test_rehashed_forged_direct_negative_finding_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root, current, target, proof, controls = self.direct_fixture(directory)
            archive = target / "raw-receipts.tar.gz"
            negative = controls["direct_negative"][0]
            prefix = negative["archive_prefix"] + "/00"
            with tarfile.open(archive, "r:gz") as source:
                observed = json.loads(source.extractfile(negative["summary_path"]).read())
                raw = json.loads(source.extractfile(prefix + "/stdout.bin").read())
                raw["diagnostics"] = [{"rule": "PLE0116", "file": "case.py", "start": 0, "end": 1}]
                forged = json.dumps(raw).encode()
                observed["results"][0]["stdout_sha256"] = hashlib.sha256(forged).hexdigest()
                changed_summary = json.dumps(observed).encode()
                negative["summary_sha256"] = hashlib.sha256(changed_summary).hexdigest()
                changes = {prefix + "/stdout.bin": forged, negative["summary_path"]: changed_summary}
                replacement = target / "replacement.tar.gz"
                with tarfile.open(replacement, "w:gz") as output:
                    for member in source.getmembers():
                        metadata = copy.copy(member)
                        data = changes.get(member.name)
                        if data is None:
                            data = source.extractfile(member).read()
                        metadata.size = len(data)
                        output.addfile(metadata, io.BytesIO(data))
            replacement.replace(archive)
            (target / "controls.json").write_text(json.dumps(controls))
            proof["controls_sha256"] = sha(target / "controls.json")
            proof["raw_receipts_sha256"] = sha(archive)
            (target / "proof.json").write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, "direct negative target has findings"):
                verified_rows(root, current, {"PLE0116": {"status": "FAIL"}}, target)

    def test_rehashed_direct_request_cannot_switch_to_noop_rules(self):
        with tempfile.TemporaryDirectory() as directory:
            root, current, target, proof, controls = self.direct_fixture(directory)
            archive = target / "raw-receipts.tar.gz"
            name = controls["direct_negative"][0]["archive_prefix"] + "/00/request.json"
            replacement = target / "replacement.tar.gz"
            with tarfile.open(archive, "r:gz") as source:
                request = json.loads(source.extractfile(name).read())
                request["rules"] = "/tmp/noop-rules"
                forged = json.dumps(request).encode()
                with tarfile.open(replacement, "w:gz") as output:
                    for member in source.getmembers():
                        metadata = copy.copy(member)
                        data = forged if member.name == name else source.extractfile(member).read()
                        metadata.size = len(data)
                        output.addfile(metadata, io.BytesIO(data))
            replacement.replace(archive)
            proof["raw_receipts_sha256"] = sha(archive)
            (target / "proof.json").write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, "direct negative target has findings"):
                verified_rows(root, current, {"PLE0116": {"status": "FAIL"}}, target)

    def test_helper_free_actual_fixture_proof_validates(self):
        target = RUFF / CURRENT / "ruf101-target-supplement"
        rows = verified_rows(RUFF, RUFF / CURRENT, {"RUF101": {"status": "FAIL"}}, target)
        self.assertEqual([(row["status"], row["case_count"]) for row in rows], [("PASS", 47)])

    def test_actual_failed_fixture_evidence_cannot_promote(self):
        target = RUFF / CURRENT / "latest-failed-observations"
        if not target.is_dir():
            target = RUFF / CURRENT / "ruf101-target-supplement"
        # RUF100 is the rule that fails a case on the current engine (29 of 30); N999, the earlier failed observation,
        # passes all 55 of its cases since checkpoint 333 and stays only as the superseded observation below.
        rows = verified_failed_rows(RUFF, {"RUF100": {"status": "FAIL"}}, target)
        self.assertEqual([(row["rule_id"], row["status"]) for row in rows], [("RUF100", "FAIL")])
        with self.assertRaisesRegex(ValueError, "changed failed fixture source/settings"):
            verified_failed_rows(RUFF, {"RUF100": {"status": "PASS"}}, target)

    def test_superseded_settings_cannot_certify_current_failed_observation(self):
        if not (RUFF / CURRENT / "latest-failed-observations").is_dir():
            self.skipTest("no newer failed observation registered")
        with self.assertRaisesRegex(ValueError, "changed failed fixture source/settings"):
            verified_failed_rows(RUFF, {"N999": {"status": "FAIL"}}, RUFF / CURRENT / "ruf101-target-supplement")
