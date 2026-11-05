import sys,json,hashlib
from pathlib import Path
sources={
 'closure-before-initializer':'def f():\n    callback = lambda: i\n    i = 0\n    for value in values:\n        i += 1\n    return callback()\n',
 'closure-inside-loop':'def f():\n    i = 0\n    for value in values:\n        callback = lambda: i\n        i += 1\n    return callback()\n',
}
rows=[]
for name,source in sources.items():
 alternate=source.replace('for value in values:', 'for i, value in enumerate(values):').replace('        i += 1\n','        pass\n')
 values=[]
 for text in [source,alternate]:
  # Execute only these two authored, self-contained counter examples with fixed inputs; no corpus code or I/O.
  env={'values':[10,20]};code=compile(text,'case.py','exec',dont_inherit=True);exec(code,env);values.append(env['f']())
 assert values==[2,1]
 rows.append(dict(name=name,filename='case.py',source=source,source_sha256=hashlib.sha256(source.encode()).hexdigest(),alternate=alternate,alternate_sha256=hashlib.sha256(alternate.encode()).hexdigest(),compile_accepted=True,dont_inherit=True,inputs={'values':[10,20]},manual_result=values[0],enumerated_result=values[1]))
print(json.dumps(dict(version=sys.version,executable=sys.executable,interpreter_sha256=hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),target_configuration='python-target=3.10',rows=rows),indent=2))
