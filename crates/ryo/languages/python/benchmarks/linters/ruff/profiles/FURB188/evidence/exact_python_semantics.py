from pathlib import Path
import json,sys,hashlib
root=Path(__file__).resolve().parent;data=json.loads((root/'results.json').read_text());rows=[]
controls=[(r['name'],r['source'],json.loads(r['stdout'])[0])for r in data['empty_suffix_probes']]
for name in ['suffix-statement','empty-prefix']:
 record=next(r for r in data['cases']if r['name']==name);controls.append((name,record['files']['case.py'],json.loads(record['stdout'])[0]))
for name,source,finding in controls:
 replacement=finding['fix']['edits'][0]['content'];before={'text':'data.txt'};after={'text':'data.txt'}
 exec(compile(source,'case.py','exec'),before);exec(compile(replacement,'case.py','exec'),after)
 rows.append({'name':name,'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'replacement':replacement,'applicability':finding['fix']['applicability'],'initial_text':'data.txt','original_result':before['text'],'replacement_result':after['text'],'equal':before['text']==after['text']})
report={'python':sys.version,'executable':sys.executable,'observations':rows};(root/'exact-python-results.json').write_text(json.dumps(report,indent=2)+'\n');print([(r['name'],r['equal'])for r in rows])
