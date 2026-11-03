from pathlib import Path
import hashlib,json,subprocess,tempfile,tomllib,os
out=Path(__file__).parent;query=Path('/tmp/n999-root-package-source-20261103/crates/ryo/languages/python/benchmarks/linters/ruff/rules/N999');ruff=Path('/home/vscode/.local/bin/ruff').resolve();results=[]
for case in tomllib.loads((query/'cases.toml').read_text())['cases'][-8:]:
 with tempfile.TemporaryDirectory() as scratch:
  container=Path(scratch);root=container/'inventory';root.mkdir()
  for name,text in case['context']['auxiliary-files'].items():
   p=container/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
  for name,text in case['files'].items():
   p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
  cmd=[str(ruff),'check','--isolated','--output-format','json','--select','N999','--target-version','py310','--',*case['files']]
  env=os.environ.copy();env['RAYON_NUM_THREADS']='1';p=subprocess.Popen(cmd,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE);pid=p.pid;stdout,stderr=p.communicate(timeout=10)
  stem=out/case['name'];stem.mkdir(exist_ok=True);(stem/'stdout.bin').write_bytes(stdout);(stem/'stderr.bin').write_bytes(stderr);(stem/'case.json').write_text(json.dumps(case,indent=2)+'\n')
  observed=[]
  if p.returncode in (0,1):
   for row in json.loads(stdout):
    filename=str(Path(row['filename']).relative_to(root));loc=row['location'];end=row['end_location'];assert loc=={'row':1,'column':1} and end==loc;observed.append(dict(rule=row['code'],file=filename,start=0,end=0))
  result=dict(name=case['name'],command=cmd,cwd=str(root),pid=pid,exit=p.returncode,observed=observed,expected=case['expected'],exact=p.returncode in (0,1) and observed==case['expected'],reference_sha256=hashlib.sha256(ruff.read_bytes()).hexdigest())
  (stem/'invocation.json').write_text(json.dumps(result,indent=2)+'\n');results.append(result)
(out/'summary.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps({'exact':sum(r['exact'] for r in results),'cases':len(results),'mismatches':[r for r in results if not r['exact']]}))
