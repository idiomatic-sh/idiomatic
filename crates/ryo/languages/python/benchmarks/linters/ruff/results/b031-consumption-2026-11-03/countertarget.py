"""Serialize an explicit py313 counter stage; shared validators retain policy.

Usage: python3 countertarget.py DEFAULT_STAGE NEW_STAGE CPYTHON313_ORACLE_JSON
The caller owns runtime/IO serialization. No source or expected array is changed.
"""
from pathlib import Path
import copy
import hashlib
import json
import shutil
import sys
import tomllib

RUFF = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RUFF))
from authoring import profile_sources, reference_discrepancies as shared

original, target, observation_path = map(Path, sys.argv[1:])
observation = json.loads(observation_path.read_text())
assert observation['python'] == '3.13.5'
observations = {row['case']: row for row in observation['cases']}
shutil.copytree(original, target)
profile_path = target / 'benchmark.toml'
profile = tomllib.loads(profile_path.read_text())
assert profile['parser-configuration'] == 'python-target=3.10'
profile['parser-configuration'] = 'python-target=3.13'
profile_path.write_text(''.join(f'{key} = {profile_sources.toml_value(value)}\n' for key, value in profile.items()))
evidence = target / 'evidence'
(evidence / 'oracle.json').write_bytes(observation_path.read_bytes())
metadata = json.loads((RUFF / 'profiles/B031/evidence/registration.json').read_text())
assert metadata['profile_sha256'] == hashlib.sha256((original / 'benchmark.toml').read_bytes()).hexdigest()
settings = copy.deepcopy(metadata['normalized_reference_settings'])
position = settings['arguments'].index('--target-version') + 1
assert settings['arguments'][position] == 'py310'
settings['arguments'][position] = 'py313'
settings['parser_configuration'] = 'python-target=3.13'
cases_path = target / 'rules/B031/cases.toml'
cases = tomllib.loads(cases_path.read_text())['cases']
for case in cases:
    if 'reference-discrepancy' not in case:
        continue
    assert observations[case['name']]['original_sha256'] == hashlib.sha256(case['files']['case.py'].encode()).hexdigest()
    certificate = case['reference-discrepancy']
    certificate['identity']['profile-sha256'] = hashlib.sha256(profile_path.read_bytes()).hexdigest()
    certificate['identity']['settings-sha256'] = shared.discrepancy_json_hash(settings)
    for item in certificate['evidence']:
        item['sha256'] = hashlib.sha256((target / item['path']).read_bytes()).hexdigest()
        item['language-version'] = 'CPython3.13.5'
        item['target-configuration'] = 'python-target=3.13'
        item['version-rationale'] = 'The exact same source programs and independent operation/scope oracle execute under the countertarget major/minor. Expected arrays are unchanged; actual pinned-reference output is rechecked by the strict shared replay.'
cases_path.write_text(''.join('[[cases]]\n' + ''.join(f'{key} = {profile_sources.toml_value(value)}\n' for key, value in case.items()) + '\n' for case in cases))
(evidence / 'registration.json').write_text(json.dumps({'profile_sha256': hashlib.sha256(profile_path.read_bytes()).hexdigest(),
    'normalized_reference_settings': settings, 'settings_sha256': shared.discrepancy_json_hash(settings),
    'historical_uncertified_receipt_target': 'python-target=3.10',
    'strict_counter_receipt': 'must be captured separately, never relabel the default-target receipt'}, indent=2) + '\n')
for case in cases:
    shared.validate_reference_discrepancy(target, case, 'B031')
print(json.dumps({'cases': len(cases), 'parser_configuration': profile['parser-configuration'], 'settings_sha256': shared.discrepancy_json_hash(settings)}, indent=2))
