import ast,json,tomllib,collections
from pathlib import Path
r=Path(__file__).resolve().parents[2]
all_diffs=[];checks=0
for target in ['py37','py310','py313']:
 p=r/'rules/UP036/cases.toml' if target=='py310' else r/f'profiles/UP036-{target}/inputs/rules/UP036/cases.toml'
 for case in tomllib.loads(p.read_text())['cases']:
  reports=[]
  for filename,text in case['files'].items():
   tree=ast.parse(text);imports={};shadows=set()
   for n in ast.walk(tree):
    if isinstance(n,ast.Import):
     for a in n.names:imports[a.asname or a.name.split('.')[0]]=a.name
    if isinstance(n,ast.ImportFrom):
     for a in n.names:imports[a.asname or a.name]=(n.module or '')+'.'+a.name
    if isinstance(n,ast.Assign):
     for v in n.targets:
      if isinstance(v,ast.Name):shadows.add(v.id)
   def path(n,params):
    if isinstance(n,ast.Name):return imports.get(n.id) if n.id not in shadows|params else None
    if isinstance(n,ast.Attribute):
     base=path(n.value,params);return base+'.'+n.attr if base else None
   def lhs(n,params):
    p=path(n,params)
    if p=='sys.version_info':return 'tuple'
    if p=='sys.version_info.major':return 'major'
    if isinstance(n,ast.Subscript) and path(n.value,params)=='sys.version_info':
     if isinstance(n.slice,ast.Constant) and type(n.slice.value)is int and n.slice.value==0:return 'major'
     if isinstance(n.slice,ast.Slice) and n.slice.lower is None and n.slice.step is None and isinstance(n.slice.upper,ast.Constant) and n.slice.upper.value in [2,3]:return 'tuple'
   def visit(node,params=set()):
    if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)):
     params=params|{a.arg for a in [*node.args.posonlyargs,*node.args.args,*node.args.kwonlyargs]}
    if isinstance(node,ast.If) and isinstance(node.test,ast.Compare):
     c=node.test
     if len(c.ops)==1:
      kind=lhs(c.left,params);v=c.comparators[0];op=type(c.ops[0]);where=None
      if kind and isinstance(v,ast.Constant) and type(v.value)is int:
       if v.value>255:where=v
       elif v.value in [2,3] and ((kind=='major' and op in [ast.Lt,ast.LtE,ast.Gt,ast.GtE,ast.Eq,ast.NotEq]) or op in [ast.Eq,ast.NotEq]):where=c
      if kind=='tuple' and isinstance(v,ast.Tuple) and 1<=len(v.elts)<=3 and all(isinstance(x,ast.Constant)and type(x.value)is int for x in v.elts) and op in [ast.Lt,ast.LtE,ast.Gt,ast.GtE]:
       vals=[x.value for x in v.elts]
       if vals[0]>255:where=v
       elif vals[0]<3:where=c
       elif vals[0]==3:
        if len(vals)==1:where=c
        elif vals[1]>255 or len(vals)>2 and vals[2]>255:where=v
        elif vals[1]<int(target[3:]):where=c
        elif op in [ast.Lt,ast.GtE] and vals[1]==int(target[3:]) and (len(vals)<3 or vals[2]==0):where=c
      if where is not None:
       lines=text.splitlines(keepends=True)
       start=len(''.join(lines[:where.lineno-1]).encode())+where.col_offset
       end=len(''.join(lines[:where.end_lineno-1]).encode())+where.end_col_offset
       # Pinned controls explicitly exercise normal one-line/file suppression.
       if 'noqa: UP036' not in lines[where.lineno-1] and '# ruff: noqa: UP036' not in text:
        reports.append({'rule':'UP036','file':filename,'start':start,'end':end})
    for ch in ast.iter_child_nodes(node):visit(ch,params)
   visit(tree)
  norm=lambda fs:collections.Counter(tuple(sorted(f.items()))for f in fs)
  if norm(reports)!=norm(case['expected']):all_diffs.append({'target':target,'case':case['name'],'intended':reports,'reference':case['expected']})
  checks+=1
print(checks,'AST policy comparisons;',len(all_diffs),'explicit differences')
print([(x['target'],x['case'])for x in all_diffs])
assert not all_diffs, all_diffs
