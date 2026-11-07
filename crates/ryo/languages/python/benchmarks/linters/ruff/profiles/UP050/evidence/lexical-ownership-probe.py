from pathlib import Path
import json,sys,hashlib
root=Path(__file__).parent/'repository/crates/ryo/languages/python/benchmarks/linters/ruff';rows=[]
for code,cases in json.loads((Path(__file__).parent/'probe-inputs.json').read_text()).items():
 for case in cases:
  if not case['name'].startswith('independent-callable-lexical-ownership-'):continue
  src=case['files']['case.py'];ns={};exec(compile(src,'case.py','exec',dont_inherit=True),ns)
  try:ns['f']();result={'status':'returned'}
  except Exception as e:result={'status':'raised','type':type(e).__name__,'message':str(e)}
  negative=case['name'].endswith('-0');assert result=={'status':'returned'} if not negative else result.get('type')=='UnboundLocalError'
  rows.append({'rule':code,'name':case['name'],'source':src,'source_sha256':hashlib.sha256(src.encode()).hexdigest(),'result':result})
print(json.dumps({'version':sys.version,'executable':sys.executable,'interpreter_sha256':hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),'probe_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'dont_inherit':True,'scope':'Exact controlled definitions and function calls only; each negative has compiler-local ownership despite a prior outer alias/builtin, each positive imports locally before use.','cases':rows},indent=2))
