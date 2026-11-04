import sys
from pathlib import Path
import os,json,hashlib,shutil,subprocess,time,signal,fcntl,collections,datetime
group=sys.argv[1]; cohorts={'binding-source4':['F401','F811','F821','F841'],'mixed-relations4':['RUF028','RET501','SIM114','B018']};out=Path('/tmp/django-family-cohorts-20261104')/group;out.parent.mkdir(exist_ok=True);out.mkdir(exist_ok=True);sample=Path('/tmp/django-rule-sample12-20261104');sm=json.loads((sample/'manifest.json').read_text());rules=cohorts[group];pack=Path('/tmp/ryo-current409-pack-v3-20261103');fields=json.loads((pack/'worker-request-fields.json').read_text());corpus=Path('/tmp/django-file-program-graph-pair-20261102/graph-on/corpus');files=json.loads(Path('/tmp/django-numeric-sparse-full-20261102/files.json').read_text());worker=Path('/tmp/axis-nearest-workers-20261104/ruff-worker');collector=Path('/tmp/perfile-index-cache-collector-build-20261101/collector/collector-4e1d3d328618bee455f9ee1f6e9c1963f076803e7251bdc953cb55db362d4205');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();native=lambda s:{'Unix':list(os.fsencode(s))}
identity={'source':sm['source'],'worker_sha256':sha(worker),'collector_sha256':sha(collector),'rules':rules,'parent_pack_sha256':sha(pack/'manifest.json'),'files_sha256':sm['files_sha256'],'corpus_files':2875,'uncertified_diagnostic':True,'isolated_timings_not_added_or_subtracted':True,'execution_policy':'User-requested progressive30→90→180→doubling, no hard final cutoff','all53_helpers_retained':True,'cohort':group,'runtime_policy':'same common DAG; optional node-result retention enabled by historical sealed worker flags; not alternative executor'};(out/'manifest.json').write_text(json.dumps(identity,indent=2)+'\n')
def descendants(pid):
 try:cs=[int(x)for x in Path(f'/proc/{pid}/task/{pid}/children').read_text().split()]
 except OSError:return []
 return [q for c in cs for q in descendants(c)]+cs
cap=30;attempts=[]
while True:
 attempt=out/f'attempt-{cap}';attempt.mkdir(exist_ok=True)
 with open('/workspaces/.cargo-build.lock','a')as outer,open('/tmp/ryo-cargo-serial.lock','a')as inner:
  fcntl.flock(outer,fcntl.LOCK_EX);fcntl.flock(inner,fcntl.LOCK_EX)
  profile=out/'profile'
  for rule in rules:(profile/'rules'/rule).mkdir(parents=True,exist_ok=True);shutil.copy2(pack/'profile/rules'/rule/'query.ryoql',profile/'rules'/rule/'query.ryoql')
  for rel in set(fields['query_modules'].values()):p=profile/rel;p.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(pack/'profile'/rel,p);assert sha(pack/'profile'/rel)==sha(p)
  request=dict(fields,root=str(corpus),files=files,rules=str(profile/'rules'),source_language='python',cache=str(attempt/'cache'),rule_applicability={r:fields['rule_applicability'][r]for r in rules});(attempt/'request.json').write_text(json.dumps(request,indent=2)+'\n')
  wire={'executable':native(str(worker)),'arguments':[native(x)for x in ['run-ryo','--request-file',str(attempt/'request.json'),'python-target=3.10']],'environment':[{'name':native(k),'value':native(v)}for k,v in {'RAYON_NUM_THREADS':'1','RYO_FILE_PROGRAM_GRAPH':'0','RYO_FILE_DAG_EXECUTION':'1'}.items()],'directory':native(str(corpus))};(attempt/'collector-request.json').write_text(json.dumps(wire,indent=2)+'\n');start=time.monotonic();capped=False
  print(json.dumps({'event':'combined12_started','utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'cap_seconds':cap,**identity}),flush=True)
  with (attempt/'observation.json').open('wb')as stdout,(attempt/'collector.stderr').open('wb')as stderr:
   child=subprocess.Popen([str(collector)],stdin=subprocess.PIPE,stdout=stdout,stderr=stderr,start_new_session=True)
   try:child.communicate(input=json.dumps(wire).encode(),timeout=cap)
   except subprocess.TimeoutExpired:
    capped=True
    for pid in descendants(child.pid):
     try:os.kill(pid,signal.SIGTERM)
     except ProcessLookupError:pass
    child.wait()
  result={'cap_seconds':cap,'capped':capped,'status':'FAIL_TIMEOUT'if capped else 'FAIL_OBSERVATION','seconds_orchestration':time.monotonic()-start,'request_sha256':sha(attempt/'request.json')}
  try:
   obs=json.loads((attempt/'observation.json').read_text());assert obs['result']=='Observed';v=obs['output'];raw=bytes(v['stdout']);err=bytes(v['stderr']);(attempt/'stdout').write_bytes(raw);(attempt/'stderr').write_bytes(err);sec=lambda d:d['secs']+d['nanos']/1e9;result.update(wall_seconds=sec(v['wall']),cpu_seconds=sec(v['cpu']),peak_rss_bytes=v['rss'],worker_exit=v['status'])
   if not capped and v['status']==0:
    report=json.loads(raw);(attempt/'report.json').write_text(json.dumps(report,indent=2)+'\n');cov=report['coverage'];complete=report['execution_metadata']['availability']=='complete'and cov['selected']==2875*len(rules) and cov['evaluated']+cov['skipped']==cov['selected']and cov['unknown_or_unread']==0 and sorted(report['files'])==sorted(files)
    bag=lambda rows:collections.Counter(json.dumps(x,sort_keys=True)for x in rows);byrule={};skip_equal={}
    state=json.loads(Path('/tmp/django-all413-progressive-20261104/inventory-state.json').read_text())
    def original(rule):
     path=sample/rule/'stdout'
     if not path.exists():path=Path(state['rules'][rule]['receipt_path']).parent/'stdout'
     return json.loads(path.read_text())
    for rule in rules:
     old=original(rule);byrule[rule]=bag([x for x in old['diagnostics']if x['rule']==rule])==bag([x for x in report['diagnostics']if x['rule']==rule]);skip_equal[rule]=bag([x for x in old['skipped']if x['rule']==rule])==bag([x for x in report['skipped']if x['rule']==rule])
    old=json.loads((sample/rule/'report.json').read_text())if(sample/rule/'report.json').exists()else json.loads((sample/rule/'stdout').read_text());byrule[rule]=bag([x for x in old['diagnostics']if x['rule']==rule])==bag([x for x in report['diagnostics']if x['rule']==rule]);skip_equal[rule]=bag([x for x in old['skipped']if x['rule']==rule])==bag([x for x in report['skipped']if x['rule']==rule])
    old=original(rules[0]);syntax_equal=bag([x for x in old['diagnostics']if x['rule']=='invalid-syntax'])==bag([x for x in report['diagnostics']if x['rule']=='invalid-syntax'])
    result.update(status='COMPLETED_UNCERTIFIED'if complete and all(byrule.values())and all(skip_equal.values())and syntax_equal else 'FAIL_REPORT_COMPARISON',coverage=cov,complete_report=complete,isolated_rule_output_bags_equal=byrule,isolated_skip_bags_equal=skip_equal,global_invalid_syntax_bag_once_equal=syntax_equal,diagnostics=len(report['diagnostics']))
   elif not capped:result['status']='FAIL_WORKER'
  except Exception as error:result['observation_error']=str(error)
  (attempt/'result.json').write_text(json.dumps(result,indent=2)+'\n');attempts.append(result);(out/'summary.json').write_text(json.dumps({**identity,'attempts':attempts,'latest':result},indent=2)+'\n');print(json.dumps({'event':'combined12_terminal',**result}),flush=True)
 if not capped:break
 cap=90 if cap==30 else 180 if cap==90 else cap*2
 time.sleep(0.1)
