"""Profile worker: linter-specific authoring and proof utilities."""
from __future__ import annotations
import contextlib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import profile_config
from authoring import profile_sources

def run_ryo_command(worker: Path | str, request: dict[str, object], wire: str | None=None) -> list[str]:
    """Pass the explicit parser setting required by the direct worker protocol."""
    configuration = request.get('parser_configuration')
    if configuration not in profile_config.PARSER_CONFIGURATIONS.values():
        raise ValueError(f'unsupported direct worker parser configuration: {configuration}')
    if wire is None:
        wire = json.dumps(request)
    elif json.loads(wire) != request:
        raise ValueError('direct worker request bytes disagree with the request')
    return [str(worker), 'run-ryo', wire, configuration]

def command_for(worker: Path, ruff: Path, profile_root: Path, reference_only: bool=False, shard: tuple[int, int] | None=None, case_selection: Path | None=None) -> list[str]:
    """Invoke the one immutable benchmark executable built for this profile batch."""
    command = [str(worker), 'verify-reference-cases' if reference_only else 'verify-cases', str(ruff), str(profile_root)]
    if shard is not None:
        command.append(f'--shard={shard[0]}/{shard[1]}')
    if case_selection is not None:
        command.append(f'--case-selection={case_selection}')
    return command

def audit_worker_protocol(worker: Path, *, runner=subprocess.run) -> tuple[bool, str]:
    """Confirm the executable is the benchmark CLI, not a same-named Rust test harness."""
    command = [str(worker), 'audit']
    try:
        result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', capture_output=True, check=False)
    except Exception as error:
        return (False, f'worker protocol smoke could not start: {type(error).__name__}: {error}')
    if result.returncode != 0:
        detail = result.stderr.decode('utf-8', errors='replace').strip()[:2048]
        return (False, f'worker protocol smoke exited {result.returncode}: {detail}')
    expected = re.fullmatch(b'ruff 0\\.16\\.5: [0-9]+ queries authored / 413 shipped defaults \\(not a parity certificate\\)\\n', result.stdout)
    if expected is None:
        return (False, f'worker protocol smoke returned an unexpected response: {result.stdout!r}')
    return (True, result.stdout.decode('utf-8').strip())

def cargo_worker_build_command() -> list[str]:
    """Build the benchmark once and request machine-readable artifact paths."""
    return ['cargo', 'bench', '--no-run', '--locked', '-p', 'idiomatic-sh-ryo-python', '--bench', 'ruff', '--message-format=json']

def parse_worker_artifact(stdout: bytes) -> Path:
    """Read Cargo JSON output and return the ruff bench executable."""
    matches = []
    for line in stdout.splitlines():
        try:
            message = json.loads(line)
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        target = message.get('target', {})
        source_path = target.get('src_path', '')
        normalized_source = source_path.replace('\\', '/') if isinstance(source_path, str) else ''
        if message.get('reason') == 'compiler-artifact' and target.get('name') == 'ruff' and ('bench' in target.get('kind', [])) and (target.get('test') is False) and normalized_source.endswith('/languages/python/benchmarks/linters/ruff/bench.rs') and isinstance(message.get('executable'), str):
            matches.append(Path(message['executable']))
    if len(matches) != 1:
        raise ValueError(f'Cargo JSON reported {len(matches)} ruff benchmark executables')
    return matches[0].resolve(strict=True)

def worker_source_identity() -> dict[str, object]:
    """Capture the Ryo source identity needed to detect edits during the build."""
    try:
        revision = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout.decode().strip()
        diff = subprocess.run(['git', 'diff', '--binary', 'HEAD', '--', 'crates/ryo'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout
        status = subprocess.run(['git', 'status', '--porcelain', '--untracked-files=all', '--', 'crates/ryo'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout.decode().splitlines()
        source_paths = subprocess.run(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', 'crates/ryo'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout.split(b'\x00')
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError(f'could not identify Ryo source before worker build: {error}') from error
    source_digest = hashlib.sha256()
    for raw_path in sorted((path for path in source_paths if path)):
        relative = Path(os.fsdecode(raw_path))
        path = profile_config.REPOSITORY / relative
        if path.is_symlink():
            content = os.fsencode(os.readlink(path))
        elif path.is_file():
            content = path.read_bytes()
        else:
            continue
        source_digest.update(len(raw_path).to_bytes(8, 'big'))
        source_digest.update(raw_path)
        source_digest.update(len(content).to_bytes(8, 'big'))
        source_digest.update(content)
    return {'repository_revision': revision, 'working_tree_status': status, 'working_tree_diff_sha256': hashlib.sha256(diff).hexdigest(), 'source_tree_sha256': source_digest.hexdigest()}

def build_worker_once(output_directory: Path | None, stack: contextlib.ExitStack, *, runner=subprocess.run) -> tuple[Path, dict[str, object]]:
    """Build, identify and freeze one executable before any profile is run."""
    command = cargo_worker_build_command()
    before = worker_source_identity()
    environment = {**os.environ, 'CARGO_BUILD_JOBS': '6'}
    error = None
    result = None
    try:
        result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', env=environment, capture_output=True, text=False, check=False)
        if result.returncode != 0:
            raise ValueError(f'Cargo benchmark build exited with status {result.returncode}')
        artifact = parse_worker_artifact(result.stdout)
        after = worker_source_identity()
        if before != after:
            raise ValueError('Ryo sources changed while building the benchmark executable')
        if output_directory is None:
            worker_directory = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix='ryo-ruff-worker-')))
        else:
            worker_directory = output_directory / 'worker'
            worker_directory.mkdir(parents=True, exist_ok=False)
        worker_copy = worker_directory / f'ruff-{profile_config.sha256(artifact)}'
        shutil.copy2(artifact, worker_copy)
        identities = {'command': command, 'status': result.returncode, 'source_before_build': before, 'source_after_build': after, 'source_stable_during_build': True, 'cargo_artifact': str(artifact), 'frozen_executable': {'path': str(worker_copy.resolve()), 'sha256': profile_config.sha256(worker_copy)}}
    except Exception as caught:
        error = f'{type(caught).__name__}: {caught}'
        after = worker_source_identity()
        identities = {'command': command, 'status': result.returncode if result is not None else 1, 'source_before_build': before, 'source_after_build': after, 'source_stable_during_build': before == after, 'error': error}
    if output_directory is not None:
        build_directory = output_directory / 'build'
        build_directory.mkdir(exist_ok=True)
        (build_directory / 'stdout.bin').write_bytes(result.stdout if result is not None else b'')
        (build_directory / 'stderr.bin').write_bytes(result.stderr if result is not None else b'')
        (build_directory / 'result.json').write_text(json.dumps(identities, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    if error is not None:
        raise ValueError(error)
    return (worker_copy, identities)

def reuse_frozen_worker(output_directory: Path) -> tuple[Path, dict[str, object]]:
    """Resume with the executable whose stable build provenance was saved."""
    record_path = output_directory / 'build' / 'result.json'
    record = json.loads(record_path.read_text(encoding='utf-8'))
    frozen = record.get('frozen_executable')
    if record.get('status') != 0 or record.get('source_stable_during_build') is not True or record.get('source_before_build') != record.get('source_after_build') or (not isinstance(frozen, dict)) or (not isinstance(frozen.get('path'), str)) or (not isinstance(frozen.get('sha256'), str)):
        raise ValueError('cannot resume: frozen worker build provenance is invalid')
    executable = Path(frozen['path'])
    if not executable.is_file() or profile_config.sha256(executable) != frozen['sha256']:
        raise ValueError('cannot resume: frozen worker executable is missing or changed')
    build_stdout = output_directory / 'build' / 'stdout.bin'
    try:
        artifact = parse_worker_artifact(build_stdout.read_bytes())
    except (OSError, ValueError) as error:
        raise ValueError(f'cannot resume: saved Cargo artifact record is invalid: {error}') from error
    command = record.get('command')
    expected = cargo_worker_build_command()
    six_jobs = expected[:4] + ['-j6'] + expected[4:]
    if record.get('cargo_artifact') != str(artifact) or command not in (expected, six_jobs):
        raise ValueError('cannot resume: saved Cargo artifact is not the Ruff benchmark target')
    return (executable, record)

def resume_manifest(output_directory: Path, profile_name: str) -> dict[str, object]:
    """Load the immutable manifest snapshot bound to a prior shard plan."""
    prepared_path = output_directory / 'prepared.json'
    plan_path = output_directory / 'profiles' / profile_name / 'shard-plan.json'
    try:
        manifest_bytes = prepared_path.read_bytes()
        manifest = json.loads(manifest_bytes)
        plan = json.loads(plan_path.read_text(encoding='utf-8'))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f'cannot resume: saved manifest or shard plan is unreadable: {error}') from error
    selection = plan.get('global_identity', {}).get('selection', {})
    if plan.get('profile') != profile_name or selection.get('manifest_sha256') != hashlib.sha256(manifest_bytes).hexdigest() or profile_name not in manifest.get('combined_profiles', {}) or (manifest['combined_profiles'][profile_name].get('validation_status') != 'active-current') or (selection.get('sealed_source_hashes') != manifest.get('files')):
        raise ValueError('cannot resume: saved manifest does not match the sealed active profile plan')
    repro = output_directory / 'profiles' / profile_name / 'repro'
    if not repro.is_dir() or plan.get('staged_inputs') != staged_hashes(repro):
        raise ValueError('cannot resume: staged profile inputs differ from the saved shard plan')
    profile_data = tomllib.loads((repro / 'benchmark.toml').read_text(encoding='utf-8'))
    if profile_data.get('defaults') != manifest['combined_profiles'][profile_name].get('selected_rules'):
        raise ValueError('cannot resume: profile rule selection differs from the saved manifest')
    if selection.get('target_version') != profile_data.get('parser-configuration', 'python-target=3.10'):
        raise ValueError('cannot resume: parser target differs from the saved run identity')
    if selection.get('selected_rule_applicability') != profile_data.get('rule-applicability', {}):
        raise ValueError('cannot resume: rule applicability differs from the saved run identity')
    return manifest

def validate_output_directory(output_directory: Path, *, resume: bool) -> None:
    """Keep new verifier receipts outside the Rust source identity being built."""
    rust_root = (profile_config.REPOSITORY / 'crates' / 'ryo').resolve()
    resolved = output_directory.resolve()
    if not resume and resolved.is_relative_to(rust_root):
        raise ValueError('new candidate output directories must be outside crates/ryo')

def create_new_output_directory(output_directory: Path) -> None:
    """Refuse an existing raw receipt root before staging or running a worker."""
    validate_output_directory(output_directory, resume=False)
    output_directory.mkdir(parents=True, exist_ok=False)

def staged_hashes(root: Path) -> dict[str, str]:
    return {path.relative_to(root).as_posix(): profile_config.sha256(path) for path in sorted(root.rglob('*')) if path.is_file()}

def prepare_profile_record_directory(output_directory: Path, name: str, artifacts: tuple[str, ...]) -> Path:
    """Create the worker's receipt parent and refuse an existing raw run."""
    record_directory = output_directory / 'profiles' / name
    record_directory.mkdir(parents=True, exist_ok=True)
    for artifact in artifacts:
        if (record_directory / artifact).exists():
            raise FileExistsError(f'existing profile receipt: {record_directory / artifact}')
    return record_directory

def persist_run_record(output_directory: Path | None, name: str, command: list[str], profile_root: Path, result: subprocess.CompletedProcess[bytes] | None, *, error: str | None=None, identities: dict[str, object] | None=None, case_results_path: Path | None=None, status_override: int | None=None) -> int:
    """Persist raw command output and exact identities before temporary inputs can be removed."""
    process_status = result.returncode if result is not None else 1
    status = status_override if status_override is not None else process_status
    if output_directory is None:
        if result is not None:
            sys.stderr.buffer.write(result.stderr)
            sys.stdout.buffer.write(result.stdout)
        if error is not None:
            print(f'{name}: could not start verifier: {error}', file=sys.stderr)
        return status
    record_directory = output_directory / 'profiles' / name
    record_directory.mkdir(parents=True, exist_ok=True)
    (record_directory / 'stdout.bin').write_bytes(result.stdout if result is not None else b'')
    (record_directory / 'stderr.bin').write_bytes(result.stderr if result is not None else b'')
    record = {'comparison_schema': 'benchmark-comparison/v1', 'expectation_contract': 'diagnostics', 'raw_count_contract': 'legacy-stored-field-equality/v1', 'execution_metadata_contract': 'explicit-capability/v1; legacy absence remains unknown', 'profile': name, 'command': command, 'cwd': str(profile_config.REPOSITORY / 'crates' / 'ryo'), 'status': status, 'process_status': process_status, 'error': error, 'identities': identities or {}, 'staged_inputs': staged_hashes(profile_root) if profile_root.is_dir() else {}, 'stdout_file': f'profiles/{name}/stdout.bin', 'stderr_file': f'profiles/{name}/stderr.bin', 'reproduction_directory': f'profiles/{name}/repro', 'case_results_file': f'profiles/{name}/cases.jsonl' if case_results_path is not None and case_results_path.is_file() else None, 'case_results_sha256': profile_config.sha256(case_results_path) if case_results_path is not None and case_results_path.is_file() else None}
    if case_results_path is not None and case_results_path.is_file():
        try:
            observed = [json.loads(line) for line in case_results_path.read_text().splitlines()]
            record['case_expectation_summary'] = case_records.case_expectation_summary(case_records.profile_case_inventory(profile_root), observed, receipt_file=case_results_path)
        except (OSError, ValueError, KeyError, TypeError) as summary_error:
            record['case_expectation_summary_error'] = str(summary_error)
    (record_directory / 'result.json').write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    return status

def with_case_receipts(command: list[str], destination: Path) -> list[str]:
    """Pass the shared inventory verifier's explicit receipt destination, without environment state."""
    return [*command, f'--case-receipts={destination}']

def verify_profile(name: str, profile_root: Path, command: list[str], output_directory: Path | None, identities: dict[str, object], *, runner=subprocess.run) -> int:
    case_files = (profile_root / 'rules').glob('*/cases.toml')
    if any((any(('reference-discrepancy' in case for case in tomllib.loads(path.read_text(encoding='utf-8')).get('cases', []))) for path in case_files)):
        try:
            profile_sources.assert_staged_profile_fixed_point(profile_root)
        except (OSError, ValueError, TypeError) as error:
            return persist_run_record(output_directory, name, command, profile_root, None, error=f'authoring profile preflight: {error}', identities=identities)
    case_results_path = None
    environment = os.environ.copy()
    if output_directory is not None and command[1] != 'verify-reference-cases':
        record_directory = prepare_profile_record_directory(output_directory, name, ('cases.jsonl', 'stdout.bin', 'stderr.bin', 'result.json'))
        case_results_path = record_directory / 'cases.jsonl'
        command = with_case_receipts(command, case_results_path)
    try:
        result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', capture_output=True, text=False, check=False, env=environment)
    except Exception as error:
        return persist_run_record(output_directory, name, command, profile_root, None, error=f'{type(error).__name__}: {error}', identities=identities, case_results_path=case_results_path)
    validation_error = None
    status_override = None
    if command[1] == 'verify-cases':
        if output_directory is None or case_results_path is None:
            validation_error = 'candidate verification requires durable output and exact case receipts'
            status_override = 1
        elif result.returncode == 0:
            valid, validation_error = case_records.candidate_result_valid(profile_root, result.stdout, case_results_path)
            if not valid:
                status_override = 1
    return persist_run_record(output_directory, name, command, profile_root, result, identities=identities, case_results_path=case_results_path, error=validation_error, status_override=status_override)

def verify_reference_profile(name: str, profile_root: Path, ruff: Path, output_directory: Path | None, identities: dict[str, object], *, runner=subprocess.run) -> int:
    """Check pinned full-report fixtures directly with Ruff, without building Ryo."""
    profile = tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8'))
    if profile.get('file-observation') is not None:
        raise ValueError('reference-only stdin transport cannot verify file observations; use the sealed native file-mode protocol')
    if profile.get('project-context-mode', 'isolated') != 'isolated':
        raise ValueError('reference-only stdin transport cannot verify ListedDisk context; use the sealed native verify-reference-cases protocol')
    target_config = profile.get('parser-configuration')
    targets = {configuration: version for version, configuration in profile_config.PARSER_CONFIGURATIONS.items()}
    target = targets.get(target_config) if isinstance(target_config, str) else None
    if target is None:
        raise ValueError(f'unsupported reference-only parser configuration: {target_config}')
    rules = profile.get('defaults')
    if not isinstance(rules, list) or not rules or any((not isinstance(rule, str) for rule in rules)):
        raise ValueError('reference-only profile must have a nonempty defaults list')
    cases_path = profile_root / 'rules' / rules[0] / 'cases.toml'
    cases = tomllib.loads(cases_path.read_text(encoding='utf-8')).get('cases')
    if not isinstance(cases, list) or not cases:
        raise ValueError('reference-only profile has no canonical cases')
    record_directory = None
    if output_directory is not None:
        record_directory = prepare_profile_record_directory(output_directory, name, ('reference-cases.jsonl', 'stdout.bin', 'stderr.bin', 'result.json'))
    rows = []
    failures = []
    command_log = []
    for case in cases:
        if 'file-metadata' in case:
            raise ValueError('reference-only stdin transport cannot observe declared file metadata; use file mode')
        if case.get('reference-discrepancy') is not None:
            raise ValueError('reference discrepancy cases require native verify-reference-cases evidence enforcement')
        actual = []
        process_records = []
        for file, source in sorted(case['files'].items()):
            command = [str(ruff), 'check', '--isolated', '--no-cache', '--output-format', 'json', '--select', ','.join(rules), '--target-version', target, '--stdin-filename', file, '-']
            command_log.append(command)
            try:
                result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', input=source.encode('utf-8'), capture_output=True, check=False)
                if result.returncode not in {0, 1}:
                    raise ValueError(f'Ruff exited {result.returncode}: {result.stderr!r}')
                findings = json.loads(result.stdout)
                if not isinstance(findings, list):
                    raise ValueError('Ruff JSON response was not an array')
                for finding in findings:
                    start = case_records.ruff_location_to_byte(source, finding['location'])
                    end_location = finding.get('end_location')
                    if not isinstance(end_location, dict):
                        raise ValueError('Ruff finding is missing end_location')
                    end = case_records.ruff_location_to_byte(source, end_location)
                    if start > end:
                        raise ValueError('Ruff returned a reversed source span')
                    actual.append({'rule': finding['code'], 'file': file, 'start': start, 'end': end})
                process_records.append({'file': file, 'status': result.returncode, 'stdout': result.stdout.decode('utf-8'), 'stderr': result.stderr.decode('utf-8')})
            except (OSError, UnicodeDecodeError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
                failures.append(f"{case['name']}/{file}: {error}")
                process_records.append({'file': file, 'error': f'{type(error).__name__}: {error}'})
        actual.sort(key=lambda row: (row['rule'], row['file'], row['start'], row['end']))
        expected = sorted(({'rule': item.get('rule', rules[0]), 'file': item['file'], 'start': item['start'], 'end': item['end']} for item in case.get('expected', [])), key=lambda row: (row['rule'], row['file'], row['start'], row['end']))
        case_passed = actual == expected
        if not case_passed:
            failures.append(f"{case['name']}: expected {expected!r}; observed {actual!r}")
        rows.append({'rule': rules[0], 'name': case['name'], 'side': 'reference', 'complete': not any((record.get('error') for record in process_records)), 'case_passed': case_passed, 'expected': expected, 'actual': actual, 'source_sha256': case_records.source_hash(case['files']), 'files': sorted(case['files']), 'commands': process_records})
    summary = f'{len(cases)} cases matched the reference; no candidate was executed\n'
    status = 1 if failures else 0
    result = subprocess.CompletedProcess(['ruff-reference-only', str(ruff), str(profile_root)], status, summary.encode('utf-8'), ('\n'.join(failures) + ('\n' if failures else '')).encode('utf-8'))
    case_results_path = None
    if record_directory is not None:
        case_results_path = record_directory / 'reference-cases.jsonl'
        case_results_path.write_text('\n'.join((json.dumps(row, ensure_ascii=False) for row in rows)) + '\n', encoding='utf-8')
    return persist_run_record(output_directory, name, command_log, profile_root, result, identities={**identities, 'reference_only': True, 'candidate_worker_built': False}, case_results_path=case_results_path)

def reference_tool_identity(ruff: Path, manifest: dict[str, object]) -> dict[str, object]:
    """Require the executable itself to match a pinned Ruff wheel and version."""
    executable = ruff.resolve()
    if profile_config.sha256(executable) not in manifest.get('reference_sha256', []):
        raise ValueError('reference-only executable hash is not one of the pinned Ruff wheels')
    version = subprocess.run([str(executable), '--version'], capture_output=True, check=False)
    if version.returncode != 0 or version.stderr:
        raise ValueError('reference-only executable did not return a clean version response')
    expected = f"ruff {manifest['reference_version']}\n".encode()
    if version.stdout != expected:
        raise ValueError(f'reference-only executable version mismatch: {version.stdout!r}')
    return {'path': str(executable), 'sha256': profile_config.sha256(executable), 'version': manifest['reference_version']}

def worker_identity(identities: dict[str, object], executable: Path) -> None:
    """Record and verify the immutable executable used for this profile."""
    worker = identities.setdefault('worker', {})
    if not isinstance(worker, dict):
        return
    worker['executable'] = {'path': str(executable.resolve()), 'sha256': profile_config.sha256(executable)}

def run_identity(manifest: dict[str, object], ruff: Path, profile_root: Path, worker_executable: Path, build_identity: dict[str, object]) -> dict[str, object]:
    """Describe the sealed inputs, candidate worker source, and pinned reference binary."""
    try:
        revision = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout.decode().strip()
    except (OSError, subprocess.CalledProcessError):
        revision = None
    try:
        worker_diff = subprocess.run(['git', 'diff', '--binary', 'HEAD', '--', 'crates/ryo'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout
        worker_status = subprocess.run(['git', 'status', '--porcelain', '--untracked-files=all', '--', 'crates/ryo'], cwd=profile_config.REPOSITORY, capture_output=True, check=True).stdout.decode().splitlines()
    except (OSError, subprocess.CalledProcessError):
        worker_diff = b''
        worker_status = None
    current_source = worker_source_identity()
    lock = profile_config.REPOSITORY / 'crates' / 'ryo' / 'Cargo.lock'
    resolved_ruff = ruff.resolve()
    cargo = shutil.which('cargo')
    try:
        cargo_version = subprocess.run([cargo, '--version'], capture_output=True, check=True).stdout.decode().strip() if cargo else None
        rustc_version = subprocess.run(['rustc', '--version', '--verbose'], capture_output=True, check=True).stdout.decode().strip()
    except (OSError, subprocess.CalledProcessError):
        cargo_version = None
        rustc_version = None
    build_source = build_identity.get('source_after_build', {})
    return {'manifest_path': profile_config.MANIFEST.relative_to(profile_config.REPOSITORY).as_posix(), 'manifest_sha256': profile_config.sha256(profile_config.MANIFEST), 'manifest_status': manifest.get('status'), 'rule_phase_registry_sha256': profile_config.sha256(profile_config.RULE_PHASES_FILE), 'selected_rule_applicability': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('rule-applicability', {}), 'target_version': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('parser-configuration', 'python-target=3.10'), 'manifest_source_commits': manifest.get('source_commits'), 'sealed_source_hashes': manifest['files'], 'worker': {'repository_revision': build_source.get('repository_revision'), 'working_tree_status': build_source.get('working_tree_status'), 'working_tree_diff_sha256': build_source.get('working_tree_diff_sha256'), 'source_tree_sha256': build_source.get('source_tree_sha256'), 'invocation_source': current_source, 'cargo_lock_sha256': profile_config.sha256(lock) if lock.is_file() else None, 'cargo_bench_package': 'idiomatic-sh-ryo-python', 'cargo_path': cargo, 'cargo_version': cargo_version, 'rustc_version': rustc_version, 'build': build_identity, 'source_status_changed_after_build': not isinstance(build_identity.get('source_after_build'), dict) or any((build_identity['source_after_build'].get(key) != current for key, current in (('repository_revision', revision), ('working_tree_status', worker_status), ('working_tree_diff_sha256', hashlib.sha256(worker_diff).hexdigest()), ('source_tree_sha256', current_source.get('source_tree_sha256'))))), 'staged_profile_sha256': staged_hashes(profile_root)}, 'ruff': {'path': str(resolved_ruff), 'sha256': profile_config.sha256(resolved_ruff), 'version': manifest.get('reference_version'), 'sealed_sha256': manifest.get('reference_sha256')}}
