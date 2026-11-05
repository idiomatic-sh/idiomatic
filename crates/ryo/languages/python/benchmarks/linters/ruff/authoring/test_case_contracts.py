"""Unavailable reference accounting never means observed zero or full-report parity."""
import copy
import unittest
from authoring.case_records import record_contract_satisfied, case_expectation_summary


class ContractTests(unittest.TestCase):
    def fixture(self):
        case = {'rule':'R1','name':'case','source_sha256':'source','files':['input'],
                'expected':[],'reference_discrepancy':None,'project_context_mode':'isolated',
                'expectation_contract':'diagnostics'}
        candidate = {**case,'side':'candidate','complete':True,'error':None,'case_passed':True,
                     'reference_expected':[],'diagnostics':[],
                     'skipped':[{'rule':'R1','path':'input','reason':'syntax-rejected'}],
                     'observation_schema':'benchmark-case/v2','execution_metadata':{'availability':'complete'},
                     'coverage':{'selected_rule_file_pairs':1,'evaluated_rule_file_pairs':0,
                                 'skipped_rule_file_pairs':1,'unknown_or_unread_rule_file_pairs':0}}
        reference = {**copy.deepcopy(candidate),'side':'reference','skipped':[],'coverage':None,
                     'execution_metadata':{'availability':'unavailable','reason':'findings-only CLI'}}
        return case, {'reference':reference,'candidate':candidate}

    def test_diagnostic_contract_preserves_raw_difference_and_unavailable_dimension(self):
        case,sides=self.fixture()
        self.assertTrue(record_contract_satisfied(case,sides))
        counts=case_expectation_summary([case],list(sides.values()))
        self.assertEqual((counts['intended_behavior_passes'],counts['raw_parity_matches'],counts['raw_parity_mismatches'],counts['contract_only_passes']), (1,0,1,1))
        self.assertEqual(counts['execution_metadata_incomparable'],1)
        self.assertEqual(counts['unexplained_mismatches'],0)
        self.assertNotEqual(sides['reference']['skipped'],sides['candidate']['skipped'])

    def test_full_report_and_real_available_skip_differences_are_strict(self):
        case,sides=self.fixture()
        case['expectation_contract']='complete-report'
        for row in sides.values():row['expectation_contract']='complete-report'
        self.assertFalse(record_contract_satisfied(case,sides))
        reference,candidate=sides['reference'],sides['candidate']
        reference['execution_metadata']={'availability':'complete'}
        reference['coverage']={**candidate['coverage'],'evaluated_rule_file_pairs':1,'skipped_rule_file_pairs':0}
        self.assertFalse(record_contract_satisfied(case,sides))
        reference['coverage']=copy.deepcopy(candidate['coverage'])
        reference['skipped']=copy.deepcopy(candidate['skipped'])
        self.assertTrue(record_contract_satisfied(case,sides))

    def test_missing_forged_or_contradictory_capability_and_contract_are_refused(self):
        for field,values in [('expectation_contract',[None,'unknown','complete-report']),
                             ('execution_metadata',[None,{'availability':'unknown'}, {'availability':'unavailable','reason':''}])]:
            for value in values:
                case,sides=self.fixture();sides['reference'][field]=value
                self.assertFalse(record_contract_satisfied(case,sides))
        case,sides=self.fixture();sides['candidate']['coverage']['unknown_or_unread_rule_file_pairs']=1
        self.assertFalse(record_contract_satisfied(case,sides))
        case,sides=self.fixture();sides['reference']['skipped']=sides['candidate']['skipped']
        self.assertFalse(record_contract_satisfied(case,sides))

    def test_erasing_both_capability_contracts_cannot_forge_a_completed_pair(self):
        case,sides=self.fixture()
        for row in sides.values():
            for field in ['observation_schema','execution_metadata','expectation_contract']:
                row.pop(field)
        counts=case_expectation_summary([case],list(sides.values()))
        self.assertEqual(counts['complete_case_pairs'],0)
        self.assertEqual(counts['intended_behavior_passes'],0)
