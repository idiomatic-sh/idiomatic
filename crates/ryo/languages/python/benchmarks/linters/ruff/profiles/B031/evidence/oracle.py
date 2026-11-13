"""Independent CPython operations/scope evidence; never reads candidate output.

The original cases remain unchanged. Companion programs insert assertions at the
end of a loop body, before advancing groupby's shared underlying iterator.
"""
from pathlib import Path
import ast
import builtins
import contextlib
import hashlib
import io
from itertools import groupby
import json
import platform
import sys

cases_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).with_name('original-cases.json')
cases = {c['name']: c for c in json.loads(cases_path.read_text())}
result = {'python': platform.python_version(), 'executable': str(Path(sys.executable).resolve()),
          'executable_sha256': hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(), 'cases': []}
# Execute unchanged originals with a list wrapper that returns builtin list.
# Only real grouper inputs are logged, so reusable replacement lists and the
# outer list(generator) materialization do not masquerade as group consumption.
grouper_type = type(next(groupby([1]))[1])
positive_calls = {'comprehension-shadow-does-not-replace', 'nested-termination-stays-local',
                  'opposite-arm-replacement', 'possible-replacement',
                  'comprehension-filter-consumer', 'generator-repeated-consumer'}
for name in [n for n in cases if n != 'upstream-full-fixture']:
    source = cases[name]['files']['case.py']
    tree = ast.parse(source)
    trials = []
    for flag in [False, True]:
        batches = []
        def observed_list(value):
            items = builtins.list(value)
            if isinstance(value, grouper_type):
                batches.append(items)
            return items
        namespace = {'__builtins__': dict(vars(builtins), list=observed_list), 'flag': flag}
        with contextlib.redirect_stdout(io.StringIO()):
            exec(compile(source, name + '.py', 'exec'), namespace)
        expectations = {
            'reference-twice': [], 'identity-twice': [], 'print-twice': [],
            'iter-is-an-observation': [], 'next-is-incremental-iteration': [],
            'conditional-expression': [[1, 1]], 'observation-before-consumption': [[1, 1]],
            'comprehension-shadow-does-not-replace': [[1, 1], []],
            'nested-termination-stays-local': [[1, 1], []],
            'one-inner-iteration-is-not-reuse': [[1, 1]],
            'opposite-arm-replacement': [] if flag else [[1, 1], []],
            'possible-replacement': [] if flag else [[1, 1], []],
            'comprehension-filter-consumer': [[1, 1], []],
            'generator-repeated-consumer': [[1, 1], []],
            'later-comprehension-iterator': [],
        }
        assert batches == expectations[name], (name, flag, batches)
        trial = {'flag': flag, 'group_consumer_results': batches}
        if name == 'next-is-incremental-iteration':
            assert namespace['first'] == namespace['second'] == 1
            trial['successive_next_values'] = [namespace['first'], namespace['second']]
        if name == 'later-comprehension-iterator':
            assert namespace['values'] == [1, 1]
            trial['later_iteration_result'] = namespace['values']
        if name in {'reference-twice', 'identity-twice', 'print-twice', 'iter-is-an-observation'}:
            companion = source + '    assert list(g) == [1, 1]\n'
            with contextlib.redirect_stdout(io.StringIO()):
                exec(compile(companion, name + '-remaining.py', 'exec'), {})
            trial['unconsumed_companion'] = companion
        trials.append(trial)
    intended = []
    if name in positive_calls:
        reads = [node.args[0] for node in ast.walk(tree) if isinstance(node, ast.Call)
                 and isinstance(node.func, ast.Name) and node.func.id == 'list'
                 and len(node.args) == 1 and isinstance(node.args[0], ast.Name) and node.args[0].id == 'g']
        read = max(reads, key=lambda n: (n.lineno, n.col_offset))
    elif name == 'later-comprehension-iterator':
        comp = next(node for node in ast.walk(tree) if isinstance(node, ast.ListComp))
        assert isinstance(comp.generators[0].iter, ast.List) and len(comp.generators[0].iter.elts) == 2
        read = comp.generators[1].iter
        assert isinstance(read, ast.Name) and read.id == 'g'
    if name in positive_calls or name == 'later-comprehension-iterator':
        start = sum(len(line.encode()) for line in source.splitlines(keepends=True)[:read.lineno - 1]) + read.col_offset
        intended = [{'rule': 'B031', 'file': 'case.py', 'start': start, 'end': start + 1}]
    assert intended == cases[name]['expected'], (name, intended, cases[name]['expected'])
    result['cases'].append({'case': name, 'original_source': source,
                            'original_sha256': hashlib.sha256(source.encode()).hexdigest(),
                            'execution_namespace': 'ordinary builtins except list, a recording wrapper returning builtin list(value); only grouper calls logged; flag explicitly False/True',
                            'trials': trials, 'independent_intended': intended})
# Exact fixture scope falsifier: a comprehension target is local to that
# comprehension, while collect_shop_items really consumes its argument (+=).
source = cases['upstream-full-fixture']['files']['case.py']
tree = ast.parse(source)
matched = []
for statement in tree.body:
    if not isinstance(statement, ast.For):
        continue
    comps = [n for n in ast.walk(statement) if isinstance(n, ast.ListComp)]
    if len(comps) == 2 and isinstance(comps[0].generators[0].target, ast.Name) and comps[0].generators[0].target.id == 'section_items':
        consuming = comps[1].elt
        assert isinstance(consuming, ast.Call)
        read = consuming.args[1]
        assert isinstance(read, ast.Name) and read.id == 'section_items'
        start = sum(len(line.encode()) for line in source.splitlines(keepends=True)[:read.lineno - 1]) + read.col_offset
        matched.append({'rule': 'B031', 'file': 'case.py', 'start': start, 'end': start + len(read.id)})
assert len(matched) == 1
companion = '''from itertools import groupby
batches = []
def consume(group):
    batches.append(list(group))
for _, section_items in groupby([1, 1]):
    original = section_items
    _ = [_ for section_items in range(3)]
    assert section_items is original
    _ = [consume(section_items) for shopper in ['Jane', 'Joe']]
assert batches == [[1, 1], []]
'''
namespace = {}
exec(compile(companion, 'comprehension-scope-control.py', 'exec'), namespace)
result['cases'].append({'case': 'upstream-full-fixture', 'original_sha256': hashlib.sha256(source.encode()).hexdigest(),
                        'companion_source': companion, 'batches': namespace['batches'],
                        'independently_added_finding': matched[0],
                        'retained_snapshot_findings': 23})
print(json.dumps(result, indent=2, sort_keys=True))
