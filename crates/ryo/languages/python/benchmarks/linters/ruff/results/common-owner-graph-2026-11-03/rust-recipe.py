"""Run the established shipped Ryo Rust --bench path on sealed matched binaries."""
from pathlib import Path
import datetime,hashlib,json,os,re,statistics,subprocess,sys,time
base=Path('/tmp/ryo-owner-selector-check-20261103')
candidate=Path('/tmp/common-owner-graph-corrections-20261103')
root=Path('/tmp/owner-rust-lint-20261103')
out=Path('/tmp/owner-rust-graph-quiet-20261103')
assert not out.exists();out.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old=Path('/tmp/rust-original-input-identities-20261102.json')
identities=json.loads(old.read_text())['recordings_base']
variants={}
for name,source in [('base',base),('candidate',candidate)]:
 package=(root/'candidate' if name=='base' else Path('/tmp/owner-perf-bench-graph-fef-20261103'));seal=json.loads((package/('seal.json' if name=='base' else 'lint-seal.json')).read_text())
 assert seal['target_kind']=='bench' and seal['features']=='default'
 assert seal['binary_sha256']==sha(package/'lint')
 assert seal['source_head']==subprocess.check_output(['git','rev-parse','HEAD'],cwd=source,text=True).strip()
 assert not subprocess.check_output(['git','status','--porcelain'],cwd=source)
 fixture=source/'crates/ryo/cli/tests/fixtures/recordings'
 for relative,expected in identities.items():
  assert sha(fixture/relative)==expected,relative
 variants[name]=(package/'lint',source/'crates/ryo',seal)
manifest={'schema':'owner-rust-original-benchmark/v1','source_commits':{v:s['source_head'] for v,(_,_,s) in variants.items()},'binary_sha256':{v:s['binary_sha256'] for v,(_,_,s) in variants.items()},'recordings_sha256':sha(old),'recordings_checked':len(identities),'command':['--bench','--rounds','1'],'binary_features':'default','script_sha256':sha(Path(__file__)),'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat()}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
results=[]
for variant in ('base','candidate'):
 binary,cwd,seal=variants[variant];folder=out/variant;folder.mkdir()
 before=os.getloadavg();started=time.monotonic()
 with (folder/'stdout.bin').open('wb') as stdout,(folder/'stderr.bin').open('wb') as stderr:
  result=subprocess.run([str(binary),'--bench','--rounds','1'],cwd=cwd,stdout=stdout,stderr=stderr,timeout=600)
 receipt={'variant':variant,'source_commit':seal['source_head'],'binary_sha256':seal['binary_sha256'],'exit_code':result.returncode,'wall_seconds':time.monotonic()-started,'load_before':before,'load_after':os.getloadavg(),'stdout_sha256':sha(folder/'stdout.bin'),'stderr_sha256':sha(folder/'stderr.bin')}
 if result.returncode==0:
  metrics=json.loads((folder/'stdout.bin').read_text())['ruff']
  receipt['metrics']={k:v['value'] for k,v in metrics.items()}
  text=(folder/'stderr.bin').read_text()
  receipt['internal_rounds']={}
  for metric in ['cold-milliseconds','cold-cpu-milliseconds','spike-cpu-milliseconds']:
   m=re.search(r'ruff/'+re.escape(metric)+r': median (\d+) of \[([^\]]+)\]',text)
   if m:receipt['internal_rounds'][metric]=[int(x.strip()) for x in m.group(2).split(',')]
 (folder/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps(receipt,sort_keys=True),flush=True)
 results.append(receipt)
 assert result.returncode==0,variant
assert results[0]['metrics'].keys()==results[1]['metrics'].keys()
summary={'schema':'owner-rust-original-benchmark-summary/v1','manifest_sha256':sha(out/'manifest.json'),'metrics':{k:{'base':results[0]['metrics'][k],'candidate':results[1]['metrics'][k],'candidate_over_base':results[1]['metrics'][k]/results[0]['metrics'][k]} for k in results[0]['metrics']},'single_pair_only':True,'strict_peak_cap_not_replaced':True}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,sort_keys=True),flush=True)
