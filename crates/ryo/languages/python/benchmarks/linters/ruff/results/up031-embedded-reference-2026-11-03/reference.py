import hashlib,json,pathlib,subprocess,tomllib
root=pathlib.Path('/tmp/up031-layer-followup-20261103/crates/ryo/languages/python/benchmarks/linters/ruff/rules/UP031')
out=pathlib.Path('/tmp/up031-followup-controls-20261103')
cases=tomllib.loads((root/'cases.toml').read_text())['cases'][-6:]
results=[]
for target in ['py310','py37']:
 for case in cases:
  source=case['files']['case.py']; stem=target+'-'+case['name']
  (out/(stem+'.py')).write_text(source)
  p=subprocess.run(['/home/vscode/.local/bin/ruff','check','--isolated','--select','UP031','--target-version',target,'--output-format','json','--stdin-filename','case.py','-'],input=source.encode(),capture_output=True)
  (out/(stem+'.stdout')).write_bytes(p.stdout);(out/(stem+'.stderr')).write_bytes(p.stderr)
  print(stem,p.returncode,repr(p.stderr),flush=True)
  if p.returncode not in (0,1):
   results.append(dict(target=target,name=case["name"],exit=p.returncode,source_sha256=hashlib.sha256(source.encode()).hexdigest(),expected=case["expected"],exact=False,refusal="reference-process-failure"));continue
  rows=json.loads(p.stdout);lines=source.splitlines(keepends=True)
  def offset(loc):return len(''.join(lines[:loc['row']-1]).encode())+len(lines[loc['row']-1][:loc['column']-1].encode())
  observed=[dict(rule=r['code'],file='case.py',start=offset(r['location']),end=offset(r['end_location'])) for r in rows]
  results.append(dict(target=target,name=case['name'],source_sha256=hashlib.sha256(source.encode()).hexdigest(),exit=p.returncode,expected=case['expected'],observed=observed,exact=observed==case['expected']))
(out/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps({'controls':len(results),'exact':sum(r['exact'] for r in results),'mismatches':[r for r in results if not r['exact']]},ensure_ascii=False))
