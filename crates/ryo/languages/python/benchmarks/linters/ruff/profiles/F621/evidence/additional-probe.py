from pathlib import Path
import hashlib,json,subprocess,sys,tempfile
root=Path(__file__).resolve().parent
ruff=Path('/home/vscode/.local/bin/ruff')
observations=[]
for case in json.loads((root/'cases.json').read_text()):
    filename,source=next(iter(case['files'].items()))
    try:
        compile(source,filename,'exec')
        python={'accepted':True}
    except SyntaxError as error:
        python={'accepted':False,'type':type(error).__name__,'message':error.msg,'line':error.lineno,'offset':error.offset,'end_line':error.end_lineno,'end_offset':error.end_offset}
    with tempfile.TemporaryDirectory() as temporary:
        work=Path(temporary)
        (work/filename).write_text(source)
        command=[str(ruff),'check','--isolated','--select','F621','--target-version','py310','--output-format','json','--',filename]
        result=subprocess.run(command,cwd=work,text=True,capture_output=True)
        observations.append({'name':case['name'],'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'python':python,'reference':{'command':command,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr,'findings':json.loads(result.stdout)}})
report={'python':sys.version,'executable':sys.executable,'ruff_sha256':hashlib.sha256(ruff.read_bytes()).hexdigest(),'observations':observations}
(root/'results.json').write_text(json.dumps(report,indent=2)+'\n')
for observation in observations:
    print(observation['name'],observation['python'],observation['reference']['findings'])
