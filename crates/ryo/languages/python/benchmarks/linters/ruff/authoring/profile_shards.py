"""Profile shards: linter-specific authoring and proof utilities."""
from __future__ import annotations
import collections
import hashlib
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import case_observations
from authoring import profile_config
from authoring import profile_worker

def partition_case_inventory(inventory: list[dict[str, object]], shard_count: int) -> list[list[dict[str, object]]]:
    """Assign each sorted global case exactly once by index modulo shard count."""
    if shard_count < 1 or shard_count > len(inventory):
        raise ValueError('invalid shard count for case inventory')
    return [[case for index, case in enumerate(inventory) if index % shard_count == shard] for shard in range(shard_count)]

def valid_shard_receipts(inventory: list[dict[str, object]], receipts: list[dict[str, object]], shard_count: int, global_identity: dict[str, object]) -> bool:
    """Reject gaps, overlaps, failed cases, and cross-run identity mixing."""
    if len(receipts) != shard_count:
        return False
    partitions = partition_case_inventory(inventory, shard_count)
    by_index = {receipt.get('shard_index'): receipt for receipt in receipts}
    if set(by_index) != set(range(shard_count)):
        return False
    all_ids = []
    for index, expected_cases in enumerate(partitions):
        receipt = by_index[index]
        expected_ids = [f"{case['rule']}/{case['name']}" for case in expected_cases]
        process_proven = receipt.get('stdout_summary_valid') is True and receipt.get('last_process_status') == 0 and (receipt.get('reconstructed_from_case_records') is not True)
        reconstructed = receipt.get('reconstructed_from_case_records') is True and receipt.get('stdout_summary_valid') is None and (receipt.get('last_process_status') is None) and isinstance(receipt.get('reconstruction'), dict) and (receipt['reconstruction'].get('method') == 'complete-case-records-no-process-summary') and (receipt['reconstruction'].get('records_sha256') == receipt.get('case_results_sha256'))
        if receipt.get('case_ids') != expected_ids or receipt.get('global_identity') != global_identity or receipt.get('shard_count') != shard_count or (receipt.get('status') != 0) or (receipt.get('case_result_records_valid') is not True) or (not (process_proven or reconstructed)):
            return False
        all_ids.extend(receipt['case_ids'])
    expected_global = [f"{case['rule']}/{case['name']}" for case in inventory]
    return len(all_ids) == len(set(all_ids)) and sorted(all_ids) == sorted(expected_global)

def shard_worker_limit() -> int:
    """Bound verifier concurrency using available memory and logical CPU count."""
    cpu_count = os.cpu_count() or 1
    available_bytes = None
    try:
        for line in Path('/proc/meminfo').read_text(encoding='ascii').splitlines():
            if line.startswith('MemAvailable:'):
                available_bytes = int(line.split()[1]) * 1024
                break
    except (OSError, ValueError):
        pass
    try:
        if available_bytes is None:
            pages = os.sysconf('SC_AVPHYS_PAGES')
            page_size = os.sysconf('SC_PAGE_SIZE')
            available_bytes = pages * page_size
    except (OSError, ValueError):
        pass
    if available_bytes is None:
        return max(1, cpu_count // 2)
    memory_slots = max(1, (available_bytes - 4 * 1024 ** 3) // (3 * 1024 ** 3))
    return max(1, min(max(1, cpu_count // 2), int(memory_slots)))

def verify_profile_sharded(name: str, profile_root: Path, worker: Path, ruff: Path, output_directory: Path, identities: dict[str, object], shard_count: int, *, resume: bool=False, worker_limit: int | None=None, runner=subprocess.run) -> int:
    """Run each audited canonical case once and persist an aggregatable receipt."""
    inventory = case_records.profile_case_inventory(profile_root)
    if not inventory:
        raise ValueError('cannot shard an empty canonical case inventory')
    effective_shard_count = min(shard_count, len(inventory))
    inventory_bytes = json.dumps(inventory, sort_keys=True, separators=(',', ':')).encode()
    inventory_sha = hashlib.sha256(inventory_bytes).hexdigest()
    profile_output = output_directory / 'profiles' / name
    shard_root = profile_output / 'shards'
    shard_root.mkdir(parents=True, exist_ok=resume)
    global_identity = {'inventory_sha256': inventory_sha, 'selection': identities, 'worker_sha256': profile_config.sha256(worker), 'ruff_sha256': profile_config.sha256(ruff), 'rule_phase_registry_sha256': identities.get('rule_phase_registry_sha256', profile_config.sha256(profile_config.RULE_PHASES_FILE)), 'selected_rule_applicability_sha256': case_records.source_hash(identities.get('selected_rule_applicability', {})), 'target': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('parser-configuration'), 'project_context_mode': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('project-context-mode', 'isolated'), 'requested_shard_count': shard_count, 'effective_shard_count': effective_shard_count}
    shards = list(enumerate(partition_case_inventory(inventory, effective_shard_count)))

    def run_one(item: tuple[int, list[dict[str, object]], list[dict[str, object]], bool, str | None]) -> dict[str, object]:
        shard_index, selected, to_run, earlier_case_failed, prior_validation_error = item
        shard_name = f'{shard_index:03d}'
        result_path = shard_root / f'{shard_name}.cases.jsonl'
        selection_path = None
        if len(to_run) != len(selected):
            selection_path = shard_root / f'{shard_name}.selection.json'
            profile_config.atomic_json_write(selection_path, [f"{case['rule']}/{case['name']}" for case in to_run])
        command = profile_worker.command_for(worker, ruff, profile_root, shard=(shard_index, effective_shard_count), case_selection=selection_path)
        environment = os.environ.copy()
        command = profile_worker.with_case_receipts(command, result_path)
        try:
            result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', env=environment, capture_output=True, text=False, check=False)
            error = prior_validation_error
            status = result.returncode
            stdout = result.stdout
            stderr = result.stderr
        except Exception as exception:
            result = None
            error = '; '.join(filter(None, (prior_validation_error, f'{type(exception).__name__}: {exception}')))
            status = 1
            stdout = b''
            stderr = error.encode()
        stdout_path = shard_root / f'{shard_name}.stdout.bin'
        stderr_path = shard_root / f'{shard_name}.stderr.bin'
        stdout_path.write_bytes(stdout)
        stderr_path.write_bytes(stderr)
        expected_ids = [f"{case['rule']}/{case['name']}" for case in selected]
        observed = []
        if result_path.is_file():
            try:
                for line in result_path.read_text(encoding='utf-8').splitlines():
                    observed.append(json.loads(line))
                observed = case_observations.case_record_objects(observed)
            except (OSError, ValueError) as exception:
                observed = []
                error = '; '.join(filter(None, (error, f'native case receipt could not be read: {exception}')))
        selected_ids = {f"{case['rule']}/{case['name']}" for case in selected}
        try:
            observed_ids = case_observations.complete_case_ids(selected, observed, receipt_file=result_path)
        except (OSError, ValueError) as exception:
            observed_ids = set()
            error = '; '.join(filter(None, (error, f'case receipt validation failed: {exception}')))
        side_counts = collections.Counter(((row.get('rule'), row.get('name'), row.get('side')) for row in observed))
        valid_records = observed_ids == selected_ids and all((side_counts[case['rule'], case['name'], side] == 1 for case in selected for side in ('reference', 'candidate'))) and (len(observed) == 2 * len(selected))
        completed_failed = any((not row.get('case_passed', False) for row in observed))
        expected_summary = f'{len(to_run)} cases passed against both contestants\n'.encode()
        summary_valid = stdout == expected_summary
        final_status = 0 if status == 0 and error is None and (not earlier_case_failed) and (not completed_failed) and valid_records and summary_valid else 1
        receipt = {'profile': name, 'shard_index': shard_index, 'shard_count': effective_shard_count, 'case_ids': expected_ids, 'status': final_status, 'last_process_status': status, 'error': error, 'global_identity': global_identity, 'stdout_sha256': hashlib.sha256(stdout).hexdigest(), 'stderr_sha256': hashlib.sha256(stderr).hexdigest(), 'case_results_sha256': profile_config.sha256(result_path) if result_path.is_file() else None, 'case_result_records_valid': valid_records, 'stdout_summary_valid': summary_valid, 'executed_case_ids': [f"{case['rule']}/{case['name']}" for case in to_run], 'case_selection_file': selection_path.name if selection_path is not None else None, 'case_selection_sha256': profile_config.sha256(selection_path) if selection_path is not None else None}
        profile_config.atomic_json_write(shard_root / f'{shard_name}.receipt.json', receipt)
        return receipt
    prior_aggregate_path = profile_output / 'aggregate.json'
    if resume:
        plan_path = profile_output / 'shard-plan.json'
        if not plan_path.is_file():
            raise ValueError('cannot resume: prior shard plan is missing')
        prior_plan = json.loads(plan_path.read_text(encoding='utf-8'))
        if prior_plan.get('global_identity') != global_identity:
            raise ValueError('cannot resume with changed global inventory/worker/reference/target identity')
        if prior_plan.get('staged_inputs') != profile_worker.staged_hashes(profile_root):
            raise ValueError('cannot resume with changed staged profile inputs')
    else:
        profile_config.atomic_json_write(profile_output / 'shard-plan.json', {'profile': name, 'global_identity': global_identity, 'staged_inputs': profile_worker.staged_hashes(profile_root), 'case_ids': [f"{case['rule']}/{case['name']}" for case in inventory], 'shard_count': effective_shard_count, 'requested_shard_count': shard_count})
    receipts = []
    pending_shards = []
    for shard_index, selected in shards:
        shard_name = f'{shard_index:03d}'
        receipt_path = shard_root / f'{shard_name}.receipt.json'
        records_path = shard_root / f'{shard_name}.cases.jsonl'
        expected_ids = [f"{case['rule']}/{case['name']}" for case in selected]
        reused = False
        if resume and receipt_path.is_file() and records_path.is_file():
            try:
                receipt = json.loads(receipt_path.read_text(encoding='utf-8'))
                records = case_observations.case_record_objects([json.loads(line) for line in records_path.read_text(encoding='utf-8').splitlines()])
                counts = collections.Counter(((row.get('rule'), row.get('name'), row.get('side')) for row in records))
                records_valid = case_observations.complete_case_ids(selected, records, receipt_file=records_path) == {f"{case['rule']}/{case['name']}" for case in selected} and all((counts[case['rule'], case['name'], side] == 1 for case in selected for side in ('reference', 'candidate'))) and (len(records) == 2 * len(selected))
                reused = receipt.get('global_identity') == global_identity and receipt.get('case_ids') == expected_ids and (receipt.get('case_result_records_valid') is True) and (receipt.get('case_results_sha256') == profile_config.sha256(records_path)) and records_valid
                if reused:
                    receipts.append(receipt)
            except (OSError, ValueError, json.JSONDecodeError):
                reused = False
        if not reused:
            if not resume and (receipt_path.exists() or records_path.exists()):
                raise ValueError('shard output already exists')
            prior_records = []
            malformed = False
            prior_validation_error = None
            if resume and records_path.is_file():
                try:
                    prior_records = case_observations.case_record_objects([json.loads(line) for line in records_path.read_text(encoding='utf-8').splitlines()])
                except (OSError, ValueError) as exception:
                    malformed = True
                    prior_validation_error = f'prior native case receipt could not be read: {exception}'
            try:
                completed = set() if malformed else case_observations.complete_case_ids(selected, prior_records, receipt_file=records_path)
            except (OSError, ValueError) as exception:
                completed = set()
                prior_validation_error = f'prior case receipt validation failed: {exception}'
            completed_ids = {f"{case['rule']}/{case['name']}" for case in selected}
            keep = [row for row in prior_records if f"{row.get('rule')}/{row.get('name')}" in completed]
            if malformed:
                keep = []
            if records_path.is_file() and records_path.stat().st_size:
                original = records_path.read_bytes()
                history = records_path.with_name(f'{records_path.name}.history-{hashlib.sha256(original).hexdigest()}')
                if history.exists() and history.read_bytes() != original:
                    raise ValueError('receipt history identity collision')
                if not history.exists():
                    with history.open('xb') as saved:
                        saved.write(original)
                        saved.flush()
                        os.fsync(saved.fileno())
            temporary_records = records_path.with_name(f'.{records_path.name}.tmp')
            with temporary_records.open('w', encoding='utf-8') as output_records:
                for row in keep:
                    output_records.write(json.dumps(row, ensure_ascii=False, separators=(',', ':')) + '\n')
                output_records.flush()
                os.fsync(output_records.fileno())
            temporary_records.replace(records_path)
            to_run = [case for case in selected if f"{case['rule']}/{case['name']}" not in completed]
            if not to_run:
                failed = any((not row.get('case_passed', False) for row in keep))
                records_valid = len(keep) == 2 * len(selected) and completed == completed_ids and all((row.get('complete') is True and row.get('error') is None for row in keep))
                receipt = {'profile': name, 'shard_index': shard_index, 'shard_count': effective_shard_count, 'case_ids': expected_ids, 'executed_case_ids': [], 'status': int(failed or not records_valid), 'last_process_status': None, 'error': None, 'global_identity': global_identity, 'stdout_sha256': None, 'stderr_sha256': None, 'case_results_sha256': profile_config.sha256(records_path), 'case_result_records_valid': records_valid, 'stdout_summary_valid': None, 'reconstructed_from_case_records': True, 'reconstruction': {'method': 'complete-case-records-no-process-summary', 'records_sha256': profile_config.sha256(records_path), 'process_exit_observed': False, 'stdout_summary_observed': False}}
                profile_config.atomic_json_write(receipt_path, receipt)
                receipts.append(receipt)
                continue
            for path in (receipt_path, shard_root / f'{shard_name}.stdout.bin', shard_root / f'{shard_name}.stderr.bin'):
                path.unlink(missing_ok=True)
            earlier_failed = any((not row.get('case_passed', False) for row in keep))
            pending_shards.append((shard_index, selected, to_run, earlier_failed, prior_validation_error))
    with ThreadPoolExecutor(max_workers=min(effective_shard_count, worker_limit or shard_worker_limit())) as pool:
        futures = [pool.submit(run_one, shard) for shard in pending_shards]
        for future in as_completed(futures):
            receipts.append(future.result())
    receipts.sort(key=lambda receipt: receipt['shard_index'])
    flattened = [case_id for receipt in receipts for case_id in receipt['case_ids']]
    receipts_valid = valid_shard_receipts(inventory, receipts, effective_shard_count, global_identity)
    aggregate = {'profile': name, 'global_identity': global_identity, 'staged_inputs': profile_worker.staged_hashes(profile_root), 'case_count': len(inventory), 'shard_count': effective_shard_count, 'requested_shard_count': shard_count, 'receipts': [f'shards/{index:03d}.receipt.json' for index in range(effective_shard_count)], 'all_shards_passed': receipts_valid, 'case_ids_exact_once': len(flattened) == len(set(flattened)) and sorted(flattened) == sorted((f"{case['rule']}/{case['name']}" for case in inventory))}
    aggregate['status'] = 0 if aggregate['all_shards_passed'] and aggregate['case_ids_exact_once'] else 1
    profile_config.atomic_json_write(profile_output / 'aggregate.json', aggregate)
    print(f"{name}: {aggregate['case_count']} cases across {effective_shard_count} of {shard_count} requested shards; status {aggregate['status']}", file=sys.stderr)
    return int(aggregate['status'])
