from pathlib import Path
import ast,json
p=Path(__file__).resolve().parent;rows=json.loads((p/'current-failing-cases.json').read_text());ev=p;by={json.dumps(r['files'],sort_keys=True):r for r in json.loads((ev/'derived-validity-spans-new-cause.json').read_text())['cases']};out=[]
extra={'future-scope-class-forward-type':'Later','future-scope-function-forward-type':'Later','future-scope-except-restoration':'Inside','future-scope-if-else-inheritance':'Later','future-scope-if-restoration':'Inside','future-scope-try-handler-inheritance':'Later'}
for r in rows:
 key=json.dumps(r['files'],sort_keys=True);o=by.get(key);intended=[x for x in r['expected'] if x['rule']!='invalid-syntax'];basis=[]
 if o is not None:
  intended += [{k:v for k,v in x.items() if k!='validity'} for x in o['derived']];basis.append('Exact source-identical prior independent token/AST validity oracle; retains all prior selected-rule lint findings unless a separately justified binding correction is stated.')
 elif r['name'] in extra:
  name=extra[r['name']];file,source=next(iter(r['files'].items()));tree=ast.parse(source);nodes=[n for n in ast.walk(tree) if isinstance(n,ast.Name) and isinstance(n.ctx,ast.Load) and n.id==name];assert len(nodes)==1
  n=nodes[0];lines=source.encode().splitlines(keepends=True);start=sum(map(len,lines[:n.lineno-1]))+n.col_offset;end=sum(map(len,lines[:n.end_lineno-1]))+n.end_col_offset
  finding=dict(rule='F821',file=file,start=start,end=end)
  if finding not in intended:intended.append(finding)
  basis.append('Independent AST name-token span for '+name+'. Existing target3.10 original/valid-pair and both unbound/bound runtime controls show this eager annotation lookup; nested future cannot enable postponement. Original other F821 findings retained.')
 else:
  assert r['name']=='stub-named-expression-preserves-undefined-read';basis.append('Grammar-valid stub expression is distinct from portable typing-expression validity. Existing F821 unknown b read retained unchanged; no generic grammar finding inferred solely from typing rejection.')
 if r['rule']=='PLW0602' and r['name']=='future-scope-nested-future-blocks-type-scope-walrus':
  intended=[x for x in intended if x['rule']!='PLW0602'];basis.append('Specific surviving-binding analysis correction: valid eager outer function annotation binds value; subsequent nonlocal replaces Global under established PLW query policy. Preserved target control removes conflicting global and invalid nested directive and executes outer() is object. Compiler conflict remains generic diagnostic; invalidity alone does not justify this lint omission.')
 out.append(dict(rule=r['rule'],name=r['name'],files=r['files'],reference_expected=r['expected'],independent_intended=intended,basis=basis))
print(json.dumps(out,indent=2))
