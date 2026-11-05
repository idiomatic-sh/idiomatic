"""Run archived exact lambda cases under explicitly stated built-in container prerequisites."""
import ast
import copy
import hashlib
import json
import pathlib
import sys

inputs = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())
results = []
for case in inputs:
    source = case['files']['case.py']
    tree = ast.parse(source, filename='case.py')
    statement = tree.body[-1]
    assert isinstance(statement, ast.If) and not statement.orelse
    if case['rule'] == 'FURB132':
        check = statement.test
        assert isinstance(check, ast.Compare) and len(check.ops) == 1 and isinstance(check.ops[0], ast.In)
        key = check.left
        container = check.comparators[0]
        removal = statement.body[0].value
        assert isinstance(removal, ast.Call) and removal.func.attr == 'remove'
        assert ast.dump(removal.args[0]) == ast.dump(key)
        assert ast.dump(removal.func.value) == ast.dump(container)
        selected = statement
        replacement = ast.Expr(value=ast.Call(func=ast.Attribute(value=copy.deepcopy(container), attr='discard', ctx=ast.Load()), args=[copy.deepcopy(key)], keywords=[]))
    else:
        assert case['rule'] == 'RUF019'
        check = statement.test
        assert isinstance(check, ast.BoolOp) and isinstance(check.op, ast.And) and len(check.values) == 2
        membership, lookup = check.values
        assert isinstance(membership, ast.Compare) and isinstance(membership.ops[0], ast.In) and isinstance(lookup, ast.Subscript)
        key, container = membership.left, membership.comparators[0]
        assert ast.dump(key) == ast.dump(lookup.slice) and ast.dump(container) == ast.dump(lookup.value)
        selected = check
        replacement = copy.deepcopy(statement)
        replacement.test = ast.Call(func=ast.Attribute(value=copy.deepcopy(container), attr='get', ctx=ast.Load()), args=[copy.deepcopy(key)], keywords=[])
    assert isinstance(key, ast.Lambda) and isinstance(container, ast.Name)
    declared = any(isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == container.id for target in node.targets) for node in tree.body[:-1])
    def execute(module):
        events = []
        def effect():
            events.append('call')
            return 0
        class Values:
            def __getitem__(self, key):
                events.append('subscript')
                return 0
        namespace = {'effect': effect, 'factory': effect, 'make': effect, 'values': Values()}
        if not declared:
            assert case['rule'] == 'RUF019'
            namespace[container.id] = {}
        exec(compile(ast.fix_missing_locations(module), 'case.py', 'exec'), namespace)
        value = namespace[container.id]
        assert type(value) is (set if case['rule'] == 'FURB132' else dict)
        return {'events': events, 'container_size': len(value)}
    original = execute(copy.deepcopy(tree))
    changed = copy.deepcopy(tree)
    changed.body[-1] = replacement
    converted = execute(changed)
    lines = source.encode().splitlines(keepends=True)
    finding = {'rule': case['rule'], 'file': 'case.py', 'start': sum(map(len, lines[:selected.lineno-1])) + selected.col_offset, 'end': sum(map(len, lines[:selected.end_lineno-1])) + selected.end_col_offset}
    intended = [finding] if not original['events'] else []
    assert original == converted
    source_identity = hashlib.sha256(b'case.py\0' + source.encode()).hexdigest()
    results.append({'rule': case['rule'], 'name': case['name'], 'source_sha256': source_identity, 'files': case['files'], 'container': container.id, 'builtin_container': 'set' if case['rule'] == 'FURB132' else 'dict', 'injected_container_namespace': not declared, 'original': original, 'discard_or_get_control': converted, 'intended_full_findings': intended})
executable = pathlib.Path(sys.executable).resolve()
print(json.dumps({'python': str(executable), 'python_version': sys.version, 'python_sha256': hashlib.sha256(executable.read_bytes()).hexdigest(), 'program_sha256': hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(), 'scope': 'Exact supplied sources under documented built-in set/dict prerequisites; unknown custom protocols and universal rewrite safety are not established.', 'cases': results}, indent=2, sort_keys=True))
