"""Profile guards: staging, frozen workers and expanded cohorts."""
from __future__ import annotations
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
from authoring import profile_config
from authoring import profile_proofs
from authoring import profile_sources
from authoring import profile_staging
from authoring import profile_worker


def staged_request_self_test() -> None:
    """`stage_measurement_request` stages the measured source, takes every rule-side field from it and records the digest of what it staged."""
    with tempfile.TemporaryDirectory(prefix='ryo-staged-request-') as temporary:
        pinned = Path(temporary) / 'pinned.json'
        pinned.write_text(json.dumps({'rules': '/stale/rules', 'selected_rules': ['STALE'], 'files': ['a.py'], 'performance': {'diagnostics': True, 'stale': 1}}), encoding='utf-8')
        output = Path(temporary) / 'staged'
        receipt = profile_staging.stage_measurement_request(pinned, 'HEAD', output, Path(temporary) / 'cache')
        request = json.loads((output / 'request.json').read_text(encoding='utf-8'))
        profile = tomllib.loads((output / 'profile' / 'benchmark.toml').read_text(encoding='utf-8'))
        if receipt != json.loads((output / 'staged.json').read_text(encoding='utf-8')) or receipt['staged_digest'] != profile_staging.staged_digest(output / 'profile'):
            raise AssertionError('the staging receipt does not record the digest of the staged files')
        if request['rules'] != str(output / 'profile' / 'rules') or request['selected_rules'] != profile['defaults'] or request['performance'] != {'diagnostics': True} or request['files'] != ['a.py']:
            raise AssertionError('a staged request kept a rule-side field of the pinned request or lost its corpus side')
        if receipt['request_sha256'] != profile_config.sha256(output / 'request.json') or receipt['pinned_request_sha256'] != profile_config.sha256(pinned):
            raise AssertionError('the staging receipt does not bind the request and the pinned request by hash')
        try:
            profile_staging.stage_measurement_request(pinned, 'HEAD', output, Path(temporary) / 'cache')
        except FileExistsError:
            pass
        else:
            raise AssertionError('a staged directory was staged over')


def guards_5(fake_runner, manifest, root, temporary) -> tuple[int, tuple]:
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
        return 1, ()
    py311_profile = root / 'py311' / 'benchmark.toml'
    py311_profile.parent.mkdir(parents=True)
    profile_sources.write_staged_profile(single_profile, py311_profile, registry, profile_sources.parser_configuration('py311'), profile_proofs.selected_rule_applicability(manifest, ['UP006']))
    if tomllib.loads(py311_profile.read_text(encoding='utf-8')).get('parser-configuration') != 'python-target=3.11':
        print('target-specific parser configuration was not staged', file=sys.stderr)
        return 1, ()
    try:
        profile_sources.parser_configuration('py316')
    except ValueError:
        pass
    else:
        print('unsupported Python target version was accepted', file=sys.stderr)
        return 1, ()
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
        return 1, ()
    result_path = durable / 'profiles' / 'mock' / 'result.json'
    result = json.loads(result_path.read_text(encoding='utf-8'))
    if len(calls) != 1 or calls[0][0] != profile_worker.with_case_receipts(command, durable / 'profiles' / 'mock' / 'cases.jsonl') or (not calls[0][1]['capture_output']) or calls[0][1]['text'] or ((durable / 'profiles' / 'mock' / 'stdout.bin').read_bytes() != fake_output.stdout) or ((durable / 'profiles' / 'mock' / 'stderr.bin').read_bytes() != fake_output.stderr) or (result['status'] != 7) or (result['identities']['identity'] != 'sealed') or (result['identities']['worker']['executable']['sha256'] != profile_config.sha256(mock_executable)) or (result['staged_inputs'] != {'benchmark.toml': profile_config.sha256(repro / 'benchmark.toml')}):
        print('durable verifier output or identities were not preserved', file=sys.stderr)
        return 1, ()
    fake_worker = root / 'ruff-fake-worker'
    fake_worker.write_text("#!/usr/bin/env python3\nimport os, pathlib, subprocess, sys\nlabel = 'child' if len(sys.argv) > 1 and sys.argv[1] == 'run-ryo' else 'parent'\npathlib.Path(os.environ['WORKER_PATHS']).open('a').write(label + '|' + sys.argv[0] + '\\n')\nif label == 'parent': subprocess.run([sys.argv[0], 'run-ryo'], check=True)\n", encoding='utf-8')
    fake_worker.chmod(493)
    worker_record = {'worker': {}}
    profile_worker.worker_identity(worker_record, fake_worker)
    if worker_record['worker']['executable'] != {'path': str(fake_worker.resolve()), 'sha256': profile_config.sha256(fake_worker)}:
        print('frozen worker identity was not captured', file=sys.stderr)
        return 1, ()
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
        return 1, ()
    build_record_path = build_root / 'build' / 'result.json'
    original_build_record = build_record_path.read_text(encoding='utf-8')
    scheduled = json.loads(original_build_record)
    scheduled['command'].insert(4, '-j6')
    return 0, (build_calls, build_identity, build_record_path, build_root, durable, frozen_worker, original_build_record, root, scheduled, successful_build_runner)


def guards_6(build_calls, build_identity, build_record_path, build_root, durable, frozen_worker, manifest, original_build_record, root, scheduled, successful_build_runner) -> tuple[int, tuple]:
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
        return 1, ()
    except ValueError as error:
        if 'missing or changed' not in str(error):
            raise
    finally:
        frozen_worker.write_bytes(original_worker_bytes)
    try:
        profile_worker.validate_output_directory(profile_config.REPOSITORY / 'crates' / 'ryo' / 'unsafe-output', resume=False)
        print('new output under the Rust source tree was accepted', file=sys.stderr)
        return 1, ()
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
        return 1, ()
    failed_build_root = root / 'failed-worker-build'
    failed_build_root.mkdir()

    def failed_build_runner(build_command, **_options):
        return subprocess.CompletedProcess(build_command, 101, stdout=b'cargo stdout\n', stderr=b'cargo compiler diagnostic\n')
    try:
        profile_worker.build_worker_once(failed_build_root, contextlib.ExitStack(), runner=failed_build_runner)
        print('failed worker build unexpectedly succeeded', file=sys.stderr)
        return 1, ()
    except ValueError:
        failed_build = json.loads((failed_build_root / 'build/result.json').read_text())
        if failed_build['status'] != 101 or (failed_build_root / 'build/stdout.bin').read_bytes() != b'cargo stdout\n' or (failed_build_root / 'build/stderr.bin').read_bytes() != b'cargo compiler diagnostic\n':
            print('failed worker build diagnostics were not persisted', file=sys.stderr)
            return 1, ()
    source_identity = profile_worker.worker_source_identity
    identities = iter([{'revision': 'before', 'source_tree_sha256': 'before'}, {'revision': 'after', 'source_tree_sha256': 'after'}, {'revision': 'after', 'source_tree_sha256': 'after'}])
    profile_worker.worker_source_identity = lambda: next(identities)
    changed_build_root = root / 'changed-worker-build'
    changed_build_root.mkdir()
    try:
        profile_worker.build_worker_once(changed_build_root, contextlib.ExitStack(), runner=successful_build_runner)
        print('worker source edit during build unexpectedly succeeded', file=sys.stderr)
        return 1, ()
    except ValueError:
        changed_build = json.loads((changed_build_root / 'build/result.json').read_text())
        if changed_build['source_stable_during_build'] or 'sources changed' not in changed_build['error']:
            print('worker source edit during build was not recorded', file=sys.stderr)
            return 1, ()
    finally:
        profile_worker.worker_source_identity = source_identity

    def failing_runner(_command, **_options):
        raise OSError('mock worker could not start')
    error_repro = durable / 'profiles' / 'mock-error' / 'repro'
    error_repro.mkdir(parents=True)
    (error_repro / 'cases.toml').write_text('[[cases]]\n', encoding='utf-8')
    if profile_worker.verify_profile('mock-error', error_repro, command, durable, {}, runner=failing_runner) != 1:
        print('mock verifier startup failure did not return failure', file=sys.stderr)
        return 1, ()
    error_record = json.loads((durable / 'profiles' / 'mock-error' / 'result.json').read_text(encoding='utf-8'))
    if error_record['status'] != 1 or 'mock worker could not start' not in error_record['error'] or (not (durable / error_record['reproduction_directory'] / 'cases.toml').is_file()):
        print('verifier startup exception was not preserved', file=sys.stderr)
        return 1, ()
    profile_source = root / 'profile-source.toml'
    profile_source.write_text('tool = "ruff"\nversion = "0"\nsha256 = ["pin"]\ndefaults = ["F401"]\n[query-modules] # existing but empty\n', encoding='utf-8')
    module_registry = {'python/ruff/suppress': 'rules/F401/query.ryoql'}
    profile_sources.copy_modules(module_registry, root)
    staged_profile = root / 'module-profile.toml'
    profile_sources.write_staged_profile(profile_source, staged_profile, module_registry)
    parsed_profile = tomllib.loads(staged_profile.read_text(encoding='utf-8'))
    if parsed_profile.get('query-modules') != module_registry:
        print('staged query module registry did not remain valid TOML', file=sys.stderr)
        return 1, ()
    staged_module = root / 'rules/F401/query.ryoql'
    if not staged_module.is_file() or staged_module.read_bytes() != (profile_config.RUFF_DIR / module_registry['python/ruff/suppress']).read_bytes():
        print('staged query module bytes changed', file=sys.stderr)
        return 1, ()
    shape_descriptor = manifest['combined_profiles'][profile_config.SHAPE_PROFILE]
    expanded_descriptor = manifest['combined_profiles']['expanded-defaults']
    return 0, (expanded_descriptor, shape_descriptor)


def guards_7(expanded_descriptor, manifest, root, shape_descriptor) -> tuple[int, tuple]:
    if profile_staging.combined_query_filename('LOG015', expanded_descriptor) != 'query.ryoql':
        print('expanded cohort did not retain its sealed LOG015 query', file=sys.stderr)
        return 1, ()
    for rule in ('LOG015', 'ASYNC115'):
        profile_case = profile_sources.prepared_case_path(rule, manifest['profiles'][rule])
        canonical_case = profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml'
        historical_case = profile_staging.combined_case_path(rule, expanded_descriptor)
        if profile_case.read_bytes() != canonical_case.read_bytes() or historical_case.read_bytes() == canonical_case.read_bytes() or expanded_descriptor.get('case_files', {}).get(rule) == canonical_case.relative_to(profile_config.REPOSITORY).as_posix() or (profile_staging.combined_query_filename(rule, expanded_descriptor) != 'query.ryoql'):
            print(f'{rule} candidate snapshot changed the frozen expanded cohort', file=sys.stderr)
            return 1, ()
    f821_record = manifest['profiles']['F821']
    f821_candidate = profile_config.RUFF_DIR / 'rules/F821/query.ryoql'
    f821_cases = profile_sources.prepared_case_path('F821', f821_record)
    historical_candidate = profile_staging.combined_query_path('F821', expanded_descriptor)
    historical_cases = profile_staging.combined_case_path('F821', expanded_descriptor)
    if f821_cases.read_bytes() != (profile_config.RUFF_DIR / 'rules/F821/cases.toml').read_bytes() or historical_candidate == f821_candidate or (historical_cases == f821_cases) or (historical_candidate.read_bytes() == f821_candidate.read_bytes()) or (historical_cases.read_bytes() == f821_cases.read_bytes()):
        print('F821 profile snapshots changed the frozen expanded cohort', file=sys.stderr)
        return 1, ()
    shape_profile = tomllib.loads((profile_config.REPOSITORY / shape_descriptor['profile_file']).read_text(encoding='utf-8'))
    shape_registry = profile_sources.combined_modules(shape_profile, shape_descriptor['selected_rules'], manifest)
    shape_stage = root / 'shape-110-profile'
    shape_stage.mkdir()
    profile_sources.copy_modules(shape_registry, shape_stage)
    profile_sources.write_staged_profile(profile_config.REPOSITORY / shape_descriptor['profile_file'], shape_stage / 'benchmark.toml', shape_registry)
    if shape_registry.get('python/ruff/suppress') != 'fragments/suppress.ryoql' or not (shape_stage / 'fragments/suppress.ryoql').is_file() or tomllib.loads((shape_stage / 'benchmark.toml').read_text(encoding='utf-8')).get('query-modules', {}).get('python/ruff/suppress') != 'fragments/suppress.ryoql':
        print("combined profile omitted an active rule's query module", file=sys.stderr)
        return 1, ()
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
                # The active query has moved on since the normalization; the frozen
                # bytes are the old path at the normalization's source commit.
                restored = subprocess.run(['git', '-C', str(profile_config.REPOSITORY), 'show', f"{normalization['source_commit']}:{relative}"], check=True, capture_output=True).stdout
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
    # Preserve pinned source inputs and raw reference observations separately
    # from certificate-backed candidate expectations in today's case files.
    # Canonical projections of the exact frozen cases at 8720cbc9e1; their
    # original file hashes are recorded in profiles/prepared.json.
    pinned = {
        'FURB132': ('6db1b57d1ae9a7cbb1e85e8dc3923fe577d73612a5769d6c2023c7060faba089',
                    'e6b6f3a34a88d01328f5fecac2ad4009af650084d9b94753c4f0a29526cc21ee'),
        'PERF402': ('89bdbecd82e5ed5fdd985da8f55c2c654de934efb9100262d36e251206143535',
                    '9d4c3b8b410bdde7ab05845ae7cb74dd389b44d91bb758a1bc7fec5b269fa24a'),
    }
    factory_controls = ['called-annotated-factory-typed-instance', 'called-annotated-factory-uninferred']
    lambda_controls = ['lambda-phase-body-call', 'lambda-phase-body-subscript',
                       'lambda-phase-default-call', 'lambda-phase-keyword-default-subscript',
                       'lambda-phase-nested-deferred-default', 'lambda-phase-nested-eager-default',
                       'lambda-phase-nested-body-root']
    return 0, (factory_controls, helper_relative, lambda_controls, pinned, relative, required)
