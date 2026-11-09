"""Identity checks must not masquerade as a successful RyoQL compilation."""
from __future__ import annotations

from contextlib import redirect_stdout
import io
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import preflight_staged_profile as preflight
from verify_profiles import RUFF_DIR, profile_modules, write_staged_profile


VALID_QUERY = (RUFF_DIR / "rules/RUF063/query.ryoql").read_text(encoding="utf-8")
CHAINED_JOIN_QUERY = '''let nodes = select id, file, span from "semantic-syntax";
let selected = select l.id as id, l.file as file, l.span as span from nodes as l join nodes as r on l.id = r.id join nodes as s on r.id = s.id;
emit selected subject file key(id);
'''


class PreflightValidationScopeTest(unittest.TestCase):
    def make_stage(self, root: Path, query: str) -> None:
        source = RUFF_DIR / "profiles/RUF063/benchmark.toml"
        profile = tomllib.loads(source.read_text(encoding="utf-8"))
        registry = profile_modules(profile, RUFF_DIR)
        for relative in registry.values():
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(RUFF_DIR / relative, destination)
        write_staged_profile(
            source, root / "benchmark.toml", registry,
            "python-target=3.10", {"RUF063": "accepted-syntax"}, module_root=root,
        )
        rule = root / "rules/RUF063"
        rule.mkdir(parents=True)
        (rule / "query.ryoql").write_text(query, encoding="utf-8")
        shutil.copyfile(RUFF_DIR / "rules/RUF063/cases.toml", rule / "cases.toml")

    def test_identity_checks_explicitly_leave_chained_join_unvalidated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_stage(root, CHAINED_JOIN_QUERY)
            output = io.StringIO()
            with patch.object(sys, "argv", ["preflight", str(root), "--active-modules"]):
                with redirect_stdout(output):
                    self.assertEqual(preflight.main(), 0)
            self.assertIn("checked: profile serialization fixed point", output.getvalue())
            self.assertIn("not checked: query parsing/typechecking", output.getvalue())
            self.assertNotIn("checked: query parsing/typechecking and fixture", output.getvalue())

    def test_real_worker_accepts_control_and_rejects_chained_join(self) -> None:
        # The sealed worker and the pinned Ruff are this test's input: named by the environment after the active native gate,
        # and present on disk; an absent one fails the test, never skips it.
        named = {name: os.environ.get(name) for name in ("RYO_PREFLIGHT_WORKER", "RYO_PREFLIGHT_RUFF")}
        absent = [name for name, value in named.items() if not value or not Path(value).is_file()]
        self.assertEqual(absent, [], "the sealed worker and pinned Ruff must be named and present: run after the active native gate")
        worker = Path(named["RYO_PREFLIGHT_WORKER"])
        ruff = Path(named["RYO_PREFLIGHT_RUFF"])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_stage(root, VALID_QUERY)
            preflight.verify_with_worker(root, worker, ruff)
            (root / "rules/RUF063/query.ryoql").write_text(CHAINED_JOIN_QUERY, encoding="utf-8")
            with self.assertRaisesRegex(ValueError, r"expected .*import.*param.*let.*emit"):
                preflight.verify_with_worker(root, worker, ruff)


if __name__ == "__main__":
    unittest.main()


class SourceLanguageSettingsTests(unittest.TestCase):
    def test_optional_explicit_source_provider_is_exact_and_cache_visible(self):
        from authoring import profile_config, profile_sources
        base = {"tool": "ruff", "version": profile_config.PINNED_RUFF_VERSION, "defaults": ["N999"]}
        phases = {"ruff_version": profile_config.PINNED_RUFF_VERSION,
                  "ruff_source_commit": profile_config.PINNED_RUFF_SOURCE_COMMIT,
                  "rules": {"N999": {"applicability": "always"}}}
        ordinary = profile_sources.canonical_run_ryo_settings(base, ["N999"], phases)
        self.assertNotIn("source_language", ordinary)
        selected = profile_sources.canonical_run_ryo_settings({**base, "source-language": "python"}, ["N999"], phases)
        self.assertEqual(selected, {**ordinary, "source_language": "python"})
        for bad in ["", " python", "python ", 1]:
            with self.assertRaisesRegex(ValueError, "exact nonblank"):
                profile_sources.canonical_run_ryo_settings({**base, "source-language": bad}, ["N999"], phases)
