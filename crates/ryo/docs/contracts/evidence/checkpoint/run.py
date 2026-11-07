import json,os,subprocess,sys,time,fcntl
from datetime import date
from pathlib import Path
p=Path(__file__).resolve().parent
locks=[open('/workspaces/.cargo-build.lock','a'),open('/tmp/ryo-cargo-serial.lock','a')]
for lock in locks: fcntl.flock(lock,fcntl.LOCK_EX)
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip(), 'freeze source before validation'
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
# The shared target directory serves several worktrees whose crates have equal build hashes; cargo
# trusts source mtimes, so artifacts another worktree built after this tree's edits would be reused.
# Touching the sources under the lock makes every artifact this run uses come from this tree.
for d,dirs,fs in os.walk('crates/ryo'):
 dirs[:]=[x for x in dirs if x not in ('target','.git','node_modules')]
 for f in fs:
  if f.endswith(('.rs','.toml')): os.utime(os.path.join(d,f))
def is_test(path):
 parts=path.split('/')
 return any(x in ('tests','test','fixtures') for x in parts[:-1]) or parts[-1] in ('test.rs','tests.rs') or parts[-1].endswith('_test.rs')
def finding_split(cmd,log):
 # `tm check` prints its findings as one JSON document; the split is per scoped directory, test trees apart from production.
 scoped=cmd[cmd.index('check')+1:cmd.index('--format')]
 found=json.loads(log.read_text())
 by_directory={x:{'production':0,'test':0} for x in scoped}
 for finding in found['findings']:
  owner=[x for x in scoped if finding['path'].startswith('crates/ryo/'+x+'/') or finding['path']=='crates/ryo/'+x]
  by_directory[owner[0]]['test' if is_test(finding['path']) else 'production']+=1
 return {'total':len(found['findings']),'suppressed':len(found['suppressed']),'production':sum(v['production'] for v in by_directory.values()),'test':sum(v['test'] for v in by_directory.values()),'by_directory':by_directory}
commands=json.loads((p/'commands.json').read_text())
(p/'identity.json').write_text(json.dumps({'source':head,'commands':commands},indent=2))
# Raw outputs (.log/.json per command) never live in the repository: they go to the directory given as the first argument,
# by default /tmp/ryo-contract-validation-<source head>-<date>.
out_dir=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/tmp/ryo-contract-validation-'+head[:10]+'-'+date.today().strftime('%Y%m%d'))
out_dir.mkdir(parents=True,exist_ok=True)
for name,cmd in commands:
 print('START '+name,flush=True)
 started=time.time()
 with (out_dir/(name+'.log')).open('w') as out: r=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT)
 result={'source':head,'command':cmd,'exit':r.returncode,'elapsed_seconds':time.time()-started}
 if name=='tm_source': result['findings']=finding_split(cmd,out_dir/(name+'.log'))
 (out_dir/(name+'.json')).write_text(json.dumps(result,indent=2))
 print('END '+name+' exit='+str(r.returncode),flush=True)
 if name=='build' and r.returncode: break
print('BATCH COMPLETE',flush=True)
