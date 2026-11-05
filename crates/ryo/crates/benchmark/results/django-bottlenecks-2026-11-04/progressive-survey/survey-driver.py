from pathlib import Path
import os,json,hashlib,shutil,subprocess,time,signal,fcntl,datetime,collections
out=Path('/tmp/django-all413-progressive-20261104');out.mkdir(exist_ok=True)
pack=Path('/tmp/ryo-current409-pack-v3-20261103');fields=json.loads((pack/'worker-request-fields.json').read_text());pm=json.loads((pack/'manifest.json').read_text());available=sorted(pm['query_ids']);assert len(available)==409
worker=Path('/tmp/axis-nearest-workers-20261104/ruff-worker');collector=Path('/tmp/perfile-index-cache-collector-build-20261101/collector/collector-4e1d3d328618bee455f9ee1f6e9c1963f076803e7251bdc953cb55db362d4205');corpus=Path('/tmp/django-file-program-graph-pair-20261102/graph-on/corpus');frozen=Path('/tmp/django-numeric-sparse-full-20261102');files=json.loads((frozen/'files.json').read_text());assert len(files)==2875
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();native=lambda s:{'Unix':list(os.fsencode(s))};now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat()
identity={'source':'db1f035022c4b758362a7f010226d7a853088b6d','worker_sha256':sha(worker),'collector_sha256':sha(collector),'parent_pack_sha256':sha(pack/'manifest.json'),'files_sha256':sha(frozen/'files.json'),'corpus_files':2875,'declared_slots':413,'public_runnable':411,'compatible_this_worker':409,'not_current411_acceptance':True,'uncertified_diagnostic':True,'worker_count':1,'progressive_caps':'30,90,180,then doubling; no hard final cutoff','timings_not_additive':True,'locks':'workspace then serial per invocation; released between rules','all_helper_bindings_retained':len(fields['query_modules'])}
(out/'manifest.json').write_text(json.dumps(identity,indent=2)+'\n')
state_path=out/'summary.json';state=json.loads(state_path.read_text()) if state_path.exists() else {**identity,'rules':{}}
assert state['worker_sha256']==identity['worker_sha256'] and state['parent_pack_sha256']==identity['parent_pack_sha256']
for r in ['I001','RUF100']:state['rules'][r]={'rule':r,'status':'FAIL_MISSING_IMPLEMENTATION','timing':None,'reason':'No published verified implementation in current413-slot inventory'}
for r in ['E902','RUF200']:state['rules'][r]={'rule':r,'status':'PENDING_COMPATIBLE_WORKER','timing':None,'reason':'Published runnable411; absent from frozen409 pack/engine cohort; never silently omitted or mixed'}
old=Path('/tmp/django-rule-sample12-20261104')
for r in available:
 p=old/r/'result.json'
 if r not in state['rules'] and p.exists():
  saved=json.loads(p.read_text());selection=json.loads((old/r/'selection.json').read_text());request=json.loads((old/r/'request.json').read_text())
  assert saved['worker_sha256']==identity['worker_sha256'] and selection['query_sha256']==sha(pack/'profile/rules'/r/'query.ryoql') and request['files']==files
  assert all(selection['module_hashes'][name]==sha(pack/'profile'/rel)for name,rel in fields['query_modules'].items())
  assert saved['status']=='COMPLETED_UNCERTIFIED' and not saved['capped'];state['rules'][r]={**saved,'reused_completed12':True,'receipt_path':str(p),'attempts':[]}
def save():
 state['updated_utc']=now();state['counts']=dict(collections.Counter(v['status']for v in state['rules'].values()));state['enumerated_slots']=len(set(available)|{'E902','I001','RUF100','RUF200'});state_path.write_text(json.dumps(state,indent=2)+'\n')
def descendants(pid):
 try:cs=[int(x)for x in Path(f'/proc/{pid}/task/{pid}/children').read_text().split()]
 except OSError:return []
 return [q for c in cs for q in descendants(c)]+cs
save();print(json.dumps({'event':'survey_started','utc':now(),'path':str(out),'reused_complete':12,**identity}),flush=True)
remaining=[r for r in available if state['rules'].get(r,{}).get('status')!='COMPLETED_UNCERTIFIED'];cap=30
while remaining:
 timed=[]
 for rule in remaining:
  previous=state['rules'].get(rule,{})
  if previous.get('status')=='COMPLETED_UNCERTIFIED':continue
  existing=previous.get('attempts',[])
  if existing and existing[-1]['cap_seconds']>=cap:
   if previous['status']=='FAIL_TIMEOUT':timed.append(rule)
   continue
  case=out/rule;attempt=case/f'attempt-{cap}';attempt.mkdir(parents=True,exist_ok=True)
  with open('/workspaces/.cargo-build.lock','a')as outer,open('/tmp/ryo-cargo-serial.lock','a')as inner:
   fcntl.flock(outer,fcntl.LOCK_EX);fcntl.flock(inner,fcntl.LOCK_EX)
   profile=case/'profile';(profile/'rules'/rule).mkdir(parents=True,exist_ok=True);shutil.copy2(pack/'profile/rules'/rule/'query.ryoql',profile/'rules'/rule/'query.ryoql')
   for relative in set(fields['query_modules'].values()):
    src=pack/'profile'/relative;dest=profile/relative;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dest);assert sha(src)==sha(dest)
   request=dict(fields,root=str(corpus),files=files,rules=str(profile/'rules'),source_language='python',cache=str(attempt/'cache'),rule_applicability={rule:fields['rule_applicability'][rule]});(attempt/'request.json').write_text(json.dumps(request,indent=2)+'\n')
   (attempt/'selection.json').write_text(json.dumps({'query_sha256':sha(pack/'profile/rules'/rule/'query.ryoql'),'module_hashes':{n:sha(profile/p)for n,p in fields['query_modules'].items()},'all_original_request_inputs_retained':True,'suite_fixed_point_certification':False},indent=2)+'\n')
   wire={'executable':native(str(worker)),'arguments':[native(x)for x in ['run-ryo','--request-file',str(attempt/'request.json'),'python-target=3.10']],'environment':[{'name':native(k),'value':native(v)}for k,v in {'RAYON_NUM_THREADS':'1','RYO_FILE_PROGRAM_GRAPH':'0','RYO_FILE_DAG_EXECUTION':'1'}.items()],'directory':native(str(corpus))};(attempt/'collector-request.json').write_text(json.dumps(wire,indent=2)+'\n');begin=time.monotonic();capped=False;started=now()
   print(json.dumps({'event':'rule_started','rule':rule,'cap_seconds':cap,'utc':started}),flush=True)
   with (attempt/'observation.json').open('wb')as stdout,(attempt/'collector.stderr').open('wb')as stderr:
    child=subprocess.Popen([str(collector)],stdin=subprocess.PIPE,stdout=stdout,stderr=stderr,start_new_session=True)
    try:child.communicate(input=json.dumps(wire).encode(),timeout=cap)
    except subprocess.TimeoutExpired:
     capped=True
     for pid in descendants(child.pid):
      try:os.kill(pid,signal.SIGTERM)
      except ProcessLookupError:pass
     child.wait()
   result={'rule':rule,'status':'FAIL_TIMEOUT'if capped else 'FAIL_OBSERVATION','cap_seconds':cap,'capped':capped,'started_utc':started,'ended_utc':now(),'worker_sha256':identity['worker_sha256'],'request_sha256':sha(attempt/'request.json'),'reference_parity_certified':False,'orchestration_wall_seconds':time.monotonic()-begin}
   try:
    obs=json.loads((attempt/'observation.json').read_text());assert obs['result']=='Observed';v=obs['output'];raw=bytes(v['stdout']);err=bytes(v['stderr']);(attempt/'stdout').write_bytes(raw);(attempt/'stderr').write_bytes(err);sec=lambda d:d['secs']+d['nanos']/1e9
    result.update(wall_seconds=sec(v['wall']),cpu_seconds=sec(v['cpu']),peak_rss_bytes=v['rss'],worker_exit=v['status'])
    if not capped and v['status']==0:
     report=json.loads(raw);(attempt/'report.json').write_text(json.dumps(report,indent=2)+'\n');cov=report['coverage'];complete=report['execution_metadata']['availability']=='complete' and cov['selected']==2875 and cov['evaluated']+cov['skipped']==2875 and cov['unknown_or_unread']==0 and sorted(report['files'])==sorted(files)
     result.update(status='COMPLETED_UNCERTIFIED'if complete else 'FAIL_INCOMPLETE',coverage=cov,diagnostics=len(report['diagnostics']),complete_report=complete)
    elif not capped:result['status']='FAIL_WORKER'
   except Exception as error:result['observation_error']=str(error)
   (attempt/'result.json').write_text(json.dumps(result,indent=2)+'\n');state['rules'][rule]={**result,'receipt_path':str(attempt/'result.json'),'attempts':existing+[result]};save();print(json.dumps({'event':'rule_terminal',**result}),flush=True)
  if capped:timed.append(rule)
  time.sleep(0.1)
 print(json.dumps({'event':'stage_complete','cap_seconds':cap,'timed_out_rules':timed,'counts':state['counts']}),flush=True)
 remaining=timed;cap=90 if cap==30 else 180 if cap==90 else cap*2
save();print(json.dumps({'event':'compatible409_survey_terminal','summary':str(state_path),'counts':state['counts']}),flush=True)
