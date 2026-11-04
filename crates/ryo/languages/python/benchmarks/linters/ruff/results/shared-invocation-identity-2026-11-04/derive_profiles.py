from pathlib import Path
import sys,json,tomllib,hashlib,re,subprocess
r=Path(__file__).resolve().parents[2];sys.path.insert(0,str(r));from authoring import profile_config,profile_proofs,rule_registration,profile_sources
manifest=json.loads(profile_config.MANIFEST.read_text());native=profile_proofs.native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text());phases=json.loads(profile_config.RULE_PHASES_FILE.read_text());base=tomllib.loads((r/'benchmark.toml').read_text());repo=profile_config.REPOSITORY
oldhead='d55db018';oldbase=tomllib.loads(subprocess.check_output(['git','show',oldhead+':'+str((r/'benchmark.toml').relative_to(repo))],cwd=repo,text=True))
def oldbytes(path):return subprocess.check_output(['git','show',oldhead+':'+str(path.relative_to(repo))],cwd=repo)
def closure(query, registry, read):
 found={};pending=[query]
 while pending:
  source=pending.pop()
  for name in re.findall(r'^\s*import\s+\w+\s+from\s+"([^"]+)"\s*;',read(source).decode(),re.M):
   if name in found:continue
   found[name]=registry[name];pending.append(r/registry[name])
 return found
rows=[]
for p in sorted((r/'rules').glob('*/cases.toml')):
 code=p.parent.name
 if code=='I001':continue
 cases=tomllib.loads(p.read_text())['cases'];certs=[c for c in cases if c.get('reference-discrepancy')or c.get('tool-contract-difference')]
 if not certs:continue
 source,target,scope=rule_registration.full_inventory_settings(code,manifest,native);data=tomllib.loads(source.read_text());olddata=tomllib.loads(oldbytes(source).decode());parser=data.get('parser-configuration',profile_sources.parser_configuration(target)if target else'python-target=3.10');app={code:profile_sources.rule_phase(phases,code)['applicability']}
 def projected(data, registry):
  out={key:value for key,value in data.items()if key not in {'deferred','pending','query-modules','rule-applicability'}};out['defaults']=[code];out.pop('supporting-rules',None);out['parser-configuration']=parser
  if registry:out['query-modules']=registry
  out['rule-applicability']=app
  return ''.join(f'{k} = {profile_sources.toml_value(v)}\n'for k,v in out.items())
 oldregistry={**oldbase.get('query-modules',{}),**olddata.get('query-modules',{})};oldclosure=closure(p.parent/'query.ryoql',oldregistry,oldbytes);oldstage=projected(olddata,{**olddata.get('query-modules',{}),**oldclosure})
 currentregistry={**base.get('query-modules',{}),**data.get('query-modules',{})};currentclosure=closure(p.parent/'query.ryoql',currentregistry,lambda p:p.read_bytes());currentstage=projected(data,{**data.get('query-modules',{}),**currentclosure})
 variants={'source-raw':hashlib.sha256(source.read_bytes()).hexdigest(),'historical-source-raw':hashlib.sha256(oldbytes(source)).hexdigest(),'historical-normal-stage':hashlib.sha256(oldstage.encode()).hexdigest()};pins={c.get('reference-discrepancy',c.get('tool-contract-difference'))['identity']['profile-sha256']for c in certs};matches={pin:[name for name,sha in variants.items()if sha==pin]for pin in pins};rows.append({'rule':code,'source_profile':str(source.relative_to(repo)),'old_matches':matches,'new_sha256':hashlib.sha256(currentstage.encode()).hexdigest(),'new_profile':currentstage,'variants':variants,'status':'proven'if all(matches.values())else'unresolved'})
Path(__file__).with_name('profile-recomputed.json').write_text(json.dumps(rows,indent=2)+'\n');from collections import Counter;print(Counter(x['status']for x in rows));print([(x['rule'],x['old_matches'])for x in rows if x['status']=='unresolved'])
