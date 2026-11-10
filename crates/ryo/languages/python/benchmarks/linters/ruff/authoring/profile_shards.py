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
        process_proven = receipt.get('stdout_summary_valid') is True and receipt.get('last_process_status') == 0
        if receipt.get('case_ids') != expected_ids or receipt.get('global_identity') != global_identity or receipt.get('shard_count') != shard_count or (receipt.get('status') != 0) or (receipt.get('case_result_records_valid') is not True) or (not process_proven):
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

def verify_profile_sharded(name: str, profile_root: Path, worker: Path, ruff: Path, output_directory: Path, identities: dict[str, object], shard_count: int, *, worker_limit: int | None=None, runner=subprocess.run) -> int:
    """Run each audited canonical case once and persist an aggregatable receipt."""
    inventory = case_records.profile_case_inventory(profile_root)
    if not inventory:
        raise ValueError('cannot shard an empty canonical case inventory')
    effective_shard_count = min(shard_count, len(inventory))
    inventory_bytes = json.dumps(inventory, sort_keys=True, separators=(',', ':')).encode()
    inventory_sha = hashlib.sha256(inventory_bytes).hexdigest()
    profile_output = output_directory / 'profiles' / name
    shard_root = profile_output / 'shards'
    shard_root.mkdir(parents=True)
    global_identity = {'inventory_sha256': inventory_sha, 'selection': identities, 'worker_sha256': profile_config.sha256(worker), 'ruff_sha256': profile_config.sha256(ruff), 'rule_phase_registry_sha256': identities.get('rule_phase_registry_sha256', profile_config.sha256(profile_config.RULE_PHASES_FILE)), 'selected_rule_applicability_sha256': case_records.source_hash(identities.get('selected_rule_applicability', {})), 'target': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('parser-configuration'), 'project_context_mode': tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8')).get('project-context-mode', 'isolated'), 'requested_shard_count': shard_count, 'effective_shard_count': effective_shard_count}
    shards = list(enumerate(partition_case_inventory(inventory, effective_shard_count)))

    def run_one(item: tuple[int, list[dict[str, object]]]) -> dict[str, object]:
        shard_index, selected = item
        shard_name = f'{shard_index:03d}'
        result_path = shard_root / f'{shard_name}.cases.jsonl'
        command = profile_worker.command_for(worker, ruff, profile_root, shard=(shard_index, effective_shard_count))
        environment = os.environ.copy()
        command = profile_worker.with_case_receipts(command, result_path)
        try:
            result = runner(command, cwd=profile_config.REPOSITORY / 'crates' / 'ryo', env=environment, capture_output=True, text=False, check=False)
            error = None
            status = result.returncode
            stdout = result.stdout
            stderr = result.stderr
        except Exception as exception:
            result = None
            error = f'{type(exception).__name__}: {exception}'
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
        expected_summary = f'{len(selected)} cases passed against both contestants\n'.encode()
        summary_valid = stdout == expected_summary
        final_status = 0 if status == 0 and error is None and (not completed_failed) and valid_records and summary_valid else 1
        receipt = {'profile': name, 'shard_index': shard_index, 'shard_count': effective_shard_count, 'case_ids': expected_ids, 'status': final_status, 'last_process_status': status, 'error': error, 'global_identity': global_identity, 'stdout_sha256': hashlib.sha256(stdout).hexdigest(), 'stderr_sha256': hashlib.sha256(stderr).hexdigest(), 'case_results_sha256': profile_config.sha256(result_path) if result_path.is_file() else None, 'case_result_records_valid': valid_records, 'stdout_summary_valid': summary_valid, 'executed_case_ids': expected_ids}
        profile_config.atomic_json_write(shard_root / f'{shard_name}.receipt.json', receipt)
        return receipt
    profile_config.atomic_json_write(profile_output / 'shard-plan.json', {'profile': name, 'global_identity': global_identity, 'staged_inputs': profile_worker.staged_hashes(profile_root), 'case_ids': [f"{case['rule']}/{case['name']}" for case in inventory], 'shard_count': effective_shard_count, 'requested_shard_count': shard_count})
    receipts = []
    with ThreadPoolExecutor(max_workers=min(effective_shard_count, worker_limit or shard_worker_limit())) as pool:
        futures = [pool.submit(run_one, shard) for shard in shards]
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
