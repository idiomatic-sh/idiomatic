"""Run the frozen Django310 profile on the sealed demand-admission worker."""
from pathlib import Path
import collections,hashlib,importlib.util,json,os,shutil,signal,subprocess,sys,time,tomllib
source=Path('/tmp/django-numeric-sparse-full-20261102')
variant=sys.argv[1]
assert variant in ('base','graph')
out=Path('/tmp/django-owner-graph-quiet-20261103')/variant
worker=Path('/tmp/owner-perf-bench-f09b-20261103/ruff-worker' if variant=='base' else '/tmp/owner-perf-bench-graph-fef-20261103/ruff-worker')
seal=Path('/tmp/owner-perf-bench-f09b-20261103/seal.json' if variant=='base' else '/tmp/owner-perf-bench-graph-fef-20261103/seal.json')
collector=Path('/tmp/perfile-index-cache-collector-build-20261101/collector/collector-4e1d3d328618bee455f9ee1f6e9c1963f076803e7251bdc953cb55db362d4205')
validator=Path('/tmp/django-io-capability-full-20261102/validator-repository/crates/ryo/languages/python/benchmarks/linters/ruff/verify_profiles.py')
manifest=json.loads((source/'manifest.json').read_text())
files=json.loads((source/'files.json').read_text())
normal=json.loads((source/'source-inventory.json').read_text())
edited=json.loads((source/'source-inventory-edited.json').read_text())
base_request=json.loads((source/'request.json').read_text())
patch=source/'django.patch'
phase=sys.argv[2]
assert phase in ('prepare','cold','warm','edit-retained','edit-fresh')
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path,value):path.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')
def verify_corpus(root,inventory):
    for item in inventory:
        path=root/item['file']
        if digest(path)!=item['sha256'] or path.stat().st_mode&0o777!=item['mode']:
            raise ValueError(f'corpus mismatch: {item["file"]}')
def native(value):return {'Unix':list(os.fsencode(value))}
def seconds(duration):return duration['secs']+duration['nanos']/1e9
assert len(files)==2875 and len(manifest['selection'])==310
assert json.loads(seal.read_text())['worker_sha256']==digest(worker)
assert json.loads(seal.read_text())['target_kind']=='bench'
script_sha256=digest(Path(__file__))
assert digest(collector)==manifest['binaries']['collector']['sha256']
assert digest(validator)==manifest['validator']['sha256']
assert digest(patch)==manifest['patch_sha256']
for relative,expected in manifest['profile_inputs'].items():assert digest(source/'profile'/relative)==expected,relative
spec=importlib.util.spec_from_file_location('frozen_validator',validator)
v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
settings=v.canonical_run_ryo_settings(tomllib.loads((source/'profile/benchmark.toml').read_text()),manifest['selection'],json.loads((source/'phase-registry.json').read_text()))
corpus=out/'corpus';cache=out/'retained-cache'
if phase=='prepare':
    assert not out.exists()
    out.mkdir()
    original=Path('/tmp/django-file-program-graph-pair-20261102/graph-on/corpus')
    verify_corpus(original,normal)
    shutil.copytree(original,corpus,copy_function=shutil.copy2)
    verify_corpus(corpus,normal)
    write(out/'manifest.json',{'schema':'django-owner-matched/v2','script_sha256':script_sha256,'target_kind':'bench','variant':variant,'source_commit':json.loads(seal.read_text())['source_head'],'profile_sha256':digest(source/'profile/benchmark.toml'),'files_sha256':digest(source/'files.json'),'source_manifest_sha256':digest(source/'manifest.json'),'worker_sha256':digest(worker),'collector_sha256':digest(collector),'validator_sha256':digest(validator),'frozen_relation_baseline_receipt':'/tmp/django-relation-contract-20261102/cold/result.json','derived_baseline_receipt':'/tmp/django-derived-view-contract-20261102/cold/result.json','rules':310,'corpus_files':2875,'rayon_threads':1,'strategy':'RYO_FILE_DAG_EXECUTION=1','measurement_kind':'ordinary-scoped','performance_acceptance':False})
else:
    assert out.exists()
    if phase != 'cold': assert cache.is_dir()
    assert json.loads((out/'manifest.json').read_text())['worker_sha256']==digest(worker)
if phase=='prepare':
    print(json.dumps({'variant':variant,'phase':'prepared','corpus':str(corpus),'files':len(files)}),flush=True)
    sys.exit(0)
if phase=='edit-retained':
    verify_corpus(corpus,normal)
    subprocess.run(['git','apply',str(patch)],cwd=corpus,check=True)
if phase=='edit-fresh':
    verify_corpus(corpus,edited)
verify_corpus(corpus,edited if phase.startswith('edit-') else normal)
phase_dir=out/phase
assert not phase_dir.exists();phase_dir.mkdir()
phase_cache=phase_dir/'fresh-cache' if phase=='edit-fresh' else cache
assert phase_cache.exists()==(phase in ('warm','edit-retained'))
request=dict(base_request,root=str(corpus),rules=str(source/'profile/rules'),cache=str(phase_cache))
write(phase_dir/'request.json',request)
command=[str(worker),'run-ryo','--request-file',str(phase_dir/'request.json'),'python-target=3.10']
wire={'executable':native(command[0]),'arguments':[native(a) for a in command[1:]],'environment':[{'name':native('RAYON_NUM_THREADS'),'value':native('1')},{'name':native('RYO_FILE_PROGRAM_GRAPH'),'value':native('0')},{'name':native('RYO_FILE_DAG_EXECUTION'),'value':native('1')}],'directory':native(corpus)}
write(phase_dir/'collector-request.json',wire)
load_before=os.getloadavg();started=time.monotonic();capped=False
with (phase_dir/'observation.json').open('wb') as stdout,(phase_dir/'collector.stderr').open('wb') as stderr:
    child=subprocess.Popen([str(collector)],stdin=subprocess.PIPE,stdout=stdout,stderr=stderr,start_new_session=True)
    try:child.communicate(input=json.dumps(wire).encode(),timeout=600)
    except subprocess.TimeoutExpired:
        capped=True
        def descendants(pid):
            try:children=[int(x) for x in Path(f'/proc/{pid}/task/{pid}/children').read_text().split()]
            except OSError:return []
            return [q for p in children for q in descendants(p)]+children
        owned=descendants(child.pid)
        for pid in owned:
            try:os.kill(pid,signal.SIGTERM)
            except ProcessLookupError:pass
        try:child.wait(timeout=15)
        except subprocess.TimeoutExpired:
            for pid in owned:
                try:os.kill(pid,signal.SIGKILL)
                except ProcessLookupError:pass
            child.kill();child.wait()
receipt={'script_sha256':script_sha256,'target_kind':'bench','load_after':os.getloadavg(),'variant':variant,'source_commit':json.loads(seal.read_text())['source_head'],'load_before':load_before,'phase':phase,'worker_sha256':digest(worker),'request_sha256':digest(phase_dir/'request.json'),'collector_exit_code':child.returncode,'capped':capped,'orchestration_wall_seconds':time.monotonic()-started,'observation_sha256':digest(phase_dir/'observation.json')}
try:
    observed=json.loads((phase_dir/'observation.json').read_text())
    if observed.get('result')=='Observed':
        output=observed['output'];raw_stdout=bytes(output['stdout']);raw_stderr=bytes(output['stderr'])
        (phase_dir/'stdout.bin').write_bytes(raw_stdout);(phase_dir/'stderr.bin').write_bytes(raw_stderr)
        receipt.update(worker_exit_code=output['status'],wall_seconds=seconds(output['wall']),cpu_seconds=seconds(output['cpu']),peak_rss_bytes=output['rss'])
        if output['status']==0:
            report=v.complete_run_ryo_report(output['status'],raw_stdout.decode(),raw_stderr.decode(),files,manifest['selection'],request=request,settings=settings,expected_cache_path=phase_cache)
            write(phase_dir/'report.json',report)
            receipt.update(report_sha256=digest(phase_dir/'report.json'),diagnostics=len(report['diagnostics']),coverage=report['coverage'])
            old=Path('/tmp/django-relation-contract-20261102')/('edit-retained' if phase.startswith('edit-') else 'cold')/'report.json'
            receipt['matches_retained_baseline_report']=json.loads((phase_dir/'report.json').read_text())==json.loads(old.read_text())
            receipt['raw_report_bytes_equal']=digest(phase_dir/'report.json')==digest(old)
except (KeyError,TypeError,ValueError) as exc:receipt['observation_validation_error']=repr(exc)
write(phase_dir/'result.json',receipt)
print(json.dumps(receipt,sort_keys=True),flush=True)
