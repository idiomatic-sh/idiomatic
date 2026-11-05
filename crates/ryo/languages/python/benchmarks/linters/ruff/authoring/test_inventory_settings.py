"""Full inventory staging preserves the registered singleton execution authority."""
import copy
import json
from pathlib import Path
import tempfile
import tomllib
import unittest

from authoring import case_records, profile_config, profile_proofs, reference_discrepancies, rule_registration


class InventorySettingsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text())
        cls.phases = json.loads(profile_config.RULE_PHASES_FILE.read_text())
        cls.manifest = json.loads(profile_config.MANIFEST.read_text())
        cls.native = profile_proofs.native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text())

    def test_each_registered_native_scope_survives_global_context_changes(self):
        for entry in self.native['profiles']:
            rule = entry['rule_id']
            with self.subTest(rule=rule):
                changed = {**self.base, 'project-context-mode': 'listed-disk', 'project-search-roots': ['', 'src']}
                actual, authority = rule_registration.native_execution_settings(rule, changed, self.native, self.phases)
                self.assertEqual(authority, entry['effective_settings'])
                self.assertEqual(actual['project-context-mode'], entry['effective_settings']['project_context_mode'])
                self.assertEqual(actual['parser-configuration'], entry['effective_settings']['parser_configuration'])
                self.assertNotIn('project-search-roots', actual)
                self.assertEqual(actual['tool'], self.base['tool'])
                self.assertEqual(actual['version'], self.base['version'])
                self.assertEqual(actual['sha256'], self.base['sha256'])
                self.assertEqual(actual.get('query-modules'), changed.get('query-modules'))
                self.assertEqual(changed['project-context-mode'], 'listed-disk')

    def test_real_staging_preserves_canonical_case_bytes_and_parser_pin(self):
        with tempfile.TemporaryDirectory(prefix='ryo-inventory-settings-') as temporary:
            for rule in ('ASYNC115', 'B025', 'C413', 'PYI017'):
                stage, registration = rule_registration.stage_full_inventory_rule(
                    rule, Path(temporary), self.base, self.phases, self.manifest, self.native)
                source = profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml'
                self.assertEqual((stage / 'rules' / rule / 'cases.toml').read_bytes(), source.read_bytes())
                settings = registration['effective_settings']
                expected = next(entry['effective_settings'] for entry in self.native['profiles'] if entry['rule_id'] == rule)
                self.assertEqual(settings['project_context_mode'], expected['project_context_mode'])
                self.assertEqual(settings['parser_configuration'], expected['parser_configuration'])
                self.assertEqual(settings['rule_applicability'][rule], expected['rule_applicability'][rule])
                self.assertEqual(registration['native_settings'], expected)
                self.assertEqual(registration['native_descriptor_sha256'], profile_config.sha256(profile_config.NATIVE_DESCRIPTOR_FILE))
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
                        rule, Path(temporary), self.base, self.phases, self.manifest, self.native)
                    self.assertEqual((stage / 'benchmark.toml').read_bytes(), source_profile.read_bytes())
                    self.assertEqual((stage / 'rules' / rule / 'cases.toml').read_bytes(), source_cases.read_bytes())
                    cases = tomllib.loads(source_cases.read_text())['cases']
                    for case in cases:
                        if 'reference-discrepancy' in case:
                            certificate = reference_discrepancies.validate_reference_discrepancy(stage, case, rule)
                            self.assertEqual(certificate['identity']['profile-sha256'], profile_config.sha256(stage / 'benchmark.toml'))
                            certified += 1
        self.assertEqual(certified, 77)

    def test_certified_staging_refuses_an_implicit_or_changed_context_identity(self):
        rule = 'UP050'
        with tempfile.TemporaryDirectory(prefix='ryo-stale-context-') as temporary:
            stage, _ = rule_registration.stage_full_inventory_rule(
                rule, Path(temporary), self.base, self.phases, self.manifest, self.native)
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

    def test_unknown_rules_keep_the_declared_context_and_strict_missing_context_refusal(self):
        listed = {**self.base, 'project-context-mode': 'listed-disk'}
        actual, authority = rule_registration.native_execution_settings('NOT_REGISTERED', listed, self.native, self.phases)
        self.assertIs(actual, listed)
        self.assertIsNone(authority)
        with self.assertRaisesRegex(ValueError, 'explicit context'):
            case_records.expected_context_identity({'files': {'case.py': 'pass\n'}}, actual['project-context-mode'])

    def test_explicit_listed_disk_native_context_is_kept(self):
        native = copy.deepcopy(self.native)
        entry = native['profiles'][0]
        expected = entry['effective_settings']
        expected['project_context_mode'] = 'listed-disk'
        expected['project_search_roots'] = ['src']
        actual, authority = rule_registration.native_execution_settings(entry['rule_id'], self.base, native, self.phases)
        self.assertEqual(actual['project-context-mode'], 'listed-disk')
        self.assertEqual(actual['project-search-roots'], ['src'])
        self.assertEqual(authority, expected)
        with self.assertRaisesRegex(ValueError, 'explicit context'):
            case_records.expected_context_identity({'files': {'case.py': 'pass\n'}}, actual['project-context-mode'])

    def test_invalid_selection_flags_phase_parser_and_unknown_settings_are_refused(self):
        changes = [
            {'defaults': ['another-rule']},
            {'flags': {'preview': True}},
            {'rule_applicability': {}},
            {'parser_configuration': 'python-target=unknown'},
            {'project_context_mode': 'unrestricted'},
            {'unreviewed': True},
        ]
        for change in changes:
            with self.subTest(change=change):
                native = copy.deepcopy(self.native)
                entry = native['profiles'][0]
                entry['effective_settings'].update(change)
                with self.assertRaises(ValueError):
                    rule_registration.native_execution_settings(entry['rule_id'], self.base, native, self.phases)


if __name__ == '__main__':
    unittest.main()
