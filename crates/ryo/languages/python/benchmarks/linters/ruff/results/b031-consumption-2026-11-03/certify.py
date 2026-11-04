"""Serialize exact-case evidence through the existing shared certificate contract.

Usage: python3 certify.py ORACLE_JSON RAW_CASES_JSONL STAGED_PROFILE
No candidate finding is used to derive intended diagnostics. The original tagged
snapshot findings are retained; the independently located comprehension finding
is appended by the CPython scope oracle.
"""
from pathlib import Path
import hashlib
import json
import shutil
import sys
import tomllib

RUFF = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RUFF))
from authoring import profile_sources, reference_discrepancies as shared

oracle_path, receipts_path, stage = map(Path, sys.argv[1:])
oracle = json.loads(oracle_path.read_text())
assert oracle['python'] == '3.10.19'
cases_path = RUFF / 'rules/B031/cases.toml'
cases = tomllib.loads(Path(__file__).with_name('initial-cases.toml').read_text())['cases']
current = tomllib.loads(cases_path.read_text())['cases']
assert [(c['name'], c['files']) for c in current] == [(c['name'], c['files']) for c in cases]
rows = [json.loads(line) for line in receipts_path.read_text().splitlines()]
assert len(rows) == 2 * len(cases)
references = {r['name']: r for r in rows if r['side'] == 'reference'}
assert len(references) == len(cases)
for r in rows:
    assert r['complete'] and r['error'] is None
profile = tomllib.loads((stage / 'benchmark.toml').read_text())
assert profile['defaults'] == ['B031']
assert profile['parser-configuration'] == 'python-target=3.10'
assert all(list(c['files']) == ['case.py'] for c in cases)
evidence_root = RUFF / 'profiles/B031/evidence'
evidence_root.mkdir(parents=True, exist_ok=True)
shutil.copyfile(oracle_path, evidence_root / 'oracle.json')
shutil.copyfile(Path(__file__).with_name('oracle.py'), evidence_root / 'oracle.py')
shutil.copyfile(Path(__file__).with_name('original-cases.json'), evidence_root / 'original-cases.json')
shutil.copyfile(receipts_path, evidence_root / 'uncertified-cases.jsonl')
# Exact reference invocation protocol in adapter.rs, normalized by the shared
# invocation_identity service. The actual verifier recomputes and checks this.
settings = {'arguments': ['check', '--isolated', '--output-format', 'json', '--cache-dir', '$STATE/ruff', '--select', 'B031', '--target-version', 'py310', '--', 'case.py'],
            'environment': {'RAYON_NUM_THREADS': '1'}, 'directory': '$ROOT',
            'parser_configuration': 'python-target=3.10', 'project_context_mode': 'Isolated'}
settings_hash = shared.discrepancy_json_hash(settings)
profile_hash = hashlib.sha256((stage / 'benchmark.toml').read_bytes()).hexdigest()
observations = {r['case']: r for r in oracle['cases']}
certified = []
for case in cases:
    name = case['name']
    reference = references[name]['diagnostics']
    if name == 'upstream-full-fixture':
        row = observations[name]
        assert row['retained_snapshot_findings'] == len(case['expected']) == 23
        case['expected'].append(row['independently_added_finding'])
        case['expected'].sort(key=lambda r: (r['file'], r['start'], r['end']))
    if case['expected'] == reference:
        continue
    observation = observations[name]
    source = case['files']['case.py']
    assert observation['original_sha256'] == hashlib.sha256(source.encode()).hexdigest()
    if name != 'upstream-full-fixture':
        assert observation['independent_intended'] == case['expected']
    source_hash = hashlib.sha256(b'case.py\0' + source.encode()).hexdigest()
    intended = shared.discrepancy_output(['case.py'], case['expected'])
    observed = shared.discrepancy_output(['case.py'], reference)
    if name in {'comprehension-shadow-does-not-replace', 'upstream-full-fixture'}:
        reason = 'Python3 comprehension targets belong to their implicit scope and do not replace the enclosing grouper. The following repeated consumer still receives the original iterator. The exact original upstream function uses list += to consume its iterator; the independently supplied companion demonstrates first data then exhaustion. The retained23 upstream findings are unchanged; only the token/AST-derived extra is added.'
    elif name == 'nested-termination-stays-local':
        reason = 'A break exits its nearest inner for, not the outer groupby loop. Both list calls in this unchanged program execute and receive first data then exhaustion. Resetting group consumption across an unrelated inner break misses the second use.'
    elif name in {'opposite-arm-replacement', 'possible-replacement'}:
        reason = 'An assignment on another or optional conditional path does not erase the original group binding on every path. The independent False branch execution consumes the original iterator twice; the True counterpart replaces it and is separately observed. AST-derived original source positions identify the later consumer.'
    elif name in {'comprehension-filter-consumer', 'generator-repeated-consumer', 'later-comprehension-iterator'}:
        reason = 'The explicit two-item outer iterable repeats this comprehension filter, generator body or later iterator clause. Independent unchanged-source execution observes repeated consumption or an exhausted later iteration. These are possible repeated uses of the same original group, not independent fresh group instances or an uncalled-body nonexecution waiver.'
    elif name == 'next-is-incremental-iteration':
        reason = 'The builtin next operation intentionally advances one item at a time. Both successive calls in the unchanged source return1; the second is a valid incremental iteration, not a repeated whole-group traversal doing nothing.'
    elif name == 'one-inner-iteration-is-not-reuse':
        reason = 'The inner iterable is an explicit one-item list with no unpacking, so its sole consumer runs once. Exact execution observes one complete group, with no second traversal. This does not exempt two consumers within the body.'
    elif name == 'conditional-expression':
        reason = 'Only one arm of a Python conditional expression is evaluated. Both explicit flag values produce exactly one consuming list call; mutually exclusive expressions do not constitute repeated consumption.'
    else:
        reason = 'A reference or resolved builtin observation does not consume the grouper. Independent execution of the unchanged source with a recording wrapper returning ordinary builtin list results observes zero or one consuming call, not repeated consumption. The wrapper supplies no alternate iterator or branch behavior.'
    case['reference-discrepancy'] = {'expected': reference, 'identity': {
        'tool': 'ruff', 'version': '0.16.5',
        'observed-executable-sha256': '52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8',
        'profile-sha256': profile_hash, 'settings-sha256': settings_hash,
        'source-sha256': source_hash,
        'reference-output-sha256': shared.discrepancy_json_hash(observed),
        'intended-output-sha256': shared.discrepancy_json_hash(intended)},
        'evidence': [{'path': 'evidence/' + filename,
                      'sha256': hashlib.sha256((evidence_root / filename).read_bytes()).hexdigest(),
                      'kind': 'independent-language-run', 'language-version': 'CPython3.10.19',
                      'target-configuration': 'python-target=3.10',
                      'version-rationale': 'Independent execution uses the exact supported target major/minor. The original sources and separately stated observation namespace/companion are archived; no candidate output derives intended diagnostics.'}
                     for filename in ['oracle.py', 'oracle.json', 'original-cases.json']],
        'reason': reason + ' Exact full diagnostic multisets only; no timeout, incomplete coverage, source rewrite or arbitrary process failure is waived. No upstream issue was opened.'}
    certified.append(name)
assert len(certified) == 16
cases_path.write_text(''.join('[[cases]]\n' + ''.join(f'{k} = {profile_sources.toml_value(v)}\n' for k,v in c.items()) + '\n' for c in cases))
(evidence_root / 'registration.json').write_text(json.dumps({'profile_sha256': profile_hash, 'normalized_reference_settings': settings, 'settings_sha256': settings_hash,
    'raw_receipt_sha256': hashlib.sha256(receipts_path.read_bytes()).hexdigest(), 'certified_cases': certified}, indent=2) + '\n')
print(json.dumps({'certified': certified, 'cases': len(cases), 'settings_sha256': settings_hash}, indent=2))
