"""Offline pinned-reference controls; does not execute or validate RyoQL."""
from pathlib import Path
import collections,json,subprocess,sys,tempfile,tomllib
root=Path(__file__).resolve().parents[2]
ruff=Path(sys.argv[1]).resolve()
checked=0
for target in ['py37','py310','py313']:
    path=root/'rules/UP036/cases.toml' if target=='py310' else root/f'profiles/UP036-{target}/inputs/rules/UP036/cases.toml'
    for case in tomllib.loads(path.read_text())['cases']:
        with tempfile.TemporaryDirectory() as directory:
            for name,text in case['files'].items():
                (Path(directory)/name).write_text(text)
            command=[str(ruff),'check','--isolated','--select','UP036','--target-version',target,'--output-format','json',*sorted(case['files'])]
            run=subprocess.run(command,cwd=directory,capture_output=True,text=True)
            assert run.returncode in (0,1),run.stderr
            findings=[]
            for finding in json.loads(run.stdout):
                name=Path(finding['filename']).name
                lines=case['files'][name].splitlines(keepends=True)
                def offset(where):
                    return len(''.join(lines[:where['row']-1]).encode())+len(lines[where['row']-1][:where['column']-1].encode())
                findings.append(dict(rule=finding['code'],file=name,start=offset(finding['location']),end=offset(finding['end_location'])))
            expected=case.get('reference-discrepancy',{}).get('expected',case['expected'])
            normalize=lambda rows:collections.Counter(tuple(sorted(row.items())) for row in rows)
            assert normalize(findings)==normalize(expected),(target,case['name'],findings,expected)
            checked+=1
print(checked,'full-output pinned-reference comparisons exact')
