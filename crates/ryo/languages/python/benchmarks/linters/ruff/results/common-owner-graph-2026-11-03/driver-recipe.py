from pathlib import Path
import fcntl,subprocess,json,datetime,os,time,hashlib
out=Path('/tmp/django-owner-graph-quiet-20261103')
log=(out/'schedule.jsonl').open('a',buffering=1)
def event(stage,**kw):
 row={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'monotonic':time.monotonic(),'event':stage,**kw};log.write(json.dumps(row)+'\n');print(json.dumps(row),flush=True)
def run(script,*args):
 event('command_start',script=script,arguments=args)
 with (out/('driver-'+('-'.join(args) or 'rust')+'.stdout')).open('wb') as stdout,(out/('driver-'+('-'.join(args) or 'rust')+'.stderr')).open('wb') as stderr:
  done=subprocess.run(['python3',script,*args],stdout=stdout,stderr=stderr,timeout=1800)
 event('command_end',script=script,arguments=args,exit=done.returncode)
 if done.returncode:raise RuntimeError('command failed')
with open('/workspaces/.cargo-build.lock','a') as outer,open('/tmp/ryo-cargo-serial.lock','a') as inner:
 fcntl.flock(outer,fcntl.LOCK_EX);fcntl.flock(inner,fcntl.LOCK_EX)
 event('quiet_lock_acquired',outer='/workspaces/.cargo-build.lock',inner='/tmp/ryo-cargo-serial.lock',heavy_local_work_prohibited=True)
 script='/tmp/run-django-owner-graph-quiet-20261103.py'
 for variant in ('base','graph'):run(script,variant,'prepare')
 event('both_roots_prepared_before_any_timing')
 (out/'processes-before.txt').write_text(subprocess.check_output(['ps','-eo','pid,pcpu,pmem,etime,args'],text=True))
 for phase in ('cold','warm','edit-retained'):
  for variant in ('base','graph'):
   run(script,variant,phase)
   receipt=json.loads((out/variant/phase/'result.json').read_text())
   assert receipt.get('worker_exit_code')==0 and receipt.get('raw_report_bytes_equal') and receipt.get('matches_retained_baseline_report') and not receipt.get('capped'),receipt
   event('validated_phase',variant=variant,phase=phase,wall_seconds=receipt['wall_seconds'],cpu_seconds=receipt['cpu_seconds'],peak_rss_bytes=receipt['peak_rss_bytes'])
 run('/tmp/run-owner-rust-graph-quiet-20261103.py')
 (out/'processes-after.txt').write_text(subprocess.check_output(['ps','-eo','pid,pcpu,pmem,etime,args'],text=True))
 event('quiet_window_complete')
