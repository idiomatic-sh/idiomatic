from pathlib import Path
import json,tomllib,hashlib,collections,datetime
root=Path('/tmp/django-all413-progressive-20261104');profile=Path('/tmp/ryo-audit-docs-publish-20261103/crates/ryo/languages/python/benchmarks/linters/ruff/results/ci-5331-full-inventory-2026-11-03/summary.json');inventory=[r['rule_id']for r in json.loads(profile.read_text())['rules']];assert len(inventory)==len(set(inventory))==413
raw=(json.dumps(inventory,separators=(',',':'))+'\n').encode();(root/'ordered-inventory.json').write_bytes(raw);digest=hashlib.sha256(raw).hexdigest()
manifest=json.loads((root/'manifest.json').read_text());manifest.update(ordered_inventory=inventory,ordered_inventory_sha256=digest,inventory_source=str(profile),inventory_source_sha256=hashlib.sha256(profile.read_bytes()).hexdigest(),authoritative_reporting_state=str(root/'inventory-state.json'));(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
internal=json.loads((root/'summary.json').read_text());rows={r:{'rule':r,'status':'PENDING_MEASUREMENT','worker_sha256':manifest['worker_sha256']}for r in inventory};rows.update(internal['rules']);assert set(rows)==set(inventory)
extra=Path('/tmp/django-current411-extra2-20261104/summary.json')
cohorts={'db1-409':{'source':manifest['source'],'worker_sha256':manifest['worker_sha256'],'compatible_rules':409}}
for row in rows.values():row['timing_cohort']='db1-409'
if extra.exists():
 current=json.loads(extra.read_text());cohorts['current411-extra2']={'source':current['source'],'worker_sha256':current['worker_sha256'],'compatible_rules':2,'profile_public_source':current['profile_public_source'],'manifest':str(extra.parent/'manifest.json')}
 for rule,row in current['rules'].items():
  assert rule in ['E902','RUF200'];rows[rule]={**row,'source':current['source'],'timing_cohort':'current411-extra2','different_source_not_pooled':True}
counts=dict(collections.Counter(r['status']for r in rows.values()));assert sum(counts.values())==413
out={**manifest,'updated_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'rules':rows,'counts':counts,'internal_resume_state':str(root/'summary.json'),'reporting_only':True,'running_worker_unmodified':True,'source_cohorts':cohorts,'timing_cohorts_must_not_be_pooled':True};(root/'inventory-state.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'counts':counts,'rows':len(rows),'inventory_sha256':digest,'path':str(root/'inventory-state.json')}))
