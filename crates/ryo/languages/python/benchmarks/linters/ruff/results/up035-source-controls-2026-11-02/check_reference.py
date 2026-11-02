from pathlib import Path
import tomllib,tempfile,subprocess,json,collections,hashlib
import sys
root=Path(__file__).resolve().parents[2];ruff=Path(sys.argv[1]);records=[]
for target in ['py37','py38','py39','py310','py311','py312','py313','py314']:
 p=root/'rules/UP035/cases.toml' if target=='py310' else root/f'profiles/UP035-{target}/inputs/rules/UP035/cases.toml'
 for c in tomllib.loads(p.read_text())['cases']:
  with tempfile.TemporaryDirectory() as d:
   for name,text in c['files'].items():(Path(d)/name).write_text(text)
   proc=subprocess.run([str(ruff),'check','--isolated','--select','UP035','--target-version',target,'--output-format','json',*map(lambda n:str(Path(d)/n),c['files'])],capture_output=True,text=True)
   assert proc.returncode in (0,1),proc.stderr
   raw=json.loads(proc.stdout);findings=[]
   for f in raw:
    filename=Path(f['filename']).name;text=c['files'][filename];lines=text.splitlines(keepends=True)
    def at(x):return len(''.join(lines[:x['row']-1]).encode())+len(lines[x['row']-1][:x['column']-1].encode())
    findings.append(dict(rule=f['code'],file=filename,start=at(f['location']),end=at(f['end_location'])))
   norm=lambda fs:collections.Counter(tuple(sorted(f.items())) for f in fs)
   assert norm(findings)==norm(c['expected']),(target,c['name'],findings,c['expected'])
   records.append(dict(target=target,case=c['name'],source_sha256={n:hashlib.sha256(t.encode()).hexdigest() for n,t in c['files'].items()},findings=findings))
out=root/'results/up035-source-controls-2026-11-02';out.mkdir(parents=True,exist_ok=True)
(out/'pinned-reference.json').write_text(json.dumps(dict(ruff_sha256=hashlib.sha256(ruff.read_bytes()).hexdigest(),upstream_commit='9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a',checks=records),indent=2)+'\n')
print(len(records),'full-output pinned reference case checks exact')
