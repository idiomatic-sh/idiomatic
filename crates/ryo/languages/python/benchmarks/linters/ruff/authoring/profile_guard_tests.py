"""Profile guards: inventory preservation, cohort checks and shard receipts."""
from __future__ import annotations
import json
import subprocess
import sys
import tempfile
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import case_observations
from authoring import profile_case_tests
from authoring import profile_config
from authoring import profile_proof_tests
from authoring import profile_proofs
from authoring import profile_shards
from authoring import profile_worker


def guards_1() -> tuple[int, tuple]:
    profile_proof_tests.output_directory_self_test()
    profile_proof_tests.authoring_profile_fixed_point_self_test()
    profile_case_tests.execution_metadata_self_test()
    profile_case_tests.project_inputs_identity_self_test()
    profile_proof_tests.profile_accounting_projection_self_test()
    'Exercise fail-closed profile-selection and hash-mismatch guards without invoking Cargo.'
    profile_proof_tests.parser_target_configuration_self_test()
    profile_case_tests.self_test_standard_suppression()
    profile_case_tests.report_cache_contract_self_test()
    profile_case_tests.reference_discrepancy_self_test()
    profile_case_tests.discrepancy_artifact_staging_self_test()
    profile_case_tests.context_guard_self_test()
    profile_proof_tests.actual_proof_asset_guard_self_test()
    profile_proof_tests.standalone_registry_self_test()
    profile_proof_tests.native_standalone_self_test()
    if case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 1, 'column': 8}) != 8 or case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 2, 'column': 2}) != 11:
        print('Ruff coordinate conversion must count physical CR/LF lines and UTF-8 bytes', file=sys.stderr)
        return 1, ()
    sample_inventory = [{'rule': 'A', 'name': 'one', 'case_sha256': 'a'}, {'rule': 'A', 'name': 'two', 'case_sha256': 'b'}, {'rule': 'B', 'name': 'one', 'case_sha256': 'c'}, {'rule': 'C', 'name': 'one', 'case_sha256': 'd'}, {'rule': 'C', 'name': 'two', 'case_sha256': 'e'}]
    prepared = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    try:
        unarchived = json.loads(json.dumps(prepared))
        unarchived_name = next(iter(sorted(unarchived['combined_profiles'])))
        unarchived['combined_profiles'][unarchived_name]['validation_status'] = 'active-current'
        try:
            profile_proofs.verify_files(unarchived, profile_config.ENTRYPOINT.relative_to(profile_config.REPOSITORY).as_posix())
        except ValueError as error:
            if f'combined profile {unarchived_name} must declare archived-historical status' not in str(error):
                raise
        else:
            raise ValueError('prepared manifest admitted a combined profile whose status is not archived-historical')
        dangling_query = json.loads(json.dumps(prepared))
        archived_name = next((name for name, item in sorted(dangling_query['combined_profiles'].items()) if item.get('validation_status') == 'archived-historical'))
        dangling_query['combined_profiles'][archived_name].setdefault('query_files', {})['F401'] = 'crates/ryo/languages/python/benchmarks/linters/ruff/profiles/missing/F401/query.ryoql'
        try:
            profile_proofs.verify_files(dangling_query, profile_config.ENTRYPOINT.relative_to(profile_config.REPOSITORY).as_posix())
        except ValueError as error:
            if f'combined profile {archived_name} names unsealed inputs' not in str(error):
                raise
        else:
            raise ValueError('prepared manifest admitted a dangling query_files entry in a non-active cohort')
        phase_registry = profile_proofs.verify_rule_phase_registry(prepared)
        base = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))
        profile_proofs.check_base_rule_applicability(base, phase_registry)
        missing_default = {**base, 'defaults': base['defaults'][:-1]}
        duplicate_default = {**base, 'defaults': [*base['defaults'][:-1], base['defaults'][0]]}
        missing_policy = {**base, 'rule-applicability': dict(base['rule-applicability'])}
        missing_policy['rule-applicability'].pop('UP025')
        wrong_policy = {**base, 'rule-applicability': {**base['rule-applicability'], 'UP025': 'always'}}
        wrong_supporting = {**base, 'rule-applicability': {**base['rule-applicability'], 'E501': 'accepted-syntax'}}
        for wrong_base in (missing_default, duplicate_default, missing_policy, wrong_policy, wrong_supporting):
            try:
                profile_proofs.check_base_rule_applicability(wrong_base, phase_registry)
            except ValueError:
                continue
            raise ValueError('native phase guard admitted missing/duplicate rules or wrong phases')
        missing_phase = json.loads(json.dumps(phase_registry))
        missing_phase['rules'].pop(next(iter(missing_phase['rules'])))
        unknown_phase = json.loads(json.dumps(phase_registry))
        unknown_rule = next(iter(unknown_phase['rules'].values()))
        unknown_rule['phases'] = ['future-checker']
        mixed_phase = json.loads(json.dumps(phase_registry))
        mixed_rule = next(iter(mixed_phase['rules'].values()))
        mixed_rule['phases'] = ['ast', 'tokens']
        mixed_rule['evidence'].append({'phase': 'tokens', 'path': 'checkers/tokens.rs', 'sha256': '0' * 64})
        for invalid in (missing_phase, unknown_phase, mixed_phase):
            try:
                profile_proofs.verify_rule_phase_registry(prepared, invalid)
            except ValueError:
                continue
            raise ValueError('phase validation accepted missing, unknown, or unreviewed mixed data')
        if phase_registry['rules']['EXE005']['applicability'] != 'always':
            raise ValueError('token-only Ruff rule lost independent applicability')
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'pinned Ruff phase registry validation failed: {error}', file=sys.stderr)
        return 1, ()
    return 0, (sample_inventory,)


def guards_2(sample_inventory) -> tuple[int, tuple]:
    partitions = profile_shards.partition_case_inventory(sample_inventory, 3)
    identity = {'inventory_sha256': 'sealed'}
    receipts = [{'shard_index': index, 'shard_count': 3, 'case_ids': [f"{case['rule']}/{case['name']}" for case in partition], 'status': 0, 'last_process_status': 0, 'case_result_records_valid': True, 'stdout_summary_valid': True, 'global_identity': identity} for index, partition in enumerate(partitions)]
    if not profile_shards.valid_shard_receipts(sample_inventory, receipts, 3, identity):
        print('valid deterministic shard receipts were rejected', file=sys.stderr)
        return 1, ()
    overlapping = json.loads(json.dumps(receipts))
    overlapping[1]['case_ids'].append(overlapping[0]['case_ids'][0])
    incomplete = json.loads(json.dumps(receipts))
    incomplete.pop()
    mixed_worker = json.loads(json.dumps(receipts))
    mixed_worker[2]['global_identity']['worker_sha256'] = 'different'
    if any((profile_shards.valid_shard_receipts(sample_inventory, altered, 3, identity) for altered in (overlapping, incomplete, mixed_worker))):
        print('shard aggregation accepted overlap, missing shard, or mixed worker', file=sys.stderr)
        return 1, ()
    return 0, ()


def guards_3() -> tuple[int, tuple]:
    with tempfile.TemporaryDirectory(prefix='ryo-ruff-shard-test-') as temporary:
        root = Path(temporary)
        profile_root = root / 'repro'
        rules_root = profile_root / 'rules'
        for rule, names in (('A', ('one', 'two')), ('B', ('one',))):
            rule_root = rules_root / rule
            rule_root.mkdir(parents=True)
            cases = [{'name': name, 'files': {'case.py': f'value = {index}\n'}, 'expected': []} for index, name in enumerate(names)]
            (rule_root / 'cases.toml').write_text('\n'.join((f'''[[cases]]\nname = "{case['name']}"\nfiles = {{ "case.py" = {json.dumps(case['files']['case.py'])} }}\nexpected = []\n''' for case in cases)), encoding='utf-8')
        (profile_root / 'benchmark.toml').write_text('parser-configuration = "python-target=3.10"\n', encoding='utf-8')
        worker = root / 'worker'
        ruff = root / 'ruff'
        worker.write_bytes(b'worker')
        ruff.write_bytes(b'ruff')
        output = root / 'output'
        output.mkdir()
        inv = case_records.profile_case_inventory(profile_root)
        case_results = root / 'candidate-cases.jsonl'
        if case_observations.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)[0]:
            raise ValueError('zero-exit worker without case records was accepted')

        def synthetic_case_record(case, side):
            # Ruff has diagnostics but no execution accounting; the candidate
            # reports complete accounting for every selected file.
            coverage = {'selected_rule_file_pairs': len(case['files']), 'evaluated_rule_file_pairs': len(case['files']), 'skipped_rule_file_pairs': 0, 'unknown_or_unread_rule_file_pairs': 0}
            return {
                'observation_schema': 'benchmark-case/v2',
                'expectation_contract': 'diagnostics',
                'rule': case['rule'], 'name': case['name'], 'side': side,
                'complete': True, 'error': None, 'case_passed': True,
                'source_sha256': case['source_sha256'], 'files': case['files'],
                'expected': case['expected'], 'diagnostics': [], 'skipped': [],
                'execution_metadata': {'availability': 'unavailable', 'reason': 'reference tool does not expose execution accounting'} if side == 'reference' else {'availability': 'complete'},
                'coverage': None if side == 'reference' else coverage,
            }

        rows = []
        for case in inv:
            for side in ('reference', 'candidate'):
                rows.append(synthetic_case_record(case, side))
        case_results.write_text('\n'.join((json.dumps(row) for row in rows)) + '\n', encoding='utf-8')
        valid_receipt, _reason = case_observations.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)
        if not valid_receipt:
            raise ValueError('exact worker case receipts were rejected')
        for altered in (
            {**rows[1], 'execution_metadata': None},
            {**rows[1], 'coverage': {**rows[1]['coverage'], 'evaluated_rule_file_pairs': 0}},
            {**rows[0], 'coverage': rows[1]['coverage']},
        ):
            invalid = [dict(row) for row in rows]
            invalid[0 if altered['side'] == 'reference' else 1] = altered
            case_results.write_text('\n'.join((json.dumps(row) for row in invalid)) + '\n', encoding='utf-8')
            if case_observations.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)[0]:
                raise ValueError('missing or contradictory execution metadata was accepted')
        case_results.write_text('\n'.join((json.dumps(row) for row in rows)) + '\n', encoding='utf-8')
        false_audit = lambda command, **_kwargs: subprocess.CompletedProcess(command, 0, b'running 0 tests\n', b'')
        valid_audit = lambda command, **_kwargs: subprocess.CompletedProcess(command, 0, b'ruff 0.16.5: 104 queries authored / 413 shipped defaults (not a parity certificate)\n', b'')
        if profile_worker.audit_worker_protocol(worker, runner=false_audit)[0]:
            raise ValueError('same-named zero-test harness passed the CLI protocol smoke')
        if not profile_worker.audit_worker_protocol(worker, runner=valid_audit)[0]:
            raise ValueError('valid benchmark CLI audit response was rejected')
        calls = []

        def fake_runner(command, *, env, **_kwargs):
            shard_arg = next((value for value in command if value.startswith('--shard=')))
            shard_index, shard_count = (int(value) for value in shard_arg.removeprefix('--shard=').split('/'))
            selected = profile_shards.partition_case_inventory(inv, shard_count)[shard_index]
            calls.append(shard_index)
            records_path = Path(next(arg.split('=', 1)[1] for arg in command if arg.startswith('--case-receipts=')))
            with records_path.open('w', encoding='utf-8') as records:
                for case in selected:
                    for side in ('reference', 'candidate'):
                        records.write(json.dumps(synthetic_case_record(case, side)) + '\n')
            summary = f'{len(selected)} cases passed against both contestants\n'.encode()
            return subprocess.CompletedProcess(command, 0, summary, b'')
        identities = {'prepared_sha256': 'sealed'}
        first = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, runner=fake_runner)
        shards_dir = output / 'profiles' / 'test' / 'shards'
        monolithic = output / 'profiles' / 'test' / 'monolithic.jsonl'
        monolithic.write_bytes(b''.join(((shards_dir / f'{index:03d}.cases.jsonl').read_bytes() for index in range(2))))
        if not case_observations.case_observations_match(monolithic, sorted(shards_dir.glob('*.cases.jsonl'))):
            print('monolithic/sharded full-report comparison failed', file=sys.stderr)
            return 1, ()
        if first != 0 or calls != [0, 1]:
            print('shard execution self-test failed', file=sys.stderr)
            print(f'status={first}; worker-calls={calls}', file=sys.stderr)
            return 1, ()
        more_shards = profile_shards.verify_profile_sharded('bounded-shards', profile_root, worker, ruff, output, identities, 5, runner=fake_runner)
        bounded_plan = json.loads((output / 'profiles' / 'bounded-shards' / 'shard-plan.json').read_text(encoding='utf-8'))
        if more_shards != 0 or bounded_plan.get('requested_shard_count') != 5 or bounded_plan.get('shard_count') != len(inv) or (calls[-len(inv):] != list(range(len(inv)))):
            print('requested shards were not bounded to the nonempty case inventory', file=sys.stderr)
            return 1, ()
    return 0, (case, fake_runner, root, ruff)


def guards_4() -> tuple[int, tuple]:
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    return 0, (manifest,)
