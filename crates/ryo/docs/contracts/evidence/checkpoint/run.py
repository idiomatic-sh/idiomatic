import json,os,subprocess,time,fcntl
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
commands=json.loads((p/'commands.json').read_text())
(p/'identity.json').write_text(json.dumps({'source':head,'commands':commands},indent=2))
for name,cmd in commands:
 print('START '+name,flush=True)
 started=time.time()
 with (p/(name+'.log')).open('w') as out: r=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT)
 result={'source':head,'command':cmd,'exit':r.returncode,'elapsed_seconds':time.time()-started}
 (p/(name+'.json')).write_text(json.dumps(result,indent=2))
 print('END '+name+' exit='+str(r.returncode),flush=True)
 if name=='build' and r.returncode: break
print('BATCH COMPLETE',flush=True)
