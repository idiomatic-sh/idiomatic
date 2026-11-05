from pathlib import Path
import sys,json,tomllib,hashlib
r=Path(__file__).resolve().parents[2];sys.path.insert(0,str(r));from authoring import profile_config,profile_proofs,rule_registration,profile_sources
manifest=json.loads(profile_config.MANIFEST.read_text());native=profile_proofs.native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text());perf={'node_result_reuse':'disabled','column_kernel':False,'diagnostics':{key:False for key in ['execution','lifecycle_memory','resource_routes','row_progress','steps','joins','prefixes','subplans','selection_detail']}}
def digest(value):return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
rows=[]
for cases_path in sorted((r/'rules').glob('*/cases.toml')):
 code=cases_path.parent.name
 if code=='I001':continue
 cases=tomllib.loads(cases_path.read_text())['cases'];certs=[c for c in cases if c.get('reference-discrepancy')or c.get('tool-contract-difference')]
 if not certs:continue
 source,target,scope=rule_registration.full_inventory_settings(code,manifest,native);profile=tomllib.loads(source.read_text());parser=profile.get('parser-configuration',profile_sources.parser_configuration(target)if target else 'python-target=3.10');version=next(k for k,v in profile_config.PARSER_CONFIGURATIONS.items()if v==parser)
 for case in certs:
  identity=case.get('reference-discrepancy',case.get('tool-contract-difference'))['identity'];selected=sorted(case.get('selected-rules',[code]));files=case.get('requested-files',sorted(case['files']));args=['check','--isolated','--output-format','json','--cache-dir','$STATE/ruff','--select',','.join(selected),'--target-version',version,'--',*files];settings={'arguments':args,'environment':{'RAYON_NUM_THREADS':'1'},'directory':'$ROOT','parser_configuration':parser,'project_context_mode':{'isolated':'Isolated','listed-disk':'ListedDisk'}[profile.get('project-context-mode','isolated')]}
  if profile.get('source-language')is not None:settings['source_language']=profile['source-language']
  for key in ['selected-rules','project-search-roots','given-project-inventory']:
   value=case.get(key,profile.get(key)if key=='project-search-roots'else None)
   if value is not None:settings[key.replace('-','_')]=sorted(value)if key=='selected-rules'else value
  declarations=case.get('rule-observations',profile_sources.projected_profile_observations(profile,selected))
  if declarations:settings['rule_observations']=declarations
  old=digest(settings);current={**settings,'performance':perf};new=digest(current);claimed=identity['settings-sha256'];state='proven-old-shape'if claimed==old else'current-shape'if claimed==new else'unresolved'
  rows.append({'rule':code,'case':case['name'],'source_profile':str(source.relative_to(profile_config.REPOSITORY)),'declared':claimed,'old_computed':old,'new_computed':new,'state':state,'normalized_before':settings,'normalized_after':current})
out=Path(__file__).with_name('recomputed.json');out.write_text(json.dumps(rows,indent=2,ensure_ascii=False)+'\n');from collections import Counter;print(Counter(x['state']for x in rows));print([(x['rule'],x['case'])for x in rows if x['state']=='unresolved'][:30])
