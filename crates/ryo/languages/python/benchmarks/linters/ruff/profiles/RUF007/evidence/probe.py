import ast
import hashlib
import io
import json
import pathlib
import sys
import tokenize

rule = sys.argv[1]
root = pathlib.Path(__file__).resolve().parent
sources = json.loads((root / 'source-controls.json').read_text())
result = {
    'rule': rule,
    'interpreter_version': sys.version,
    'interpreter_sha256': hashlib.sha256(pathlib.Path(sys.executable).read_bytes()).hexdigest(),
    'controls': {},
}
for name, source in sorted(sources.items()):
    row = {'source_sha256': hashlib.sha256(source.encode()).hexdigest()}
    if rule == 'PLE0643':
        variants = {'exact': {}}
        if name == 'unknown-starred':
            variants = {'empty_items': {'items': []}, 'two_items': {'items': [10, 20]}}
        outcomes = {}
        for label, values in variants.items():
            namespace = dict(values)
            try:
                exec(compile(source, 'case.py', 'exec'), namespace)
                outcomes[label] = {'result': namespace.get('result')}
            except Exception as error:
                outcomes[label] = {'exception': type(error).__name__}
        row['outcomes'] = outcomes
    elif rule == 'RUF007':
        tree = ast.parse(source)
        call = tree.body[0].value
        lowers = [arg.slice.lower.value for arg in call.args]
        namespace = {'xs': [0, 1, 2]}
        exec(compile(source, 'case.py', 'exec'), namespace)
        row['lower_bounds'] = lowers
        row['exact_successor'] = lowers[1] == lowers[0] + 1
        row['result'] = list(namespace['result'])
    elif rule == 'RUF028':
        lines = source.splitlines(keepends=True)
        def offset(line, column):
            return sum(len(part.encode()) for part in lines[:line - 1]) + len(lines[line - 1][:column].encode())
        comments = []
        for token in tokenize.generate_tokens(io.StringIO(source).readline):
            if token.type == tokenize.COMMENT:
                comments.append({
                    'text': token.string,
                    'line': token.start[0],
                    'column': token.start[1],
                    'start_byte': offset(*token.start),
                    'end_byte': offset(*token.end),
                })
        row['comments'] = comments
        row['syntax_valid'] = bool(ast.parse(source))
    result['controls'][name] = row
print(json.dumps(result, sort_keys=True, indent=2))
