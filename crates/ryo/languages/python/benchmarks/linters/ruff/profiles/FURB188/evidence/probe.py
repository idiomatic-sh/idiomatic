from pathlib import Path
import json,subprocess,hashlib,tempfile,sys
root=Path(__file__).resolve().parent;ruff=Path('/home/vscode/.local/bin/ruff');records=[]
def offset(source,row,column):
 lines=source.splitlines(keepends=True)
 return sum(len(line.encode())for line in lines[:row-1])+len(lines[row-1][:column-1].encode())
for case in json.loads((root/'cases.json').read_text()):
 with tempfile.TemporaryDirectory()as temporary:
  directory=Path(temporary)
  for file,source in case['files'].items():
   path=directory/file;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(source)
  command=[str(ruff),'check','--isolated','--select','FURB188','--target-version','py310','--output-format','json','--',*sorted(case['files'])]
  result=subprocess.run(command,cwd=directory,text=True,capture_output=True);findings=json.loads(result.stdout);normalized=[]
  for finding in findings:
   file=str(Path(finding['filename']).relative_to(directory));source=case['files'][file]
   normalized.append({'rule':finding['code'],'file':file,'start':offset(source,finding['location']['row'],finding['location']['column']),'end':offset(source,finding['end_location']['row'],finding['end_location']['column'])})
  normalized.sort(key=lambda x:(x['file'],x['start'],x['end'],x['rule']));expected=sorted(case['expected'],key=lambda x:(x['file'],x['start'],x['end'],x['rule']))
  records.append({'name':case['name'],'files':case['files'],'expected':expected,'normalized':normalized,'matched':normalized==expected,'exit_code':result.returncode,'command':command,'stdout':result.stdout,'stderr':result.stderr})
empty_probes=[]
for name,source in [('literal-zero','if text.endswith(""):\n    text = text[:-0]\n'),('literal-len','if text.endswith(""):\n    text = text[:-len("")]\n')]:
 with tempfile.TemporaryDirectory()as temporary:
  directory=Path(temporary);(directory/'case.py').write_text(source);command=[str(ruff),'check','--isolated','--select','FURB188','--target-version','py310','--output-format','json','--','case.py'];result=subprocess.run(command,cwd=directory,text=True,capture_output=True)
  empty_probes.append({'name':name,'source':source,'command':command,'stdout':result.stdout,'stderr':result.stderr,'exit_code':result.returncode})
report={'python':sys.version,'executable':sys.executable,'ruff_sha256':hashlib.sha256(ruff.read_bytes()).hexdigest(),'cases':records,'empty_suffix_probes':empty_probes}
(root/'results.json').write_text(json.dumps(report,indent=2)+'\n');print('reference',sum(x['matched']for x in records),'/',len(records));print('mismatches',[x['name']for x in records if not x['matched']]);print('empty-suffix finding counts',[(x['name'],len(json.loads(x['stdout'])))for x in empty_probes])
