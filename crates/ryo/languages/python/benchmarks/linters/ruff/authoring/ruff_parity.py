"""Whole-project rule parity: one ryo `run-ryo` report against pinned Ruff over the same staged request, finding by finding."""
from __future__ import annotations
import bisect
import collections
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from authoring import profile_config
from authoring.case_records import ruff_location_to_byte

# The differences Ruff parity accepts, each with its reason; every other difference fails the run.
ACCEPTED = profile_config.REPOSITORY / 'crates/ryo/docs/contracts/evidence/parity/django-413/accepted-differences.json'
SAMPLES = 3
MATCH_KEY = '(file, rule, line, column), multiset'
NORMALISATION = ('ryo reports UTF-8 byte offsets; they are converted to a 1-based line and a 1-based Unicode-character column, lines split on '
                 '\\r\\n|\\r|\\n as ruff_location_to_byte splits them. Ruff positions are used as given, and every Ruff start is checked to '
                 'agree with ruff_location_to_byte. Ruff filenames are made relative to the request root. End positions are compared on matched pairs.')


class Positions:
    """Byte offsets of one corpus to 1-based (line, Unicode column), one decoded source per file."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.files: dict[str, tuple[str, list[int], list[int]]] = {}

    def source(self, name: str) -> str:
        return self.lines(name)[0]

    def lines(self, name: str) -> tuple[str, list[int], list[int]]:
        if name not in self.files:
            text = (self.root / name).read_bytes().decode('utf-8')
            starts = [0, *(match.end() for match in re.finditer('\\r\\n|\\r|\\n', text))]
            self.files[name] = (text, [len(text[:start].encode('utf-8')) for start in starts], starts)
        return self.files[name]

    def row_column(self, name: str, offset: int) -> tuple[int, int]:
        text, byte_starts, character_starts = self.lines(name)
        line = bisect.bisect_right(byte_starts, offset) - 1
        prefix = text[character_starts[line]:].encode('utf-8')[:offset - byte_starts[line]].decode('utf-8')
        return line + 1, len(prefix) + 1


def ruff_command(ruff: str, request: dict[str, object]) -> list[str]:
    """The pinned Ruff invocation over the request's files and rules, isolated from any project configuration."""
    targets = {configuration: target for target, configuration in profile_config.PARSER_CONFIGURATIONS.items()}
    target = targets.get(request['parser_configuration'])
    if target is None:
        raise ValueError(f"unsupported parser configuration: {request['parser_configuration']}")
    return [ruff, 'check', '--isolated', '--no-cache', '--output-format', 'json', '--select', ','.join(request['selected_rules']), '--target-version', target, *request['files']]


def compare(ryo: dict[str, object], ruff: list[dict[str, object]], request: dict[str, object], positions: Positions) -> tuple[list[dict[str, object]], dict[str, object], dict[str, list[tuple[str, str, int, int]]]]:
    """Per-rule rows, the cross-rule counts and every unmatched finding per rule, as (missing or extra, file, line, column), of a multiset match on (file, rule, line, column)."""
    selected = list(request['selected_rules'])
    chosen = set(selected)
    root = Path(request['root'])
    candidate = collections.defaultdict(list)
    reference = collections.defaultdict(list)
    outside = {'ryo': collections.Counter(), 'ruff': collections.Counter()}
    for finding in ryo['diagnostics']:
        if finding['rule'] not in chosen:
            outside['ryo'][finding['rule']] += 1
            continue
        line, column = positions.row_column(finding['file'], finding['start'])
        candidate[(finding['file'], finding['rule'], line, column)].append(list(positions.row_column(finding['file'], finding['end'])))
    for finding in ruff:
        name = Path(finding['filename']).relative_to(root).as_posix()
        code = str(finding['code'])
        if code not in chosen:
            outside['ruff'][code] += 1
            continue
        key = (name, code, finding['location']['row'], finding['location']['column'])
        if positions.row_column(name, ruff_location_to_byte(positions.source(name), finding['location'])) != key[2:]:
            raise ValueError(f'ruff_location_to_byte disagrees with the Ruff position {key}')
        reference[key].append({'end': [finding['end_location']['row'], finding['end_location']['column']], 'message': finding['message']})
    differing: dict[str, list[tuple[str, str, int, int]]] = {rule: [] for rule in selected}
    rows = {rule: {'rule': rule, 'ryo': 0, 'ruff': 0, 'matched': 0, 'missing': 0, 'extra': 0, 'end_differs': 0, 'missing_samples': [], 'extra_samples': [], 'end_differs_samples': []} for rule in selected}
    for key in sorted(set(candidate) | set(reference)):
        name, rule, line, column = key
        ours, theirs = candidate.get(key, []), reference.get(key, [])
        row = rows[rule]
        row['ryo'] += len(ours)
        row['ruff'] += len(theirs)
        matched = min(len(ours), len(theirs))
        row['matched'] += matched
        for end, observed in zip(ours, theirs):
            if end != observed['end']:
                row['end_differs'] += 1
                row['end_differs_samples'].append({'file': name, 'line': line, 'column': column, 'ryo_end': end, 'ruff_end': observed['end']})
        for observed in theirs[matched:]:
            row['missing'] += 1
            differing[rule].append(('missing', name, line, column))
            row['missing_samples'].append({'file': name, 'line': line, 'column': column, 'end': observed['end'], 'ruff_message': observed['message']})
        for end in ours[matched:]:
            row['extra'] += 1
            differing[rule].append(('extra', name, line, column))
            row['extra_samples'].append({'file': name, 'line': line, 'column': column, 'end': end})
    for row in rows.values():
        for field in ('missing_samples', 'extra_samples', 'end_differs_samples'):
            row[field] = row[field][:SAMPLES]
        row['class'] = 'identical' if not (row['missing'] or row['extra']) else 'both' if row['missing'] and row['extra'] else 'ryo-missing' if row['missing'] else 'ryo-extra'
        row['differences'] = row['missing'] + row['extra']
    ordered = [rows[rule] for rule in selected]
    skipped = ryo.get('skipped', [])
    counts = {
        'outside_selection': {side: dict(sorted(counter.items())) for side, counter in outside.items()},
        'ryo_skipped': {'rows': len(skipped), 'reasons': dict(collections.Counter(entry['reason'] for entry in skipped)), 'files': sorted({entry['path'] for entry in skipped})},
        'report_totals': {'ryo': len(ryo['diagnostics']), 'ruff': len(ruff)},
        'totals': {field: sum(row[field] for row in ordered) for field in ('ryo', 'ruff', 'matched', 'missing', 'extra', 'end_differs')},
        'classes': dict(collections.Counter(row['class'] for row in ordered)),
        'rules_by_class': {kind: [row['rule'] for row in ordered if row['class'] == kind] for kind in ('ryo-missing', 'ryo-extra', 'both')},
    }
    return ordered, counts, differing


def rows_text(rows: list[dict[str, object]]) -> str:
    """per-rule.json: the rows in selection order, a run of identical rules on one line and each other rule on a line of its own, so the file stays within the line budget."""
    lines, run = [], []
    for row in rows:
        text = json.dumps(row, ensure_ascii=False)
        if row['class'] == 'identical' and not row['end_differs']:
            run.append(text)
            continue
        if run:
            lines.append(', '.join(run))
            run = []
        lines.append(text)
    if run:
        lines.append(', '.join(run))
    return '[\n' + ',\n'.join(lines) + '\n]\n'


def corpus_digest(request: dict[str, object]) -> str:
    """The identity of the corpus a request reads: its file list and its context manifest's source, auxiliary and missing entries, never its location, rules or store."""
    manifest = request['context_manifest']
    corpus = {'files': request['files'], 'logical_root': manifest['logical_root'], 'source_files': manifest['source_files'], 'auxiliary_files': manifest['auxiliary_files'], 'missing': manifest['missing']}
    return hashlib.sha256(json.dumps(corpus, sort_keys=True, separators=(',', ':')).encode('utf-8')).hexdigest()


def load_accepted(text: str, request: dict[str, object]) -> dict[str, collections.Counter]:
    """The accepted findings per rule; refuses a file written for another corpus, a rule named twice and a finding named twice."""
    accepted = json.loads(text)
    if accepted['corpus'] != corpus_digest(request):
        raise ValueError(f"accepted differences are for corpus {accepted['corpus']}, the request reads corpus {corpus_digest(request)}")
    entries: dict[str, collections.Counter] = {}
    for entry in accepted['differences']:
        if entry['rule'] in entries:
            raise ValueError(f"accepted differences name {entry['rule']} twice")
        findings = collections.Counter((finding['kind'], finding['file'], finding['line'], finding['column']) for finding in entry['findings'])
        if not entry['reason'] or not findings or max(findings.values()) > 1 or any(kind not in ('missing', 'extra') for kind, *_ in findings):
            raise ValueError(f"accepted differences for {entry['rule']} need a reason and distinct missing or extra findings")
        entries[entry['rule']] = findings
    return entries


def accepted_problems(rows: list[dict[str, object]], differing: dict[str, list[tuple[str, str, int, int]]], accepted: dict[str, collections.Counter]) -> list[str]:
    """Each unmatched finding the accepted file does not name, each accepted finding that no longer occurs, each end-position difference, and each accepted rule not selected."""
    problems = []
    for row in rows:
        actual = collections.Counter(differing[row['rule']])
        expected = accepted.get(row['rule'], collections.Counter())
        problems.extend(f"{row['rule']}: {kind} {name}:{line}:{column} not accepted" for kind, name, line, column in sorted((actual - expected).elements()))
        problems.extend(f"{row['rule']}: accepted {kind} {name}:{line}:{column} no longer occurs" for kind, name, line, column in sorted((expected - actual).elements()))
        if row['end_differs']:
            problems.append(f"{row['rule']}: {row['end_differs']} end positions differ")
    selected = {row['rule'] for row in rows}
    problems.extend(f'{rule}: accepted but not selected' for rule in sorted(set(accepted) - selected))
    return problems


def summary(rows: list[dict[str, object]], counts: dict[str, object], reasons: dict[str, str], problems: list[str], identity: dict[str, object]) -> dict[str, object]:
    differences = [{'rule': row['rule'], 'class': row['class'], 'ryo': row['ryo'], 'ruff': row['ruff'], 'missing': row['missing'], 'extra': row['extra'],
                    **({'accepted': reasons[row['rule']]} if row['rule'] in reasons else {})} for row in rows if row['differences']]
    return {**identity, 'match_key': MATCH_KEY, 'normalisation': NORMALISATION, **counts, 'differences': differences, 'problems': problems, 'status': 'fail' if problems else 'pass'}


def run_ruff_parity(ruff: Path, request_path: Path, ryo_output: Path | None, worker: Path | None, output_directory: Path, accepted_path: Path = ACCEPTED) -> int:
    """Run (or read) ryo and run pinned Ruff over one staged request; write summary.json and per-rule.json; 0 only when every difference is accepted."""
    executable = shutil.which(str(ruff))
    if executable is None:
        raise ValueError(f'Ruff executable not found: {ruff}')
    version = subprocess.run([executable, '--version'], check=True, capture_output=True, text=True).stdout.strip()
    if version != f'ruff {profile_config.PINNED_RUFF_VERSION}':
        raise ValueError(f'Ruff parity requires ruff {profile_config.PINNED_RUFF_VERSION}, found {version}')
    request = json.loads(request_path.read_text(encoding='utf-8'))
    if not (request_path.parent / 'staged.json').is_file():
        raise ValueError(f'{request_path} is not a staged request: staged.json is missing beside it')
    accepted_text = accepted_path.read_text(encoding='utf-8')
    accepted = load_accepted(accepted_text, request)
    output_directory.mkdir(parents=True)
    ryo_run: dict[str, object]
    if worker is not None:
        ryo_output = output_directory / 'ryo.json'
        command = [str(worker), 'run-ryo', '--request-file', str(request_path), str(request['parser_configuration'])]
        with ryo_output.open('wb') as stdout:
            completed = subprocess.run(command, cwd=request['root'], stdout=stdout, stderr=subprocess.PIPE, check=False)
        (output_directory / 'ryo.stderr').write_bytes(completed.stderr)
        if completed.returncode != 0:
            raise ValueError(f'ryo run-ryo exited {completed.returncode}; stderr in {output_directory / "ryo.stderr"}')
        ryo_run = {'worker': str(worker), 'worker_sha256': profile_config.sha256(worker), 'rc': completed.returncode}
    else:
        ryo_run = {'reused': str(ryo_output)}
    command = ruff_command(executable, request)
    completed = subprocess.run(command, cwd=request['root'], capture_output=True, check=False)
    (output_directory / 'ruff.json').write_bytes(completed.stdout)
    if completed.returncode not in (0, 1):
        raise ValueError(f'ruff exited {completed.returncode}: {completed.stderr.decode(errors="replace")[-2000:]}')
    rows, counts, differing = compare(json.loads(ryo_output.read_text(encoding='utf-8')), json.loads(completed.stdout), request, Positions(Path(request['root'])))
    problems = accepted_problems(rows, differing, accepted)
    reasons = {entry['rule']: entry['reason'] for entry in json.loads(accepted_text)['differences']}
    staged = request_path.parent / 'staged.json'
    source = subprocess.run(['git', '-C', str(profile_config.REPOSITORY), 'rev-parse', 'HEAD'], check=True, capture_output=True, text=True).stdout.strip()
    identity = {
        'source': source,
        'staged': json.loads(staged.read_text(encoding='utf-8')),
        'request_sha256': profile_config.sha256(request_path),
        'corpus': request['root'],
        'corpus_digest': corpus_digest(request),
        'files': len(request['files']),
        'selected_rules': len(request['selected_rules']),
        'target': request['parser_configuration'],
        'ryo_run': {**ryo_run, 'sha256': profile_config.sha256(ryo_output)},
        'ruff_run': {'version': version, 'path': executable, 'sha256': profile_config.sha256(Path(executable)), 'rc': completed.returncode,
                     'sha256_output': hashlib.sha256(completed.stdout).hexdigest(), 'stderr': completed.stderr.decode(errors='replace').strip(),
                     'command': ' '.join(command[:command.index('--select') + 1]) + ' <request selected_rules> ' + ' '.join(command[command.index('--target-version'):command.index('--target-version') + 2]) + ' <request files>, cwd the request root'},
        'accepted_differences': {'path': accepted_path.relative_to(profile_config.REPOSITORY).as_posix(), 'sha256': profile_config.sha256(accepted_path)},
    }
    (output_directory / 'per-rule.json').write_text(rows_text(rows), encoding='utf-8')
    profile_config.atomic_json_write(output_directory / 'summary.json', summary(rows, counts, reasons, problems, identity))
    print(f"ruff parity: {counts['classes'].get('identical', 0)} identical of {len(rows)} rules; {len(problems)} unaccepted", flush=True)
    for problem in problems:
        print(problem, flush=True)
    return 1 if problems else 0
