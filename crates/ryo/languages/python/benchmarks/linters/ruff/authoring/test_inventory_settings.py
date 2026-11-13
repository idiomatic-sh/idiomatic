"""Full inventory staging preserves the registered singleton execution authority."""
import json
from pathlib import Path
import tempfile
import tomllib
import unittest

from authoring import case_records, profile_config, profile_sources, reference_discrepancies, rule_registration


class InventorySettingsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text())
        cls.phases = json.loads(profile_config.RULE_PHASES_FILE.read_text())
        cls.manifest = json.loads(profile_config.MANIFEST.read_text())

    def own_settings(self, rule):
        own = tomllib.loads((profile_config.RUFF_DIR / 'profiles' / rule / 'benchmark.toml').read_text())
        return profile_sources.canonical_run_ryo_settings(own, [rule], self.phases)

    def test_each_native_member_keeps_its_own_context_when_the_root_changes(self):
        changed = {**self.base, 'project-context-mode': 'listed-disk', 'project-search-roots': ['', 'src']}
        with tempfile.TemporaryDirectory(prefix='ryo-native-members-') as temporary:
            for rule in profile_sources.native_members():
                with self.subTest(rule=rule):
                    stage, registration = rule_registration.stage_full_inventory_rule(
                        rule, Path(temporary), changed, self.phases, self.manifest)
                    settings = registration['effective_settings']
                    expected = self.own_settings(rule)
                    self.assertEqual(settings['project_context_mode'], expected['project_context_mode'])
                    self.assertEqual(settings['parser_configuration'], expected['parser_configuration'])
                    self.assertEqual(settings['rule_applicability'][rule], expected['rule_applicability'][rule])
                    self.assertNotIn('project-search-roots', tomllib.loads((stage / 'benchmark.toml').read_text()))

    def test_real_staging_preserves_canonical_case_bytes_and_parser_pin(self):
        with tempfile.TemporaryDirectory(prefix='ryo-inventory-settings-') as temporary:
            for rule in ('ASYNC115', 'B025', 'C413', 'PYI017'):
                stage, registration = rule_registration.stage_full_inventory_rule(
                    rule, Path(temporary), self.base, self.phases, self.manifest)
                source = profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml'
                self.assertEqual((stage / 'rules' / rule / 'cases.toml').read_bytes(), source.read_bytes())
                settings = registration['effective_settings']
                expected = self.own_settings(rule)
                self.assertEqual(settings['project_context_mode'], expected['project_context_mode'])
                self.assertEqual(settings['parser_configuration'], expected['parser_configuration'])
                self.assertEqual(settings['rule_applicability'][rule], expected['rule_applicability'][rule])
                self.assertTrue(case_records.profile_case_inventory(stage))

    def test_certified_singletons_have_explicit_context_and_keep_exact_profile_identity(self):
        rules = ('FURB122', 'FURB129', 'PLE0643', 'PLE1142', 'PLE1700', 'PLW2101',
                 'RUF007', 'RUF028', 'SIM113', 'UP008', 'UP012', 'UP045', 'UP050')
        certified = 0
        with tempfile.TemporaryDirectory(prefix='ryo-certified-singletons-') as temporary:
            for rule in rules:
                with self.subTest(rule=rule):
                    source_profile = profile_config.RUFF_DIR / 'profiles' / rule / 'benchmark.toml'
                    source_cases = profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml'
                    self.assertEqual(tomllib.loads(source_profile.read_text())['project-context-mode'], 'isolated')
                    stage, _ = rule_registration.stage_full_inventory_rule(
                        rule, Path(temporary), self.base, self.phases, self.manifest)
                    self.assertEqual((stage / 'benchmark.toml').read_bytes(), source_profile.read_bytes())
                    self.assertEqual((stage / 'rules' / rule / 'cases.toml').read_bytes(), source_cases.read_bytes())
                    cases = tomllib.loads(source_cases.read_text())['cases']
                    for case in cases:
                        if 'reference-discrepancy' in case:
                            certificate = reference_discrepancies.validate_reference_discrepancy(stage, case, rule)
                            self.assertEqual(certificate['identity']['profile-sha256'], profile_config.sha256(stage / 'benchmark.toml'))
                            certified += 1
        self.assertGreater(certified, 0)

    def test_certified_staging_refuses_an_implicit_or_changed_context_identity(self):
        rule = 'UP050'
        with tempfile.TemporaryDirectory(prefix='ryo-stale-context-') as temporary:
            stage, _ = rule_registration.stage_full_inventory_rule(
                rule, Path(temporary), self.base, self.phases, self.manifest)
            cases = tomllib.loads((stage / 'rules' / rule / 'cases.toml').read_text())['cases']
            case = next(case for case in cases if 'reference-discrepancy' in case)
            profile = stage / 'benchmark.toml'
            explicit = profile.read_text()
            for replacement in ('', 'project-context-mode = "listed-disk"\n'):
                with self.subTest(context=replacement):
                    profile.write_text(explicit.replace('project-context-mode = "isolated"\n', replacement))
                    with self.assertRaisesRegex(ValueError, 'stale discrepancy tool/profile identity'):
                        reference_discrepancies.validate_reference_discrepancy(stage, case, rule)
            profile.write_text(explicit)
            self.assertIsNotNone(reference_discrepancies.validate_reference_discrepancy(stage, case, rule))

    def test_listed_disk_requires_an_explicit_case_context(self):
        with self.assertRaisesRegex(ValueError, 'explicit context'):
            case_records.expected_context_identity({'files': {'case.py': 'pass\n'}}, 'listed-disk')


if __name__ == '__main__':
    unittest.main()
