import pathlib,json,hashlib,tomllib,subprocess
r=pathlib.Path(__file__).resolve().parents[2]; casepath=r/'rules/UP031/cases.toml'; text=subprocess.check_output(['git','show','ce33d1d8aeb027e90d4e021d0f5bfd96a1d817d8:crates/ryo/languages/python/benchmarks/linters/ruff/rules/UP031/cases.toml'],cwd=r,text=True); cases=tomllib.loads(text)['cases']; out=r/'results/up031-current-context-2026-11-04'; out.mkdir(exist_ok=True)
def sha(b):return hashlib.sha256(b).hexdigest()
def digest(x):return sha(json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode())
performance={'node_result_reuse':'disabled','column_kernel':False,'diagnostics':{x:False for x in ['execution','lifecycle_memory','resource_routes','row_progress','steps','joins','prefixes','subplans','selection_detail']}}
proof=[]
for c in cases:
 disc=c.get('reference-discrepancy',{}); process=disc.get('process')
 if not process:continue
 identity=disc['identity']
 for ob in process['observations']:
  old=ob['invocation']; raw=(r/old['path']).read_bytes(); assert sha(raw)==old['sha256']; d=json.loads(raw)
  assert digest(d['normalized_settings'])==d['settings_sha256']; assert d['source_sha256']==identity['source-sha256']
  original=d['normalized_settings'].copy(); assert 'performance' not in original
  d['normalized_settings']['performance']=performance; assert digest(d['normalized_settings'])==identity['settings-sha256']
  d['profile_sha256']=identity['profile-sha256']; d['settings_sha256']=identity['settings-sha256']
  d['context_derivation']={'kind':'canonical-invocation-context-rebind','original_invocation':old,'original_profile_sha256':json.loads(raw)['profile_sha256'],'original_settings_sha256':json.loads(raw)['settings_sha256'],'changes':['profile includes explicit query-module declarations; reference argv unchanged','normalized settings adds typed default performance; reference argv/environment/directory/source/process bytes unchanged'],'not_a_new_process_capture':True}
  suffix=pathlib.Path(old['path']).relative_to('results/up031-process-harness-2026-11-03'); target=out/suffix;target.parent.mkdir(parents=True,exist_ok=True); newraw=(json.dumps(d,indent=2,sort_keys=True)+'\n').encode();target.write_bytes(newraw); newpath=target.relative_to(r).as_posix()
  assert text.count(old['path'])==1;text=text.replace(old['path'],newpath).replace(old['sha256'],sha(newraw));proof.append({'case':c['name'],'original':old,'derived':{'path':newpath,'sha256':sha(newraw)},'executable_sha256':ob['executable-sha256'],'original_settings_sha256':json.loads(raw)['settings_sha256'],'current_settings_sha256':identity['settings-sha256']})
casepath.write_text(text)
(out/'derivation.json').write_text(json.dumps({'scope':'Derived current context, not new process execution. Original raw captures remain immutable. stdout/stderr observations still reference their original archives.','records':proof},indent=2)+'\n')
(out/'README.md').write_text('These eight invocation records derive current canonical settings context from the original immutable UP031 process captures. They are not new process captures. Each record pins its original invocation hash; stdout, stderr, termination, executable, arguments, environment, directory and source remain unchanged. The derivation verifies the original normalized settings digest and proves adding only typed default performance yields the active settings digest. Current profile identity reflects explicit query-module declarations; reference argv is unchanged. Original archives are preserved.\n')
print(len(proof))
