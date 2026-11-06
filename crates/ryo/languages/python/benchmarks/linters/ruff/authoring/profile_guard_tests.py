"""Profile guards: inventory preservation, cohort checks, resume manifests and shard receipts."""
from __future__ import annotations
import hashlib
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
        for rule in ('UP031', 'UP032', 'PYI020', 'F821', 'PLW0602'):
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
        f821_profile = ruff / 'profiles/F821/benchmark.toml'
        f821_cases = tomllib.loads((ruff / 'rules/F821/cases.toml').read_text(encoding='utf-8'))['cases']
        f821_case = next(case for case in f821_cases if case.get('reference-discrepancy'))
        f821_root = profile_config.discrepancy_source_root('F821', f821_profile)
        f821_pinned = reference_discrepancies.declared_discrepancy_artifacts(f821_profile, [f821_case], source_root=f821_root)
        assert f821_pinned and all((output / 'profiles/F821/repro' / path).is_file() for path in f821_pinned)
        try:
            reference_discrepancies.declared_discrepancy_artifacts(f821_profile, [f821_case], source_root=f821_profile.parent)
        except ValueError:
            pass
        else:
            raise AssertionError('incorrect F821 evidence root was accepted')


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
    inventory_registration_source_root_self_test()
    profile_case_tests.context_guard_self_test()
    profile_proof_tests.actual_proof_asset_guard_self_test()
    profile_proof_tests.standalone_registry_self_test()
    profile_proof_tests.native_standalone_self_test()
    if case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 1, 'column': 8}) != 8 or case_records.ruff_location_to_byte('é\x0cvalue\r\nx', {'row': 2, 'column': 2}) != 11:
        print('Ruff coordinate conversion must count physical CR/LF lines and UTF-8 bytes', file=sys.stderr)
        return 1, ()
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
        return 1, ()
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
        return 1, ()
    return 0, (sample_inventory,)


def guards_2(sample_inventory) -> tuple[int, tuple]:
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
            return 1, ()
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
    return 0, (partitions,)


def guards_3(partitions) -> tuple[int, tuple]:
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
            selection_arg = next((value for value in command if value.startswith('--case-selection=')), None)
            if selection_arg is not None:
                selected_ids = set(json.loads(Path(selection_arg.split('=', 1)[1]).read_text(encoding='utf-8')))
                selected = [case for case in selected if f"{case['rule']}/{case['name']}" in selected_ids]
            calls.append(shard_index)
            records_path = Path(next(arg.split('=', 1)[1] for arg in command if arg.startswith('--case-receipts=')))
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
        if not case_observations.case_observations_match(monolithic, sorted(shards_dir.glob('*.cases.jsonl'))):
            print('monolithic/sharded full-report comparison failed', file=sys.stderr)
            return 1, ()
        lost_receipt = shards_dir / '000.receipt.json'
        lost_receipt.unlink()
        recovered = profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError('complete case rows should be reconstructed without rerunning')))
        recovered_receipt = json.loads(lost_receipt.read_text(encoding='utf-8'))
        if recovered != 0 or recovered_receipt.get('reconstructed_from_case_records') is not True or recovered_receipt.get('stdout_summary_valid') is not None or (recovered_receipt.get('last_process_status') is not None) or (recovered_receipt.get('reconstruction', {}).get('process_exit_observed') is not False):
            print('full-record/missing-receipt recovery fabricated process proof', file=sys.stderr)
            return 1, ()
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
            return 1, ()
        if first != 0 or recovered != 0 or second != 0 or (third != 0) or (fourth != 1) or (calls != [0, 1, 0, 0]):
            print('shard execution or identity-checked resume self-test failed', file=sys.stderr)
            print(f'statuses={(first, recovered, second, third, fourth)}; worker-calls={calls}', file=sys.stderr)
            return 1, ()
        cases_path = rules_root / 'A' / 'cases.toml'
        original_cases = cases_path.read_bytes()
        cases_path.write_bytes(original_cases + b'# staged-input mutation\n')
        try:
            profile_shards.verify_profile_sharded('test', profile_root, worker, ruff, output, identities, 2, resume=True, runner=lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError('changed staged cases should fail before worker execution')))
            print('changed staged case inputs were accepted on resume', file=sys.stderr)
            return 1, ()
        except ValueError as error:
            if 'staged profile inputs' not in str(error):
                raise
        finally:
            cases_path.write_bytes(original_cases)
        more_shards = profile_shards.verify_profile_sharded('bounded-shards', profile_root, worker, ruff, output, identities, 5, runner=fake_runner)
        bounded_plan = json.loads((output / 'profiles' / 'bounded-shards' / 'shard-plan.json').read_text(encoding='utf-8'))
        if more_shards != 0 or bounded_plan.get('requested_shard_count') != 5 or bounded_plan.get('shard_count') != len(inv) or (calls[-len(inv):] != list(range(len(inv)))):
            print('requested shards were not bounded to the nonempty case inventory', file=sys.stderr)
            return 1, ()
    return 0, (case, fake_runner, root, ruff)


def guards_4() -> tuple[int, tuple]:
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    return 0, (manifest,)
