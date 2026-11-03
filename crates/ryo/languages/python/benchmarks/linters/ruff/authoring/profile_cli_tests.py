"""Profile cli tests: linter-specific authoring and proof utilities."""
from __future__ import annotations
import collections
import contextlib
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import profile_case_tests
from authoring import profile_config
from authoring import profile_proof_tests
from authoring import profile_proofs
from authoring import profile_shards
from authoring import profile_sources
from authoring import profile_staging
from authoring import profile_worker
from authoring import reference_discrepancies
from authoring import rule_registration

def inventory_registration_source_root_self_test() -> None:
    """The full inventory must stage pinned evidence from its registered source root."""
    ruff = profile_config.RUFF_DIR
    base = tomllib.loads((ruff / 'benchmark.toml').read_text(encoding='utf-8'))
    phases = json.loads(profile_config.RULE_PHASES_FILE.read_text(encoding='utf-8'))
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    native = profile_proofs.native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text(encoding='utf-8'))
    with tempfile.TemporaryDirectory(prefix='ryo-inventory-registration-') as temporary:
        output = Path(temporary)
        for rule in ('UP031', 'UP032', 'PYI020'):
            count = len(tomllib.loads((ruff / 'rules' / rule / 'cases.toml').read_text(encoding='utf-8'))['cases'])
            stage, entry = rule_registration.stage_full_inventory_rule(rule, output, base, phases, manifest, native)
            assert entry['case_count'] == count and stage.is_dir()
        profile = ruff / 'profiles/UP031/benchmark.toml'
        cases = tomllib.loads((ruff / 'rules/UP031/cases.toml').read_text(encoding='utf-8'))['cases']
        case = next(case for case in cases if case.get('reference-discrepancy'))
        pinned = reference_discrepancies.declared_discrepancy_artifacts(profile, [case], source_root=ruff)
        assert pinned and all((output / 'profiles/UP031/repro' / path).is_file() for path in pinned)
        bad_stage = output / 'negative'
        bad_stage.mkdir()
        for path, digest in (('../escape.json', next(iter(pinned.values()))[1]), ('results/missing.json', next(iter(pinned.values()))[1]), (next(iter(pinned)), '0' * 64)):
            changed = json.loads(json.dumps(case))
            changed['reference-discrepancy']['evidence'][0] = {**changed['reference-discrepancy']['evidence'][0], 'path': path, 'sha256': digest}
            try:
                reference_discrepancies.stage_discrepancy_artifacts(profile, bad_stage, [changed], 'UP031', source_root=ruff)
            except ValueError:
                pass
            else:
                raise AssertionError('unsafe or stale full-inventory evidence was staged')
        try:
            reference_discrepancies.stage_discrepancy_artifacts(profile, bad_stage, [case], 'UP031', source_root=profile.parent)
        except ValueError:
            pass
        else:
            raise AssertionError('incorrect evidence root was accepted')

def self_test() -> int:
    profile_proof_tests.output_directory_self_test()
    profile_proof_tests.authoring_profile_fixed_point_self_test()
    profile_case_tests.execution_metadata_self_test()
    profile_proof_tests.profile_accounting_projection_self_test()
    'Exercise fail-closed profile-selection and hash-mismatch guards without invoking Cargo.'
    profile_proof_tests.parser_target_configuration_self_test()
    profile_case_tests.self_test_standard_suppression()
    profile_case_tests.report_cache_contract_self_test()
    profile_case_tests.reference_discrepancy_self_test()
    profile_case_tests.discrepancy_artifact_staging_self_test()
    inventory_registration_source_root_self_test()
    profile_case_tests.context_guard_self_test()
    profile_proof_tests.actual_proof_asset_guard_self_test()
    profile_proof_tests.standalone_registry_self_test()
    profile_proof_tests.native_standalone_self_test()
    if case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 1, 'column': 8}) != 8 or case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 2, 'column': 2}) != 11:
        print('Ruff coordinate conversion must count physical CR/LF lines and UTF-8 bytes', file=sys.stderr)
        return 1
    sample_inventory = [{'rule': 'A', 'name': 'one', 'case_sha256': 'a'}, {'rule': 'A', 'name': 'two', 'case_sha256': 'b'}, {'rule': 'B', 'name': 'one', 'case_sha256': 'c'}, {'rule': 'C', 'name': 'one', 'case_sha256': 'd'}, {'rule': 'C', 'name': 'two', 'case_sha256': 'e'}]
    prior_cases = [{'name': 'old-name', 'files': {'case.py': 'x = 1\n'}, 'expected': []}, {'name': 'kept', 'files': {'case.py': 'y = 2\n'}, 'expected': [{'file': 'case.py', 'start': 0, 'end': 1}]}]
    extended_cases = [{'name': 'renamed-old-name', 'files': {'case.py': 'x = 1\n'}, 'expected': []}, prior_cases[1], {'name': 'new-case', 'files': {'case.py': 'z = 3\n'}, 'expected': []}]
    try:
        profile_proofs.verify_case_inventory_extension(prior_cases, extended_cases, {'old-name': 'renamed-old-name'})
        for invalid_cases, migrations in ((extended_cases[1:], {'old-name': 'renamed-old-name'}), ([prior_cases[0], {**prior_cases[1], 'expected': []}], {}), ([prior_cases[0], prior_cases[1], prior_cases[0]], {})):
            try:
                profile_proofs.verify_case_inventory_extension(prior_cases, invalid_cases, migrations)
            except ValueError:
                continue
            raise ValueError('case preservation accepted a removal, changed expectation, or duplicate identity')
    except (TypeError, ValueError) as error:
        print(f'canonical case inventory preservation failed: {error}', file=sys.stderr)
        return 1
    prepared = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    try:
        no_active_cohort = json.loads(json.dumps(prepared))
        for descriptor in no_active_cohort.get('combined_profiles', {}).values():
            descriptor['validation_status'] = 'archived-historical'
        try:
            profile_proofs.verify_active_combined_cohort(no_active_cohort)
        except ValueError as error:
            if 'exactly one active combined cohort' not in str(error):
                raise
        else:
            raise ValueError('default acceptance allowed a missing active combined cohort')
        ambiguous_cohorts = json.loads(json.dumps(prepared))
        for descriptor in ambiguous_cohorts.get('combined_profiles', {}).values():
            descriptor['validation_status'] = 'active-current'
        try:
            profile_proofs.verify_active_combined_cohort(ambiguous_cohorts)
        except ValueError as error:
            if 'exactly one active combined cohort' not in str(error):
                raise
        else:
            raise ValueError('default acceptance allowed multiple active combined cohorts')
        phase_registry = profile_proofs.verify_rule_phase_registry(prepared)
        base = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))
        profile_proofs.check_base_rule_applicability(base, phase_registry)
        missing_default = {**base, 'defaults': base['defaults'][:-1]}
        duplicate_default = {**base, 'defaults': [*base['defaults'][:-1], base['defaults'][0]]}
        missing_policy = {**base, 'rule-applicability': dict(base['rule-applicability'])}
        missing_policy['rule-applicability'].pop('UP025')
        wrong_policy = {**base, 'rule-applicability': {**base['rule-applicability'], 'UP025': 'always'}}
        for wrong_base in (missing_default, duplicate_default, missing_policy, wrong_policy):
            try:
                profile_proofs.check_base_rule_applicability(wrong_base, phase_registry)
            except ValueError:
                continue
            raise ValueError('native phase guard admitted missing/duplicate rules or wrong phases')
        if profile_proofs.selected_rule_applicability(prepared, prepared['combined_profiles']['current-canonical']['selected_rules']) != {rule: phase_registry['rules'][rule]['applicability'] for rule in prepared['combined_profiles']['current-canonical']['selected_rules']}:
            raise ValueError('applicability projection changed the selected rule inventory')
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
        return 1
    with tempfile.TemporaryDirectory(prefix='ryo-ruff-resume-manifest-') as temporary:
        resume_root = Path(temporary)
        profile_name = 'sealed-active'
        resume_manifest_data = {'files': {}, 'combined_profiles': {profile_name: {'validation_status': 'active-current', 'expectations_mode': 'complete-full-report', 'selected_rules': ['A']}}}
        prepared_path = resume_root / 'prepared.json'
        manifest_bytes = json.dumps(resume_manifest_data, separators=(',', ':')).encode()
        prepared_path.write_bytes(manifest_bytes)
        repro = resume_root / 'profiles' / profile_name / 'repro'
        repro.mkdir(parents=True)
        (repro / 'benchmark.toml').write_text('defaults = ["A"]\nparser-configuration = "python-target=3.10"\nrule-applicability = {}\n', encoding='utf-8')
        staged = profile_worker.staged_hashes(repro)
        selection = {'manifest_sha256': hashlib.sha256(manifest_bytes).hexdigest(), 'sealed_source_hashes': {}, 'staged_profile_sha256': staged, 'target_version': 'python-target=3.10', 'selected_rule_applicability': {}}
        plan_root = resume_root / 'profiles' / profile_name
        plan_root.mkdir(exist_ok=True)
        (plan_root / 'shard-plan.json').write_text(json.dumps({'profile': profile_name, 'staged_inputs': staged, 'global_identity': {'selection': selection}}), encoding='utf-8')
        try:
            if profile_worker.resume_manifest(resume_root, profile_name) != resume_manifest_data:
                raise ValueError('resume did not use its saved manifest snapshot')
            original_manifest_path = profile_config.MANIFEST
            profile_config.MANIFEST = resume_root / 'mutable-current.json'
            profile_config.MANIFEST.write_text('{"different":true}', encoding='utf-8')
            try:
                if profile_worker.resume_manifest(resume_root, profile_name) != resume_manifest_data:
                    raise ValueError('resume followed the mutable current manifest')
            finally:
                profile_config.MANIFEST = original_manifest_path
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f'sealed resume manifest check failed: {error}', file=sys.stderr)
            return 1
    partitions = profile_shards.partition_case_inventory(sample_inventory, 3)
    identity = {'inventory_sha256': 'sealed'}
    receipts = [{'shard_index': index, 'shard_count': 3, 'case_ids': [f"{case['rule']}/{case['name']}" for case in partition], 'status': 0, 'last_process_status': 0, 'case_result_records_valid': True, 'stdout_summary_valid': True, 'global_identity': identity} for index, partition in enumerate(partitions)]
    if not profile_shards.valid_shard_receipts(sample_inventory, receipts, 3, identity):
        print('valid deterministic shard receipts were rejected', file=sys.stderr)
        return 1
    overlapping = json.loads(json.dumps(receipts))
    overlapping[1]['case_ids'].append(overlapping[0]['case_ids'][0])
    incomplete = json.loads(json.dumps(receipts))
    incomplete.pop()
    mixed_worker = json.loads(json.dumps(receipts))
    mixed_worker[2]['global_identity']['worker_sha256'] = 'different'
    if any((profile_shards.valid_shard_receipts(sample_inventory, altered, 3, identity) for altered in (overlapping, incomplete, mixed_worker))):
        print('shard aggregation accepted overlap, missing shard, or mixed worker', file=sys.stderr)
        return 1
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
        if case_records.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)[0]:
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
        valid_receipt, _reason = case_records.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)
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
            if case_records.candidate_result_valid(profile_root, f'{len(inv)} cases passed against both contestants\n'.encode(), case_results)[0]:
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
            selection_arg = next((value for value in command if value.startswith('--case-selection=')), None)
            if selection_arg is not None:
                selected_ids = set(json.loads(Path(selection_arg.split('=', 1)[1]).read_text(encoding='utf-8')))
                selected = [case for case in selected if f"{case['rule']}/{case['name']}" in selected_ids]
            calls.append(shard_index)
            records_path = Path(env['RYO_VERIFY_CASE_RESULTS'])
            with records_path.open('a' if records_path.exists() else 'w', encoding='utf-8') as records:
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
        if not case_records.case_observations_match(monolithic, sorted(shards_dir.glob('*.cases.jsonl'))):
            print('monolithic/sharded full-report comparison failed', file=sys.stderr)
            return 1
        lost_receipt = shards_dir / '000.receipt.json'
        lost_receipt.unlink()
        recovered = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError('complete case rows should be reconstructed without rerunning')))
        recovered_receipt = json.loads(lost_receipt.read_text(encoding='utf-8'))
        if recovered != 0 or recovered_receipt.get('reconstructed_from_case_records') is not True or recovered_receipt.get('stdout_summary_valid') is not None or (recovered_receipt.get('last_process_status') is not None) or (recovered_receipt.get('reconstruction', {}).get('process_exit_observed') is not False):
            print('full-record/missing-receipt recovery fabricated process proof', file=sys.stderr)
            return 1
        second = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError('complete shard was rerun')))
        interrupted_index = 0
        interrupted_name = f'{interrupted_index:03d}'
        interrupted_path = shards_dir / f'{interrupted_name}.cases.jsonl'
        interrupted_rows = [json.loads(line) for line in interrupted_path.read_text(encoding='utf-8').splitlines()]
        one_case = f"{partitions[interrupted_index][0]['rule']}/{partitions[interrupted_index][0]['name']}"
        interrupted_path.write_text('\n'.join((json.dumps(row) for row in interrupted_rows if f"{row['rule']}/{row['name']}" == one_case)) + '\n', encoding='utf-8')
        (shards_dir / f'{interrupted_name}.receipt.json').unlink()
        third = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=fake_runner)
        torn = shards_dir / '000.cases.jsonl'
        with torn.open('a', encoding='utf-8') as output_records:
            output_records.write('{incomplete json')
        malformed_bytes = torn.read_bytes()
        fourth = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=fake_runner)
        failed_receipt = json.loads((shards_dir / '000.receipt.json').read_text(encoding='utf-8'))
        history = torn.with_name(f'{torn.name}.history-{hashlib.sha256(malformed_bytes).hexdigest()}')
        if (failed_receipt.get('status') != 1
            or failed_receipt.get('last_process_status') != 0
            or failed_receipt.get('case_result_records_valid') is not True
            or 'prior native case receipt could not be read' not in str(failed_receipt.get('error'))
            or not history.is_file() or history.read_bytes() != malformed_bytes):
            print('malformed resume lost its durable failure or exact raw history', file=sys.stderr)
            return 1
        if first != 0 or recovered != 0 or second != 0 or (third != 0) or (fourth != 1) or (calls != [0, 1, 0, 0]):
            print('shard execution or identity-checked resume self-test failed', file=sys.stderr)
            print(f'statuses={(first, recovered, second, third, fourth)}; worker-calls={calls}', file=sys.stderr)
            return 1
        cases_path = rules_root / 'A' / 'cases.toml'
        original_cases = cases_path.read_bytes()
        cases_path.write_bytes(original_cases + b'# staged-input mutation\n')
        try:
            profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError('changed staged cases should fail before worker execution')))
            print('changed staged case inputs were accepted on resume', file=sys.stderr)
            return 1
        except ValueError as error:
            if 'staged profile inputs' not in str(error):
                raise
        finally:
            cases_path.write_bytes(original_cases)
        more_shards = profile_shards.verify_profile_sharded('bounded-shards', profile_root, worker, ruff, output, identities, 5, runner=fake_runner)
        bounded_plan = json.loads((output / 'profiles' / 'bounded-shards' / 'shard-plan.json').read_text(encoding='utf-8'))
        if more_shards != 0 or bounded_plan.get('requested_shard_count') != 5 or bounded_plan.get('shard_count') != len(inv) or (calls[-len(inv):] != list(range(len(inv)))):
            print('requested shards were not bounded to the nonempty case inventory', file=sys.stderr)
            return 1
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    with tempfile.TemporaryDirectory(prefix='ryo-ruff-profile-guards-') as temporary:
        root = Path(temporary)
        single_profile = profile_config.PROFILE_DIR / 'UP006' / 'benchmark.toml'
        single_data = tomllib.loads(single_profile.read_text(encoding='utf-8'))
        registry = profile_sources.combined_modules(single_data, ['UP006'], manifest)
        module_id = 'python/ruff/suppress'
        module_source = 'fragments/suppress.ryoql'
        expected_registry = profile_sources.profile_modules(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8')))
        expected_registry.update(profile_sources.profile_modules(single_data))
        if expected_registry.get('python/ruff/suppress-directives') != 'fragments/suppress-directives.ryoql':
            raise AssertionError('standard suppression lost its explicit transitive directive module')
        staged_profile = root / 'single' / 'benchmark.toml'
        staged_profile.parent.mkdir(parents=True)
        profile_sources.write_staged_profile(single_profile, staged_profile, registry, rule_applicability=profile_proofs.selected_rule_applicability(manifest, ['UP006']))
        profile_sources.copy_modules(registry, staged_profile.parent)
        if registry != expected_registry or tomllib.loads(staged_profile.read_text(encoding='utf-8')).get('query-modules') != expected_registry or tomllib.loads(staged_profile.read_text(encoding='utf-8')).get('rule-applicability') != {'UP006': 'accepted-syntax'} or any((not (staged_profile.parent / value).is_file() or profile_config.sha256(staged_profile.parent / value) != profile_config.sha256(profile_config.RUFF_DIR / value) for value in expected_registry.values())):
            print('single-rule profile did not inherit the shared query module', file=sys.stderr)
            return 1
        py311_profile = root / 'py311' / 'benchmark.toml'
        py311_profile.parent.mkdir(parents=True)
        profile_sources.write_staged_profile(single_profile, py311_profile, registry, profile_sources.parser_configuration('py311'), profile_proofs.selected_rule_applicability(manifest, ['UP006']))
        if tomllib.loads(py311_profile.read_text(encoding='utf-8')).get('parser-configuration') != 'python-target=3.11':
            print('target-specific parser configuration was not staged', file=sys.stderr)
            return 1
        try:
            profile_sources.parser_configuration('py316')
        except ValueError:
            pass
        else:
            print('unsupported Python target version was accepted', file=sys.stderr)
            return 1
        durable = root / 'durable-run'
        durable.mkdir()
        repro = durable / 'profiles' / 'mock' / 'repro'
        repro.mkdir(parents=True)
        (repro / 'benchmark.toml').write_text('defaults = ["F401"]\n', encoding='utf-8')
        fake_output = subprocess.CompletedProcess(['fake-worker'], 7, stdout=b'candidate stdout\n', stderr=b'candidate stderr\n')
        calls = []

        def fake_runner(command, **options):
            calls.append((command, options))
            return fake_output
        command = ['fake-worker', '--case', 'exact']
        mock_executable = root / 'mock-executable'
        mock_executable.write_bytes(b'fixed mock executable')
        mock_identity = {'identity': 'sealed', 'worker': {'executable': {'path': str(mock_executable.resolve()), 'sha256': profile_config.sha256(mock_executable)}}}
        if profile_worker.verify_profile('mock', repro, command, durable, mock_identity, runner=fake_runner) != 7:
            print('mock verifier status was not preserved', file=sys.stderr)
            return 1
        result_path = durable / 'profiles' / 'mock' / 'result.json'
        result = json.loads(result_path.read_text(encoding='utf-8'))
        if len(calls) != 1 or calls[0][0] != command or (not calls[0][1]['capture_output']) or calls[0][1]['text'] or ((durable / 'profiles' / 'mock' / 'stdout.bin').read_bytes() != fake_output.stdout) or ((durable / 'profiles' / 'mock' / 'stderr.bin').read_bytes() != fake_output.stderr) or (result['status'] != 7) or (result['identities']['identity'] != 'sealed') or (result['identities']['worker']['executable']['sha256'] != profile_config.sha256(mock_executable)) or (result['staged_inputs'] != {'benchmark.toml': profile_config.sha256(repro / 'benchmark.toml')}):
            print('durable verifier output or identities were not preserved', file=sys.stderr)
            return 1
        fake_worker = root / 'ruff-fake-worker'
        fake_worker.write_text("#!/usr/bin/env python3\nimport os, pathlib, subprocess, sys\nlabel = 'child' if len(sys.argv) > 1 and sys.argv[1] == 'run-ryo' else 'parent'\npathlib.Path(os.environ['WORKER_PATHS']).open('a').write(label + '|' + sys.argv[0] + '\\n')\nif label == 'parent': subprocess.run([sys.argv[0], 'run-ryo'], check=True)\n", encoding='utf-8')
        fake_worker.chmod(493)
        worker_record = {'worker': {}}
        profile_worker.worker_identity(worker_record, fake_worker)
        if worker_record['worker']['executable'] != {'path': str(fake_worker.resolve()), 'sha256': profile_config.sha256(fake_worker)}:
            print('frozen worker identity was not captured', file=sys.stderr)
            return 1
        build_root = root / 'worker-build'
        build_root.mkdir()
        artifact_line = json.dumps({'reason': 'compiler-artifact', 'target': {'name': 'ruff', 'kind': ['bench'], 'src_path': '/repo/languages/python/benchmarks/linters/ruff/bench.rs', 'test': False}, 'executable': str(fake_worker)}).encode()
        build_calls = []

        def successful_build_runner(build_command, **options):
            build_calls.append((build_command, options))
            return subprocess.CompletedProcess(build_command, 0, stdout=artifact_line + b'\n', stderr=b'build output\n')
        source_identity = profile_worker.worker_source_identity
        stable_source_identity = {'repository_revision': 'self-test-revision', 'working_tree_status': [], 'working_tree_diff_sha256': '0' * 64, 'source_tree_sha256': '1' * 64}
        profile_worker.worker_source_identity = lambda: dict(stable_source_identity)
        try:
            frozen_worker, build_identity = profile_worker.build_worker_once(build_root, contextlib.ExitStack(), runner=successful_build_runner)
        finally:
            profile_worker.worker_source_identity = source_identity
        profile_worker.worker_source_identity = lambda: (_ for _ in ()).throw(AssertionError('resuming a frozen worker must not re-identify current sources'))
        try:
            reused_worker, reused_identity = profile_worker.reuse_frozen_worker(build_root)
        finally:
            profile_worker.worker_source_identity = source_identity
        if reused_worker != frozen_worker or reused_identity['source_before_build'] != reused_identity['source_after_build']:
            print('source edits after a stable build invalidated the frozen worker', file=sys.stderr)
            return 1
        build_record_path = build_root / 'build' / 'result.json'
        original_build_record = build_record_path.read_text(encoding='utf-8')
        scheduled = json.loads(original_build_record)
        scheduled['command'].insert(4, '-j6')
        try:
            build_record_path.write_text(json.dumps(scheduled), encoding='utf-8')
            profile_worker.reuse_frozen_worker(build_root)
            bad_commands = [['other' if value == 'ruff' else value for value in scheduled['command']], scheduled['command'] + ['--features', 'other'], scheduled['command'] + ['--profile', 'dev'], [value for value in scheduled['command'] if value != '--locked'], scheduled['command'] + ['--manifest-path', 'other.toml']]
            for command in bad_commands:
                changed = json.loads(json.dumps(scheduled))
                changed['command'] = command
                build_record_path.write_text(json.dumps(changed), encoding='utf-8')
                try:
                    profile_worker.reuse_frozen_worker(build_root)
                except ValueError:
                    pass
                else:
                    raise AssertionError('changed worker build target/policy was accepted')
            changed = json.loads(json.dumps(scheduled))
            changed['source_after_build']['source_tree_sha256'] = 'changed'
            build_record_path.write_text(json.dumps(changed), encoding='utf-8')
            try:
                profile_worker.reuse_frozen_worker(build_root)
            except ValueError:
                pass
            else:
                raise AssertionError('unstable worker build source was accepted')
        finally:
            build_record_path.write_text(original_build_record, encoding='utf-8')
        original_worker_bytes = frozen_worker.read_bytes()
        frozen_worker.write_bytes(b'changed worker')
        try:
            profile_worker.reuse_frozen_worker(build_root)
            print('changed frozen worker bytes were accepted', file=sys.stderr)
            return 1
        except ValueError as error:
            if 'missing or changed' not in str(error):
                raise
        finally:
            frozen_worker.write_bytes(original_worker_bytes)
        try:
            profile_worker.validate_output_directory(profile_config.REPOSITORY / 'crates' / 'ryo' / 'unsafe-output', resume=False)
            print('new output under the Rust source tree was accepted', file=sys.stderr)
            return 1
        except ValueError as error:
            if 'outside crates/ryo' not in str(error):
                raise
        profile_commands = [profile_worker.command_for(frozen_worker, Path('ruff'), root / 'profile-a'), profile_worker.command_for(frozen_worker, Path('ruff'), root / 'profile-b')]
        worker_paths = root / 'worker-paths.log'
        env = {**os.environ, 'WORKER_PATHS': str(worker_paths)}
        runs = [subprocess.run(command, env=env, capture_output=True, check=False) for command in profile_commands]
        observed_paths = worker_paths.read_text(encoding='utf-8').splitlines()
        if len(build_calls) != 1 or len(runs) != 2 or any((run.returncode != 0 for run in runs)) or (profile_commands[0][0] != profile_commands[1][0]) or (observed_paths != [f'parent|{frozen_worker}', f'child|{frozen_worker}', f'parent|{frozen_worker}', f'child|{frozen_worker}']) or any(('--bench' in command for command in profile_commands)) or (build_identity['frozen_executable']['sha256'] != profile_config.sha256(frozen_worker)) or (not build_identity['source_stable_during_build']):
            print('profile batch did not use one stable frozen worker', file=sys.stderr)
            return 1
        failed_build_root = root / 'failed-worker-build'
        failed_build_root.mkdir()

        def failed_build_runner(build_command, **_options):
            return subprocess.CompletedProcess(build_command, 101, stdout=b'cargo stdout\n', stderr=b'cargo compiler diagnostic\n')
        try:
            profile_worker.build_worker_once(failed_build_root, contextlib.ExitStack(), runner=failed_build_runner)
            print('failed worker build unexpectedly succeeded', file=sys.stderr)
            return 1
        except ValueError:
            failed_build = json.loads((failed_build_root / 'build/result.json').read_text())
            if failed_build['status'] != 101 or (failed_build_root / 'build/stdout.bin').read_bytes() != b'cargo stdout\n' or (failed_build_root / 'build/stderr.bin').read_bytes() != b'cargo compiler diagnostic\n':
                print('failed worker build diagnostics were not persisted', file=sys.stderr)
                return 1
        source_identity = profile_worker.worker_source_identity
        identities = iter([{'revision': 'before', 'source_tree_sha256': 'before'}, {'revision': 'after', 'source_tree_sha256': 'after'}, {'revision': 'after', 'source_tree_sha256': 'after'}])
        profile_worker.worker_source_identity = lambda: next(identities)
        changed_build_root = root / 'changed-worker-build'
        changed_build_root.mkdir()
        try:
            profile_worker.build_worker_once(changed_build_root, contextlib.ExitStack(), runner=successful_build_runner)
            print('worker source edit during build unexpectedly succeeded', file=sys.stderr)
            return 1
        except ValueError:
            changed_build = json.loads((changed_build_root / 'build/result.json').read_text())
            if changed_build['source_stable_during_build'] or 'sources changed' not in changed_build['error']:
                print('worker source edit during build was not recorded', file=sys.stderr)
                return 1
        finally:
            profile_worker.worker_source_identity = source_identity

        def failing_runner(_command, **_options):
            raise OSError('mock worker could not start')
        error_repro = durable / 'profiles' / 'mock-error' / 'repro'
        error_repro.mkdir(parents=True)
        (error_repro / 'cases.toml').write_text('[[cases]]\n', encoding='utf-8')
        if profile_worker.verify_profile('mock-error', error_repro, command, durable, {}, runner=failing_runner) != 1:
            print('mock verifier startup failure did not return failure', file=sys.stderr)
            return 1
        error_record = json.loads((durable / 'profiles' / 'mock-error' / 'result.json').read_text(encoding='utf-8'))
        if error_record['status'] != 1 or 'mock worker could not start' not in error_record['error'] or (not (durable / error_record['reproduction_directory'] / 'cases.toml').is_file()):
            print('verifier startup exception was not preserved', file=sys.stderr)
            return 1
        profile_source = root / 'profile-source.toml'
        profile_source.write_text('tool = "ruff"\nversion = "0"\nsha256 = ["pin"]\ndefaults = ["F401"]\n[query-modules] # existing but empty\n', encoding='utf-8')
        module_registry = {'python/ruff/suppress': 'rules/F401/query.ryoql'}
        profile_sources.copy_modules(module_registry, root)
        staged_profile = root / 'module-profile.toml'
        profile_sources.write_staged_profile(profile_source, staged_profile, module_registry)
        parsed_profile = tomllib.loads(staged_profile.read_text(encoding='utf-8'))
        if parsed_profile.get('query-modules') != module_registry:
            print('staged query module registry did not remain valid TOML', file=sys.stderr)
            return 1
        staged_module = root / 'rules/F401/query.ryoql'
        if not staged_module.is_file() or staged_module.read_bytes() != (profile_config.RUFF_DIR / module_registry['python/ruff/suppress']).read_bytes():
            print('staged query module bytes changed', file=sys.stderr)
            return 1
        shape_descriptor = manifest['combined_profiles'][profile_config.SHAPE_PROFILE]
        expanded_descriptor = manifest['combined_profiles']['expanded-defaults']
        if profile_staging.combined_query_filename('LOG015', expanded_descriptor) != 'query.ryoql':
            print('expanded cohort did not retain its sealed LOG015 query', file=sys.stderr)
            return 1
        for rule in ('LOG015', 'ASYNC115'):
            profile_case = profile_sources.prepared_case_path(rule, manifest['profiles'][rule])
            canonical_case = profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml'
            historical_case = profile_staging.combined_case_path(rule, expanded_descriptor)
            if profile_case.read_bytes() != canonical_case.read_bytes() or historical_case.read_bytes() == canonical_case.read_bytes() or expanded_descriptor.get('case_files', {}).get(rule) == canonical_case.relative_to(profile_config.REPOSITORY).as_posix() or (profile_staging.combined_query_filename(rule, expanded_descriptor) != 'query.ryoql'):
                print(f'{rule} candidate snapshot changed the frozen expanded cohort', file=sys.stderr)
                return 1
        f821_record = manifest['profiles']['F821']
        f821_candidate = profile_config.RUFF_DIR / 'rules/F821/query.ryoql'
        f821_cases = profile_sources.prepared_case_path('F821', f821_record)
        historical_candidate = profile_staging.combined_query_path('F821', expanded_descriptor)
        historical_cases = profile_staging.combined_case_path('F821', expanded_descriptor)
        if f821_cases.read_bytes() != (profile_config.RUFF_DIR / 'rules/F821/cases.toml').read_bytes() or historical_candidate == f821_candidate or (historical_cases == f821_cases) or (historical_candidate.read_bytes() == f821_candidate.read_bytes()) or (historical_cases.read_bytes() == f821_cases.read_bytes()):
            print('F821 profile snapshots changed the frozen expanded cohort', file=sys.stderr)
            return 1
        shape_profile = tomllib.loads((profile_config.REPOSITORY / shape_descriptor['profile_file']).read_text(encoding='utf-8'))
        shape_registry = profile_sources.combined_modules(shape_profile, shape_descriptor['selected_rules'], manifest)
        shape_stage = root / 'shape-110-profile'
        shape_stage.mkdir()
        profile_sources.copy_modules(shape_registry, shape_stage)
        profile_sources.write_staged_profile(profile_config.REPOSITORY / shape_descriptor['profile_file'], shape_stage / 'benchmark.toml', shape_registry)
        if shape_registry.get('python/ruff/suppress') != 'fragments/suppress.ryoql' or not (shape_stage / 'fragments/suppress.ryoql').is_file() or tomllib.loads((shape_stage / 'benchmark.toml').read_text(encoding='utf-8')).get('query-modules', {}).get('python/ruff/suppress') != 'fragments/suppress.ryoql':
            print("combined profile omitted an active rule's query module", file=sys.stderr)
            return 1
        for receipt in manifest.pop('actual_proof_receipts', []):
            manifest['files'].pop(receipt, None)
            receipt_data = json.loads((profile_config.REPOSITORY / receipt).read_text(encoding='utf-8'))
            for artifact in receipt_data.get('execution_artifacts', {}):
                manifest['files'].pop(artifact, None)
        normalization_root = profile_config.RUFF_DIR / 'results/canonical-query-sources-2026-11-02'
        normalization = json.loads((normalization_root / 'normalization.json').read_text(encoding='utf-8'))
        migrated = {entry['old_path']: entry for entry in normalization['entries']}
        with tarfile.open(normalization_root / 'divergent-candidates.tar.gz') as archive:
            for relative, expected_hash in manifest['files'].items():
                source = profile_config.REPOSITORY / relative
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                if source.is_file():
                    shutil.copyfile(source, destination)
                    continue
                entry = migrated.get(relative)
                if entry is None:
                    raise FileNotFoundError(f'frozen input is missing: {relative}')
                if entry['kind'] == 'divergent_candidate_archived':
                    member = archive.extractfile(f"rules/{entry['rule']}/candidate.ryoql")
                    if member is None:
                        raise FileNotFoundError(f'archived candidate is missing: {relative}')
                    restored = member.read()
                else:
                    restored = (profile_config.REPOSITORY / entry['active_path']).read_bytes()
                if hashlib.sha256(restored).hexdigest() != expected_hash:
                    raise ValueError(f'frozen input bytes changed during canonical path migration: {relative}')
                destination.write_bytes(restored)
        for query in (profile_config.RUFF_DIR / 'rules').glob('*/query.ryoql'):
            destination = root / query.relative_to(profile_config.REPOSITORY)
            destination.parent.mkdir(parents=True, exist_ok=True)
            if destination.is_file():
                assert destination.read_bytes() == query.read_bytes()
            else:
                shutil.copyfile(query, destination)
        for module in (profile_config.RUFF_DIR / 'authoring').glob('*.py'):
            relative = module.relative_to(profile_config.REPOSITORY).as_posix()
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(module, destination)
            manifest['files'][relative] = profile_config.sha256(module)
        helper_relative = Path(profile_config.ENTRYPOINT).relative_to(profile_config.REPOSITORY).as_posix()
        required = profile_proofs.manifest_paths(manifest, helper_relative)
        for relative in required:
            destination = root / relative
            if not destination.is_file():
                source = profile_config.REPOSITORY / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
        # The frozen prepared descriptors predate two appended factory controls.
        # This sandbox copies today's canonical case files, so align only those
        # append-only inventories before testing unrelated preflight refusals.
        for rule in ('FURB132', 'PERF402'):
            recorded = manifest['profiles'][rule]
            cases_relative = profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY) / recorded['cases_file']
            cases_path = root / cases_relative
            if not cases_path.is_file():
                cases_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(profile_config.REPOSITORY / cases_relative, cases_path)
            names = [case['name'] for case in tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']]
            previous = recorded['case_names']
            if names[:len(previous)] != previous or len(names) != len(previous) + 2:
                print(f'{rule} fixture changed beyond its two appended controls', file=sys.stderr)
                return 1
            recorded['case_names'] = names
            recorded['case_count'] = len(names)
        manifest['files'] = {relative: profile_config.sha256(root / relative) for relative in sorted(required)}
        temporary_manifest = root / profile_config.MANIFEST.relative_to(profile_config.REPOSITORY)
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
        helper = root / helper_relative
        ruff = str(Path(sys.executable).resolve())
        pristine_manifest = json.loads(json.dumps(manifest))
        invalid_selection = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'invalid-output'), 'INVALID-RULE'], capture_output=True, text=True, check=False)
        if invalid_selection.returncode != 2 or 'unknown or duplicate profile rule' not in invalid_selection.stderr:
            print(f'invalid profile selection did not fail closed: status={invalid_selection.returncode}; stderr={invalid_selection.stderr!r}', file=sys.stderr)
            return 1
        duplicate_selection = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'duplicate-output'), 'F401', 'F401'], capture_output=True, text=True, check=False)
        if duplicate_selection.returncode != 2 or 'unknown or duplicate profile rule' not in duplicate_selection.stderr:
            print('duplicate profile selection did not fail closed', file=sys.stderr)
            return 1
        candidate = root / next((path for path in manifest['files'] if path.endswith('/rules/F401/candidate.ryoql')))
        original_candidate = candidate.read_bytes()
        candidate.write_bytes(original_candidate + b'\n')
        changed_input = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'changed-output'), 'F401'], capture_output=True, text=True, check=False)
        if changed_input.returncode != 2 or 'prepared input hash mismatch' not in changed_input.stderr:
            print('prepared input hash mismatch did not fail closed', file=sys.stderr)
            return 1
        candidate.write_bytes(original_candidate)
        manifest = json.loads(temporary_manifest.read_text(encoding='utf-8'))
        last_profile = max(manifest['profiles'])
        last_rule = manifest['profiles'][last_profile].get('rule_id', last_profile)
        manifest['profiles'][last_profile]['case_count'] += 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
        fake_bin = root / 'fake-bin'
        fake_bin.mkdir()
        marker = root / 'worker-was-started'
        fake_cargo = fake_bin / 'cargo'
        fake_git = fake_bin / 'git'
        fake_git.write_text('#!/bin/sh\ncase "$1" in\n  rev-parse) echo self-test-revision ;;\n  diff|status|ls-files) : ;;\n  *) exit 2 ;;\nesac\n', encoding='utf-8')
        fake_git.chmod(493)
        cargo_build_marker = root / 'cargo-build-count'
        reference_worker = root / 'reference-worker'
        reference_worker.write_text(f"""#!/bin/sh\nif [ "$1" = audit ]; then echo 'ruff 0.16.5: 104 queries authored / 413 shipped defaults (not a parity certificate)'; exit 0; fi\nprintf '%s\\n' "$*" >> {marker}\nexit 0\n""", encoding='utf-8')
        reference_worker.chmod(493)
        artifact_json = json.dumps({'reason': 'compiler-artifact', 'target': {'name': 'ruff', 'kind': ['bench'], 'src_path': '/repo/languages/python/benchmarks/linters/ruff/bench.rs', 'test': False}, 'executable': str(reference_worker)})
        fake_cargo.write_text(f"""#!/bin/sh\nif [ "$1" = "--version" ]; then echo 'cargo test'; exit 0; fi\nprintf '%s\\n' "$*" >> {cargo_build_marker}\nprintf '%s\\n' '{artifact_json}'\n""", encoding='utf-8')
        fake_cargo.chmod(493)
        pinned_ruff = shutil.which('ruff')
        if pinned_ruff is None:
            print('reference-only self-test requires the pinned Ruff executable on PATH', file=sys.stderr)
            return 1
        inventory_collector = fake_bin / 'inventory-collector'
        inventory_collector.write_text('#!/bin/sh\nexit 0\n', encoding='utf-8')
        inventory_collector.chmod(493)
        inventory_output = root / 'default-inventory-output'
        default_inventory = subprocess.run([sys.executable, str(helper), pinned_ruff, '--worker', str(reference_worker), '--inventory-runner', str(inventory_collector), '--output-dir', str(inventory_output), 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
        registration_path = inventory_output / 'registration.json'
        registration = json.loads(registration_path.read_text(encoding='utf-8')) if registration_path.is_file() else {}
        rows = registration.get('rules', [])
        if default_inventory.returncode != 0 or len(rows) != 413 or [row['rule_id'] for row in rows if row['command'] is not None] != ['F401'] or sum(row['reason'] == 'not selected for this invocation' for row in rows) != 412:
            print(f'default canonical inventory routing failed: status={default_inventory.returncode}; stderr={default_inventory.stderr!r}', file=sys.stderr)
            return 1
        reference_output = root / 'reference-only-output'
        reference_only = subprocess.run([sys.executable, str(helper), pinned_ruff, '--prepared', '--output-dir', str(reference_output), '--reference-only', 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}", 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
        builds = cargo_build_marker.read_text().splitlines() if cargo_build_marker.exists() else []
        reference_records = reference_output / 'profiles' / 'F401' / 'reference-cases.jsonl'
        expected_reference_names = {case['name'] for case in tomllib.loads((profile_config.REPOSITORY / 'crates/ryo/languages/python/benchmarks/linters/ruff/rules/F401/cases.toml').read_text(encoding='utf-8'))['cases']}
        observed_reference_names = [json.loads(line)['name'] for line in reference_records.read_text(encoding='utf-8').splitlines()] if reference_records.is_file() else []
        if reference_only.returncode != 0 or builds or (not reference_records.is_file()) or (len(observed_reference_names) != len(expected_reference_names)) or (set(observed_reference_names) != expected_reference_names):
            print(f'reference-only mode did not verify all cases without building a candidate: status={reference_only.returncode}; stderr={reference_only.stderr!r}; builds={builds!r}; records={reference_records.is_file()}', file=sys.stderr)
            return 1
        cargo_build_marker.unlink(missing_ok=True)
        invalid_rule_manifest = json.loads(json.dumps(pristine_manifest))
        invalid_rule_manifest['combined_profiles']['expanded-defaults']['selected_rules'].append('NOT-SHIPPED')
        temporary_manifest.write_text(json.dumps(invalid_rule_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        invalid_rule = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'invalid-rule-output'), '--profile', 'expanded-defaults'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
        if invalid_rule.returncode != 2 or 'combined profile contains a non-default rule' not in invalid_rule.stderr or marker.exists():
            print('non-default combined rule did not fail before candidate work', file=sys.stderr)
            return 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')

        def later_combined_preflight_failure() -> tuple[bool, str]:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            later_name = next((name for name, descriptor in sorted(trial_manifest['combined_profiles'].items()) if descriptor.get('validation_status') == 'active-current'))
            for item in trial_manifest['combined_profiles'].values():
                item['validation_status'] = 'archived-historical'
            descriptor = trial_manifest['combined_profiles'][later_name]
            descriptor['validation_status'] = 'active-current'
            descriptor['case_count'] += 1
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            marker.unlink(missing_ok=True)
            preflight_output = root / 'later-preflight-output'
            check = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(preflight_output), '--profile', later_name, 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}", 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
            receipt_path = preflight_output / 'preflight-error.json'
            receipt = json.loads(receipt_path.read_text(encoding='utf-8')) if receipt_path.is_file() else {}
            valid = check.returncode == 2 and 'complete expected inventory count mismatch' in check.stderr and (not marker.exists()) and (receipt.get('manifest_path') == temporary_manifest.relative_to(root).as_posix()) and (receipt.get('manifest_sha256') == profile_config.sha256(temporary_manifest))
            return (valid, check.stderr)
        later_combined_refused, later_combined_output = later_combined_preflight_failure()
        if not later_combined_refused:
            print('later active combined profile did not fail before candidate work', file=sys.stderr)
            print(later_combined_output, file=sys.stderr)
            return 1
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
        overlay_relative = pristine_manifest['combined_profiles'][profile_config.SHAPE_PROFILE]['expectations_file']
        baseline_overlay = (profile_config.REPOSITORY / overlay_relative).read_bytes()
        (root / pristine_manifest['combined_profiles']['expanded-defaults']['expectations_file']).write_bytes((profile_config.REPOSITORY / pristine_manifest['combined_profiles']['expanded-defaults']['expectations_file']).read_bytes())
        (root / pristine_manifest['combined_profiles']['shape-110']['expectations_file']).write_bytes(baseline_overlay)
        preflight = subprocess.run([sys.executable, str(helper), str(Path(sys.executable).resolve()), '--prepared', '--output-dir', str(root / 'inventory-output'), 'F401', last_rule], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
        if preflight.returncode != 2 or f'case inventory mismatch for {last_profile}' not in preflight.stderr or marker.exists():
            print(f'later-profile preflight guard failed: status={preflight.returncode}; stderr={preflight.stderr.strip()!r}; marker={marker.exists()}', file=sys.stderr)
            return 1

        def shape_preflight_failure(kind: str, expected_error: str) -> bool:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            descriptor = trial_manifest['combined_profiles'][profile_config.SHAPE_PROFILE]
            overlay_path = root / descriptor['expectations_file']
            overlay = json.loads(overlay_path.read_text(encoding='utf-8'))
            if kind == 'source-hash':
                overlay['cases'][0]['case_source_sha256'] = '0' * 64
            else:
                overlay['cases'][0]['case'] = 'unknown-case-key'
            overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            overlay_hash = profile_config.sha256(overlay_path)
            descriptor['expectations_sha256'] = overlay_hash
            trial_manifest['files'][descriptor['expectations_file']] = overlay_hash
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            marker.unlink(missing_ok=True)
            check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'shape-{kind}-output'), '--profile', profile_config.SHAPE_PROFILE], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
            valid = check.returncode == 2 and expected_error in check.stderr and (not marker.exists())
            if not valid:
                print(f'shape guard status={check.returncode}; stderr={check.stderr!r}', file=sys.stderr)
            return valid
        for kind, message in (('source-hash', 'case source hash mismatch'), ('case-key', 'overlay case is absent')):
            (root / overlay_relative).write_bytes(baseline_overlay)
            if not shape_preflight_failure(kind, message):
                print(f'shape {kind} refusal did not fail before candidate work', file=sys.stderr)
                return 1
        (root / overlay_relative).write_bytes(baseline_overlay)
        active_name = next((name for name, item in pristine_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
        ready_descriptor = pristine_manifest['combined_profiles'][active_name]
        ready_file = ready_descriptor.get('ready_assets_file')
        if ready_file is not None:
            ready_path = root / ready_file
            pristine_ready = ready_path.read_bytes()
            for kind in ('query', 'module'):
                trial_manifest = json.loads(json.dumps(pristine_manifest))
                receipt = json.loads(pristine_ready)
                query_paths = set(ready_descriptor['query_files'].values())
                asset = next((item for item in receipt['assets'] if (item['path'] in query_paths) == (kind == 'query')))
                asset['sha256'] = '0' * 64
                ready_path.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
                trial_manifest['files'][ready_file] = profile_config.sha256(ready_path)
                temporary_manifest.write_text(json.dumps(trial_manifest, indent=2) + '\n', encoding='utf-8')
                marker.unlink(missing_ok=True)
                check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'ready-{kind}-output'), '--profile', active_name], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
                if check.returncode != 2 or 'READY query/module bytes differ from receipt' not in check.stderr or marker.exists():
                    print(f'READY {kind} receipt disagreement did not fail before candidate work: {check.stderr!r}', file=sys.stderr)
                    return 1
            ready_path.write_bytes(pristine_ready)

        def expanded_preflight_failure(kind: str, expected_error: str) -> bool:
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            active_name = next((name for name, item in trial_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
            descriptor = trial_manifest['combined_profiles'][active_name]
            overlay_path = root / descriptor['expectations_file']
            overlay = json.loads(overlay_path.read_text(encoding='utf-8'))
            if kind == 'source-hash':
                overlay['cases'][0]['case_source_sha256'] = '0' * 64
            elif kind == 'case-key':
                overlay['cases'][0]['case'] = 'unknown-case-key'
            elif kind == 'missing-case':
                overlay['cases'].pop()
            elif kind == 'missing-rule':
                item = next((row for row in overlay['cases'] if row['expected']))
                item['expected'][0].pop('rule')
            elif kind == 'duplicate':
                item = duplicate = None
                for row in overlay['cases']:
                    cases_path = profile_config.REPOSITORY / row['cases_file']
                    source_cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
                    canonical = next((case for case in source_cases if case['name'] == row['case']))
                    original = collections.Counter(((finding.get('rule', row['rule']), finding['file'], finding['start'], finding['end']) for finding in canonical['expected']))
                    current = collections.Counter(((finding['rule'], finding['file'], finding['start'], finding['end']) for finding in row['expected']))
                    extra_duplicate = next((key for key, count in current.items() if count > 1 and count > original[key]), None)
                    if extra_duplicate is not None:
                        item, duplicate = (row, extra_duplicate)
                        break
                if item is None or duplicate is None:
                    raise ValueError('no externally added duplicate to exercise')
                item['expected'].remove(next((finding for finding in item['expected'] if (finding['rule'], finding['file'], finding['start'], finding['end']) == duplicate)))
                overlay['expected_diagnostic_count'] -= 1
            else:
                raise ValueError(f'unknown self-test: {kind}')
            overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            overlay_hash = profile_config.sha256(overlay_path)
            descriptor['expectations_sha256'] = overlay_hash
            trial_manifest['files'][descriptor['expectations_file']] = overlay_hash
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            marker.unlink(missing_ok=True)
            check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'active-{kind}-output'), '--profile', active_name], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
            success = check.returncode == 2 and expected_error in check.stderr and (not marker.exists())
            if not success:
                print(f'active {kind}: status={check.returncode}; stderr={check.stderr.strip()!r}; cargo-started={marker.exists()}', file=sys.stderr)
            return success
        active_name = next((name for name, item in pristine_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
        expanded_overlay_relative = pristine_manifest['combined_profiles'][active_name]['expectations_file']
        expanded_overlay_bytes = (root / expanded_overlay_relative).read_bytes()
        for kind, message in (('source-hash', 'case source hash mismatch'), ('case-key', 'overlay case is absent or duplicated'), ('missing-case', 'complete expected inventory count mismatch'), ('missing-rule', f'invalid {active_name} expected finding'), ('duplicate', 'overlay diagnostics differ from the sealed oracle')):
            (root / expanded_overlay_relative).write_bytes(expanded_overlay_bytes)
            if not expanded_preflight_failure(kind, message):
                print(f'active full-report {kind} refusal did not fail before candidate work', file=sys.stderr)
                return 1
    print('profile selection and hash-mismatch guards passed')
    return 0
