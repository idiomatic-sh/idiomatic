"""Verification cli: linter-specific authoring and proof utilities."""
from __future__ import annotations
import argparse
import contextlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
import tomllib
from authoring import case_observations
from authoring import profile_cli_tests
from authoring import profile_config
from authoring import profile_proof_tests
from authoring import profile_proofs
from authoring import native_standalone
from authoring import profile_shards
from authoring import profile_staging
from authoring import profile_worker
from authoring import rule_registration

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ruff', type=Path, nargs='?', help='pinned Ruff executable')
    parser.add_argument('rules', nargs='*', help='optional rule IDs in the 413-rule inventory; use --prepared for historical authoring profiles')
    parser.add_argument('--profile', help='run one named combined prepared profile')
    parser.add_argument('--full-inventory', action='store_true', help='enumerate all 413 pinned defaults; strict failure unless every rule passes')
    parser.add_argument('--prepared', action='store_true', help='use the historical prepared-profile authoring manifest')
    parser.add_argument('--prepare-inventory', action='store_true', help='materialize registered case contexts without executing them; shared certification owns execution')
    parser.add_argument('--worker', type=Path, help='existing sealed worker for --full-inventory; never builds a worker')
    parser.add_argument('--inventory-runner', type=Path, help='existing shared benchmark collector executable')
    parser.add_argument('--rule-timeout', type=float, default=120.0, help='per-rule full-inventory timeout in seconds (default: 120)')
    parser.add_argument('--native-standalone', action='store_true', help='execute every explicitly declared native query/case profile separately; never the combined cohort')
    parser.add_argument('--self-test-native-standalone', action='store_true', help='exercise native inventory/selection/identity guards without building or running contestants')
    parser.add_argument('--shards', type=int, help='execute each case once across deterministic case shards')
    parser.add_argument('--workers', type=int, help='bound concurrent shard workers (1 through 64); scheduling may change on resume')
    parser.add_argument('--resume', action='store_true', help='reuse only complete shard receipts with identical sealed identities')
    parser.add_argument('--reuse-worker-from', type=Path, help='reuse a frozen benchmark worker and its original stable build provenance')
    parser.add_argument('--parity-check', help='single-rule profile to compare monolithic and two-shard full reports')
    parser.add_argument('--output-dir', type=Path, help='persist staged repro inputs, identities, exit status, stdout and stderr')
    parser.add_argument('--reference-only', action='store_true', help='check sealed expected findings with Ruff only')
    parser.add_argument('--self-test', action='store_true', help='exercise fail-closed profile guards without running Cargo')
    parser.add_argument('--stage-request', type=Path, metavar='PINNED_REQUEST', help='stage rules, fragments and benchmark.toml from --source into a fresh --output-dir and build a run-ryo request over the corpus of PINNED_REQUEST; prints the staged digest')
    parser.add_argument('--source', default='HEAD', help='the measured source commit --stage-request stages from (default: HEAD)')
    parser.add_argument('--cache', type=Path, help='the store directory the staged request names (default: <output-dir>/cache)')
    arguments = parser.parse_args()
    if arguments.stage_request is not None:
        if arguments.output_dir is None or arguments.output_dir.exists():
            parser.error('--stage-request requires a fresh --output-dir')
        output_directory = arguments.output_dir.resolve()
        receipt = profile_staging.stage_measurement_request(arguments.stage_request.resolve(strict=True), arguments.source, output_directory, (arguments.cache or output_directory / 'cache').resolve())
        print(json.dumps(receipt, indent=2))
        return 0
    inventory_default = not (arguments.prepared or arguments.profile or arguments.native_standalone or arguments.self_test or arguments.self_test_native_standalone)
    if arguments.full_inventory or inventory_default:
        if arguments.ruff is None or arguments.output_dir is None or (arguments.worker is not None and arguments.reuse_worker_from is not None) or (arguments.rule_timeout <= 0) or arguments.profile or arguments.native_standalone or arguments.reference_only or arguments.resume or (arguments.shards is not None) or (arguments.workers is not None) or arguments.parity_check or arguments.prepared or (arguments.full_inventory and arguments.rules):
            parser.error('inventory verification requires Ruff, fresh output-dir and positive timeout; use --prepared or --profile for historical authoring flags; --full-inventory accepts no rule subset')
        return rule_registration.verify_full_inventory(arguments)
    if arguments.worker is not None:
        parser.error('--worker is supported only with inventory verification')
    if arguments.inventory_runner is not None:
        parser.error('--inventory-runner is supported only with inventory verification')
    if arguments.self_test_native_standalone:
        profile_proof_tests.native_standalone_self_test()
        return 0
    if arguments.self_test:
        return profile_cli_tests.self_test()
    if arguments.native_standalone and (arguments.profile or arguments.rules or arguments.resume or arguments.parity_check):
        parser.error('native standalone verification requires the complete declared inventory, separate from combined/resume selections')
    if arguments.ruff is None:
        parser.error('the pinned Ruff executable is required unless a self-test mode is selected')
    if arguments.reuse_worker_from is not None and (arguments.resume or arguments.reference_only):
        parser.error('--reuse-worker-from requires a new candidate verification run')
    if arguments.shards is not None and (arguments.shards < 2 or arguments.reference_only):
        parser.error('--shards requires at least two shards and candidate verification')
    if arguments.workers is not None and (arguments.shards is None or not 1 <= arguments.workers <= 64):
        parser.error('--workers requires --shards and a value from 1 through 64')
    if arguments.resume and (arguments.shards is None or arguments.output_dir is None):
        parser.error('--resume requires --shards and --output-dir')
    if arguments.resume and arguments.profile is None:
        parser.error('--resume requires the exact prior --profile selection')
    if arguments.resume and arguments.rules:
        parser.error('--resume does not accept a changed rule selection')
    if not arguments.reference_only and arguments.output_dir is None:
        parser.error('candidate verification requires --output-dir for exact case receipts')
    output_directory = arguments.output_dir.resolve() if arguments.output_dir else None
    if arguments.parity_check is not None and (output_directory is None or arguments.shards is None or arguments.resume or (arguments.parity_check not in arguments.rules) or (arguments.profile is None and len(arguments.rules) != 1)):
        parser.error('--parity-check requires output-dir, shards, and exactly one selected rule (plus a combined profile when specified)')
    if output_directory is not None:
        if arguments.resume:
            if not output_directory.is_dir():
                parser.error('resume output directory does not exist')
        else:
            try:
                profile_worker.create_new_output_directory(output_directory)
            except (ValueError, FileExistsError) as error:
                parser.error(str(error))
            shutil.copyfile(profile_config.MANIFEST, output_directory / 'prepared.json')
    try:
        manifest = profile_worker.resume_manifest(output_directory, arguments.profile) if arguments.resume and output_directory is not None and (arguments.profile is not None) else json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    helper_path = Path(profile_config.ENTRYPOINT).relative_to(profile_config.REPOSITORY).as_posix()
    statuses = []
    try:
        if not arguments.resume:
            profile_proofs.verify_files(manifest, helper_path)
        if arguments.native_standalone:
            manifest = native_standalone.native_standalone_manifest(manifest)
            if output_directory is not None:
                shutil.copyfile(profile_config.NATIVE_DESCRIPTOR_FILE, output_directory / 'native-standalone.json')
                profile_config.atomic_json_write(output_directory / 'native-execution.json', {'identity': manifest['native_identity'], 'profiles': manifest['profiles'], 'limits': 'New explicit singleton execution contract; no historical correctness certification; combined cohorts are separate.'})
        if not arguments.native_standalone and arguments.profile is None and (not arguments.rules) and (not arguments.resume):
            profile_proofs.verify_active_combined_cohort(manifest)
        if arguments.profile is not None:
            if arguments.profile not in manifest.get('combined_profiles', {}):
                raise ValueError(f'unknown prepared combined profile: {arguments.profile}')
            if arguments.resume:
                profile_proofs.verify_active_combined_cohort(manifest)
            defaults = set(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))['defaults'])
            if len(set(arguments.rules)) != len(arguments.rules) or set(arguments.rules) - defaults:
                raise ValueError('unknown or duplicate profile rule')
            selections = [arguments.profile]
        else:
            profiles = manifest['profiles']
            defaults = set(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))['defaults'])
            if len(set(arguments.rules)) != len(arguments.rules) or set(arguments.rules) - defaults:
                raise ValueError('unknown or duplicate profile rule')
            selections = [profile_name for profile_name, profile in sorted(profiles.items()) if not arguments.rules or profile.get('rule_id', profile_name) in arguments.rules]
            if not selections:
                raise ValueError('selected rules have no prepared candidate profiles')
        ruff = arguments.ruff.resolve()
        with contextlib.ExitStack() as stack:
            staged = []
            if arguments.profile is not None:
                descriptor = manifest['combined_profiles'][arguments.profile]
                if arguments.resume:
                    staged.append((arguments.profile, output_directory / 'profiles' / arguments.profile / 'repro'))
                else:
                    if arguments.rules:
                        single_profiles = [profile_name for profile_name, profile in sorted(manifest['profiles'].items()) if profile.get('rule_id', profile_name) in arguments.rules]
                        staged.extend(profile_staging.stage_binding(manifest, single_profiles, stack, output_directory))
                    staged.append(profile_staging.stage_combined(arguments.profile, descriptor, manifest, stack, output_directory))
            else:
                staged.extend(profile_staging.stage_binding(manifest, selections, stack, output_directory))
                if not arguments.rules and (not arguments.native_standalone):
                    for name, descriptor in sorted(manifest.get('combined_profiles', {}).items()):
                        if descriptor.get('validation_status') != 'active-current':
                            continue
                        staged.append(profile_staging.stage_combined(name, descriptor, manifest, stack, output_directory))
            if arguments.resume:
                if output_directory is None:
                    raise ValueError('resume requires an output directory')
                worker_executable, build_identity = profile_worker.reuse_frozen_worker(output_directory)
            elif arguments.reference_only:
                worker_executable = None
                build_identity = {'status': 'not-built', 'reason': 'reference-only mode invokes the sealed Ruff executable directly'}
            elif arguments.reuse_worker_from is not None:
                original = arguments.reuse_worker_from.resolve()
                worker_executable, build_identity = profile_worker.reuse_frozen_worker(original)
                build_directory = output_directory / 'build'
                build_directory.mkdir(exist_ok=False)
                for filename in ('stdout.bin', 'stderr.bin', 'result.json'):
                    shutil.copyfile(original / 'build' / filename, build_directory / filename)
                profile_config.atomic_json_write(output_directory / 'worker-reuse.json', {'original_output_directory': str(original), 'original_build_receipt_sha256': profile_config.sha256(original / 'build' / 'result.json'), 'worker_sha256': profile_config.sha256(worker_executable), 'source_identity_policy': 'original stable build identity; current source tree is not relabeled'})
            else:
                worker_executable, build_identity = profile_worker.build_worker_once(output_directory, stack)
            if not arguments.reference_only:
                assert worker_executable is not None
                protocol_ok, protocol_message = profile_worker.audit_worker_protocol(worker_executable)
                protocol_record = {'command': [str(worker_executable), 'audit'], 'worker_sha256': profile_config.sha256(worker_executable), 'status': 0 if protocol_ok else 1, 'message': protocol_message}
                build_identity['protocol_smoke'] = protocol_record
                if not protocol_ok:
                    if output_directory is not None:
                        profile_config.atomic_json_write(output_directory / 'worker-protocol.json', protocol_record)
                    print(f'worker protocol smoke failed: {protocol_message}', file=sys.stderr)
                    return 1
            if not arguments.reference_only:
                guard_script = profile_config.REPOSITORY / 'crates/ryo/languages/python/tests/fixtures/ryoql-fragments/sparse-traversal-guards/run.py'
                subprocess.run([sys.executable, str(guard_script), '--self-test'], check=True)
                guard_root = Path(tempfile.mkdtemp(prefix='sparse-traversal-', dir=output_directory))
                subprocess.run([sys.executable, str(guard_script), '--candidate-root', str(profile_config.REPOSITORY), '--worker', str(worker_executable), '--output', str(guard_root / 'proof')], check=True)
            for name, profile_root in staged:
                if arguments.reference_only:
                    ref_identity = profile_worker.reference_tool_identity(ruff, manifest)
                    identities = {'manifest_path': profile_config.MANIFEST.relative_to(profile_config.REPOSITORY).as_posix(), 'manifest_sha256': profile_config.sha256(profile_config.MANIFEST), 'target_version': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('parser-configuration', 'python-target=3.10'), 'pinned_reference': ref_identity, 'staged_profile_sha256': profile_worker.staged_hashes(profile_root), 'selected_rule_applicability': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('rule-applicability', {})}
                    if arguments.native_standalone:
                        identities['native_standalone'] = {**manifest['native_identity'], 'profile_name': name, 'effective_settings': manifest['profiles'][name]['effective_settings']}
                    status = profile_worker.verify_reference_profile(name, profile_root, ruff, output_directory, identities)
                    print(f'{name}: verifier exit status {status}', file=sys.stderr)
                    statuses.append((name, status))
                    continue
                assert worker_executable is not None
                executable = build_identity['frozen_executable']
                if not isinstance(executable, dict) or profile_config.sha256(Path(executable['path'])) != executable['sha256']:
                    raise ValueError('frozen worker executable hash changed before profile invocation')
                command = profile_worker.command_for(worker_executable, ruff, profile_root, False)
                if arguments.resume:
                    prior_plan_path = output_directory / 'profiles' / name / 'shard-plan.json'
                    prior_plan = json.loads(prior_plan_path.read_text(encoding='utf-8'))
                    identities = prior_plan['global_identity']['selection']
                    reference_identity = profile_worker.reference_tool_identity(ruff, manifest)
                    if identities.get('manifest_sha256') != profile_config.sha256(output_directory / 'prepared.json') or identities.get('ruff', {}).get('sha256') != reference_identity['sha256']:
                        raise ValueError('cannot resume with a changed manifest or pinned reference executable')
                else:
                    identities = profile_worker.run_identity(manifest, ruff, profile_root, worker_executable, build_identity)
                if arguments.native_standalone:
                    identities['native_standalone'] = {**manifest['native_identity'], 'profile_name': name, 'effective_settings': manifest['profiles'][name]['effective_settings']}
                profile_worker.worker_identity(identities, worker_executable)
                if arguments.shards is not None and (arguments.profile is not None and name == arguments.profile or (arguments.profile is None and arguments.parity_check is None)):
                    if output_directory is None:
                        raise ValueError('sharded verification requires --output-dir for durable receipts')
                    status = profile_shards.verify_profile_sharded(name, profile_root, worker_executable, ruff, output_directory, identities, arguments.shards, resume=arguments.resume, worker_limit=arguments.workers)
                else:
                    status = profile_worker.verify_profile(name, profile_root, command, output_directory, identities)
                print(f'{name}: verifier exit status {status}', file=sys.stderr)
                statuses.append((name, status))
            if arguments.parity_check is not None:
                parity_name = arguments.parity_check
                parity_root = output_directory / 'profiles' / parity_name / 'repro'
                parity_identity = profile_worker.run_identity(manifest, ruff, parity_root, worker_executable, build_identity)
                profile_worker.worker_identity(parity_identity, worker_executable)
                shard_status = profile_shards.verify_profile_sharded(parity_name, parity_root, worker_executable, ruff, output_directory, parity_identity, 2)
                shard_paths = sorted((output_directory / 'profiles' / parity_name / 'shards').glob('*.cases.jsonl'))
                mono_path = output_directory / 'profiles' / parity_name / 'cases.jsonl'
                observations_match = case_observations.case_observations_match(mono_path, shard_paths)
                parity_status = 0 if dict(statuses).get(parity_name) == 0 and shard_status == 0 and observations_match else 1
                profile_config.atomic_json_write(output_directory / 'profiles' / parity_name / 'parity.json', {'profile': parity_name, 'status': parity_status, 'worker_sha256': profile_config.sha256(worker_executable), 'monolithic_case_results_sha256': profile_config.sha256(mono_path) if mono_path.is_file() else None, 'sharded_case_result_shas': {path.name: profile_config.sha256(path) for path in shard_paths}, 'full_report_observations_match': observations_match, 'rule_profile_status': dict(statuses).get(parity_name), 'sharded_status': shard_status})
                print(f'{parity_name}: monolithic/sharded full-report parity {parity_status}', file=sys.stderr)
                statuses.append((f'{parity_name}-shard-parity', parity_status))
    except (KeyError, TypeError, ValueError, OSError) as error:
        if output_directory is not None:
            (output_directory / 'preflight-error.json').write_text(json.dumps({'error': f'{type(error).__name__}: {error}', 'manifest_path': profile_config.MANIFEST.relative_to(profile_config.REPOSITORY).as_posix(), 'manifest_sha256': profile_config.sha256(profile_config.MANIFEST), 'arguments': sys.argv[1:]}, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        parser.error(str(error))
    failed = [rule for rule, status in statuses if status != 0]
    if failed:
        print(f"candidate profiles failed: {', '.join(failed)}", file=sys.stderr)
    if output_directory is not None:
        (output_directory / 'run.json').write_text(json.dumps({'manifest_path': profile_config.MANIFEST.relative_to(profile_config.REPOSITORY).as_posix(), 'manifest_sha256': profile_config.sha256(profile_config.MANIFEST), 'target_version': manifest.get('target_version'), 'ruff_argument': str(arguments.ruff), 'reference_version': manifest.get('reference_version'), 'reference_sha256': manifest.get('reference_sha256'), 'worker_build': build_identity, 'statuses': dict(statuses), 'aggregate_status': 1 if failed else 0}, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    return 1 if failed else 0
