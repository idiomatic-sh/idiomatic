import json,sys,ast,dis,hashlib,builtins
from pathlib import Path
root=Path('/tmp/next-five-ruf-oracles-sol');refs={r['name']:r for r in (json.loads(s) for s in (root/'references.jsonl').read_text().splitlines())}
sources={'ruf046-fresh-outer-late-rebind':'x=int((int:=5))\n','ruf046-slow-outer-late-rebind':'import builtins\nx=int((int:=5))\n','ruf057-annotation-return':'def n()->int: ...\nx=round(n)\n','function-call-result-own-control':'def n()->int: return 1\nx=round(n())\n'}
rows=[]
for name,source in sources.items():
 code=compile(source,'case.py','exec',dont_inherit=True);namespace={};row={'name':name,'file':'case.py','source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'bytecode':[{'op':x.opname,'arg':x.argrepr} for x in dis.get_instructions(code)]}
 try:exec(code,namespace);row.update(completed=True,x_type=type(namespace.get('x')).__name__,x_repr=repr(namespace.get('x')),int_binding_repr=repr(namespace.get('int',builtins.int)))
 except Exception as e:row.update(completed=False,error_type=type(e).__name__,error_message=str(e),n_is_function=isinstance(namespace.get('n'),type(lambda:None)),n_is_int=isinstance(namespace.get('n'),int))
 if name in refs:row['pinned_ruff_record']=refs[name]
 else:row['pinned_ruff_record']=None
 tree=ast.parse(source);call=next(x for x in ast.walk(tree) if isinstance(x,ast.Call) and isinstance(x.func,ast.Name) and x.func.id in ['int','round']);lines=source.splitlines(keepends=True);start=len(''.join(lines[:call.lineno-1]).encode())+call.col_offset;end=len(''.join(lines[:call.end_lineno-1]).encode())+call.end_col_offset
 rule='RUF046' if name.startswith('ruf046') else 'RUF057';row['intended_scoped_diagnostics']=[] if name=='ruf057-annotation-return' else [{'rule':rule,'file':'case.py','start':start,'end':end}];rows.append(row)
print(json.dumps({'status':'independent-rule-evidence-input-not-a-discrepancy-approval','python':sys.version,'executable':sys.executable,'compile_dont_inherit':True,'scope':'each named rule plus invalid-syntax; original full413 Ruff records retained','cases':rows},indent=2))
