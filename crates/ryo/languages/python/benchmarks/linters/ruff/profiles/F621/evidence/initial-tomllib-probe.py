from pathlib import Path
import json,sys,hashlib,subprocess,tempfile,tomllib
ruff=Path('/home/vscode/.local/bin/ruff');existing=Path('/workspaces/idiomatic-benchmark-guards/crates/ryo/languages/python/benchmarks/linters/ruff/rules/F621/cases.toml')
def target(n):return ', '.join([*(f'x{i}' for i in range(n)),'*rest'])
cases=[(x['name'],x['files']['case.py'])for x in tomllib.loads(existing.read_text())['cases']]
cases += [('512-after-star','*rest, '+', '.join(f'x{i}' for i in range(512))+' = values\n'),('300-without-star',', '.join(f'x{i}' for i in range(300))+' = values\n'),('load-tuple', 'result = ('+target(256)+')\n'),('list-target','['+target(256)+'] = values\n'),('for-target','for '+target(256)+' in items:\n    pass\n'),('nested-target','outer, ('+target(256)+') = values\n'),('upstream-F622',Path('/tmp/ruff-0165-src/crates/ruff_linter/resources/test/fixtures/pyflakes/F622.py').read_text())]
observed=[]
for name,source in cases:
 try:compile(source,'case.py','exec');python={'accepted':True}
 except SyntaxError as e:python={'accepted':False,'type':type(e).__name__,'message':e.msg,'line':e.lineno,'offset':e.offset,'end_line':e.end_lineno,'end_offset':e.end_offset}
 with tempfile.TemporaryDirectory()as tmp:
  root=Path(tmp);(root/'case.py').write_text(source);cmd=[str(ruff),'check','--isolated','--select','F621','--target-version','py310','--output-format','json','--','case.py'];run=subprocess.run(cmd,cwd=root,text=True,capture_output=True)
  observed.append({'name':name,'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'python':python,'reference':{'command':cmd,'exit_code':run.returncode,'stdout':run.stdout,'stderr':run.stderr,'findings':json.loads(run.stdout)}})
out={'python':sys.version,'executable':sys.executable,'ruff_sha256':hashlib.sha256(ruff.read_bytes()).hexdigest(),'observations':observed};Path('/tmp/f621-independent-controls-20261102/results.json').write_text(json.dumps(out,indent=2)+'\n')
for x in observed:print(x['name'],x['python'],[(d['code'],d['location'])for d in x['reference']['findings']])
