"""Reproduce named controls into a NEW output directory; archived reference.py is immutable."""
from pathlib import Path
import argparse,hashlib,json,subprocess,tempfile,tomllib,os
NAMES = (
    "nested-uppercase-child-root-policy", "nested-bad-package-root",
    "namespace-prefix-bad-package-root", "nested-dotted-package-root",
    "nested-main-root-policy", "ordinary-module-retains-own-name",
    "numeric-migration-retains-immediate-directory-policy", "namespace-prefix-good-root",
)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ruff',type=Path,required=True)
parser.add_argument('--output-dir',type=Path,required=True)
parser.add_argument('--cases',type=Path,default=Path(__file__).resolve().parents[2]/'rules/N999/cases.toml')
args=parser.parse_args()
ruff=args.ruff.resolve(strict=True);out=args.output_dir.resolve()
out.mkdir(parents=True,exist_ok=False)
all_cases=tomllib.loads(args.cases.read_text())['cases'];by_name={case['name']:case for case in all_cases}
if len(by_name)!=len(all_cases) or set(NAMES)-by_name.keys():raise ValueError('missing or duplicate named controls')
selected=[by_name[name] for name in NAMES];results=[]
for case in selected:
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
