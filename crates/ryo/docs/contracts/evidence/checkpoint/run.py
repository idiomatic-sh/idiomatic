import json,os,re,subprocess,sys,time,fcntl
from datetime import date
from pathlib import Path
p=Path(__file__).resolve().parent
contracts=p.parents[1]
readme=contracts/'README.md'
validation_json=p/'validation.json'
TM_TARGETS=('tm_source','tm_cli_source')
def dumps(v,pad=''):
 """JSON with two-space indent where a list of scalars or a short dict of scalars stays on one line, so a command line is one line."""
 if isinstance(v,list) and all(not isinstance(x,(list,dict)) for x in v): return json.dumps(v)
 if isinstance(v,list): return '[\n'+',\n'.join(pad+'  '+dumps(x,pad+'  ') for x in v)+'\n'+pad+']'
 if isinstance(v,dict) and len(v)<=4 and all(not isinstance(x,(list,dict)) for x in v.values()) and len(json.dumps(v))<=100: return json.dumps(v)
 if isinstance(v,dict): return '{\n'+',\n'.join(pad+'  '+json.dumps(k)+': '+dumps(x,pad+'  ') for k,x in v.items())+'\n'+pad+'}'
 return json.dumps(v)
def is_test_path(path): return '/test' in path or '_test.rs' in path
def tm_target(command,log):
 """Finding count, production/test split and findings per scoped directory of one `tm check` log."""
 report=json.loads(Path(log).read_text())
 root=command[command.index('--root')+1]
 skip=set(); words=[]
 for i,w in enumerate(command[command.index('check')+1:]):
  if w in ('--format',): skip.add(i+1)
  if not w.startswith('--') and i not in skip: words.append(w)
 findings=[f['path'] for f in report['findings']]
 directories={d:sum(1 for f in findings if f.startswith(root+'/'+d.rstrip('/')+'/') or f==root+'/'+d) for d in words}
 test=sum(1 for f in findings if is_test_path(f))
 return {'findings':len(findings),'suppressed':len(report.get('suppressed',[])),'production':len(findings)-test,'test':test,'scoped_directories':directories}
def record(v,commands,out_dir):
 v['tm_targets']={n:tm_target(c,out_dir/(n+'.log')) for n,c in commands if n in TM_TARGETS and (out_dir/(n+'.log')).exists()}
def scope_delta(v):
 """The scoped directories that are new since the validation.json at HEAD, each with its finding count."""
 r=subprocess.run(['git','-C',str(p),'show','HEAD:./validation.json'],capture_output=True,text=True)
 old=json.loads(r.stdout).get('tm_targets',{}) if r.returncode==0 else {}
 lines=[]
 for n,t in v.get('tm_targets',{}).items():
  before=old.get(n,{}).get('scoped_directories',{})
  for d,c in t['scoped_directories'].items():
   if d not in before: lines.append('newly scoped for '+n+': '+d+' ('+str(c)+' findings)')
  for d in before:
   if d not in t['scoped_directories']: lines.append('no longer scoped for '+n+': '+d)
 return lines or ['scoped directories unchanged since HEAD']
def fmt(x):
 if isinstance(x,dict) and 'passed' in x: return str(x['passed'])+' PASS / '+str(x['failed'])+' failed'+(' ('+x['note']+')' if 'note' in x else '')
 if isinstance(x,dict): return ', '.join(k.replace('_',' ')+' '+fmt(y) for k,y in x.items())
 if isinstance(x,list): return str(len(x))+': '+'; '.join('`'+str(y)+'`' for y in x) if x else '0'
 return str(x)
def render(v):
 rows=[('Source','`'+v['source'][:10]+'`'),('Validation','validation'+str(v['validation'])+' (raw logs in the directory named `'+Path(v['raw_identity']).name+'`, kept outside the repository)')]
 rows+=[(k.replace('_',' '),fmt(x)) for k,x in v['results'].items()]
 for n,t in v.get('tm_targets',{}).items():
  rows.append((n+' findings',str(t['findings'])+' ('+str(t['production'])+' production, '+str(t['test'])+' test), '+str(t['suppressed'])+' suppressed, '+str(len(t['scoped_directories']))+' scoped directories'))
 w=v['tm_whole_tree']
 rows.append(('Whole-tree TM',str(w['total'])+' findings, '+str(w['suppressed'])+' suppressed ('+str(w['at_checkpoint_318'])+' at 318)'))
 g=v['ownership_graph']
 rows.append(('File-level ownership cycles',str(g['other_cycles'])+' ('+fmt(g['targeted_cycles'])+' targeted)'))
 table=['| Check | Result |','|---|---|']+['| '+k+' | '+fmt(x).replace('|','\\|')+' |' for k,x in rows]
 head='### Current ownership checkpoint — source `'+v['source'][:10]+'`, validation'+str(v['validation'])
 return '\n'.join([head,'','Written by `evidence/checkpoint/run.py` from [validation.json](evidence/checkpoint/validation.json); do not edit by hand. Review: [checkpoint](audits/2026-11-06-derived/ownership/checkpoint.json).','']+table)+'\n\n'
TICKED=re.compile(r'^(- \[x\]|\|[^|]*\|\s*\[x\]|\|\s*☑\s*\|)',re.I)
CITED=re.compile(r'(?:validation|proof|review|reviewed|pass|runtime|reuse|reused)\s?(\d{3})\b|\b(?:in|at)(\d{3})\b',re.I)
def ticked_rows_without_proof(text,produced):
 """Ticked rows citing no validation, or one newer than the validation produced."""
 bad=[]
 for n,line in enumerate(text.splitlines(),1):
  if not TICKED.match(line): continue
  cited=[int(a or b) for a,b in CITED.findall(line)]
  if not cited: bad.append(str(n)+': cites no validation: '+line[:110])
  elif max(cited)>produced: bad.append(str(n)+': cites validation'+str(max(cited))+' beyond '+str(produced)+': '+line[:110])
 return bad
def write_block(text,v):
 """Replace the block from the `### Current ownership checkpoint` heading to the first following bullet."""
 lines=text.splitlines(keepends=True)
 start=next(i for i,l in enumerate(lines) if l.startswith('### Current ownership checkpoint'))
 end=next(i for i in range(start+1,len(lines)) if lines[i].startswith('- ['))
 return ''.join(lines[:start])+render(v)+''.join(lines[end:])
def finish(v):
 """Write the matrix numbers block from validation.json, print the scope delta, check the ticked rows. Returns the exit code."""
 old=readme.read_text(); text=write_block(old,v)
 readme.write_text(text)
 for line in scope_delta(v): print(line)
 bad=ticked_rows_without_proof(text,v['validation'])
 for part in sorted(contracts.glob('*.md')):
  if part!=readme: bad+=[part.name+' '+x for x in ticked_rows_without_proof(part.read_text(),v['validation'])]
 for line in bad: print('TICKED ROW '+line)
 if text!=old: print('matrix numbers block rewritten from validation.json; stage the README')
 return 1 if bad or text!=old else 0
if __name__=='__main__' and sys.argv[1:]==['--check']: sys.exit(finish(json.loads(validation_json.read_text())))
locks=[open('/workspaces/.cargo-build.lock','a'),open('/tmp/ryo-cargo-serial.lock','a')]
def locked(run):
 """Runs one command under the build locks, so other agents' short runs interleave between this batch's commands."""
 for lock in locks: fcntl.flock(lock,fcntl.LOCK_EX)
 try: return run()
 finally:
  for lock in locks: fcntl.flock(lock,fcntl.LOCK_UN)
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip(), 'freeze source before validation'
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
# The shared target directory serves several worktrees whose crates have equal build hashes; cargo
# trusts source mtimes, so artifacts another worktree built after this tree's edits would be reused.
# Touching the sources under the lock makes every artifact this run uses come from this tree.
def touch_sources():
 for d,dirs,fs in os.walk('crates/ryo'):
  dirs[:]=[x for x in dirs if x not in ('target','.git','node_modules')]
  for f in fs:
   if f.endswith(('.rs','.toml')): os.utime(os.path.join(d,f))
locked(touch_sources)
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
(p/'identity.json').write_text(dumps({'source':head,'commands':commands})+'\n')
# Raw outputs (.log/.json per command) never live in the repository: they go to the directory given as the first argument,
# by default a directory named for the source head and the date under the system temporary directory.
out_dir=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/tmp/ryo-contract-validation-'+head[:10]+'-'+date.today().strftime('%Y%m%d'))
out_dir.mkdir(parents=True,exist_ok=True)
for name,cmd in commands:
 print('START '+name,flush=True)
 started=time.time()
 with (out_dir/(name+'.log')).open('w') as out: r=locked(lambda: subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT))
 result={'source':head,'command':cmd,'exit':r.returncode,'elapsed_seconds':time.time()-started}
 if name=='tm_source': result['findings']=finding_split(cmd,out_dir/(name+'.log'))
 (out_dir/(name+'.json')).write_text(json.dumps(result,indent=2))
 print('END '+name+' exit='+str(r.returncode),flush=True)
 if name=='build' and r.returncode: break
print('BATCH COMPLETE',flush=True)
v=json.loads(validation_json.read_text())
v.update({'source':head,'validation':v['validation']+1,'raw_identity':str(out_dir)})
record(v,commands,out_dir)
validation_json.write_text(dumps(v)+'\n')
sys.exit(finish(v))
