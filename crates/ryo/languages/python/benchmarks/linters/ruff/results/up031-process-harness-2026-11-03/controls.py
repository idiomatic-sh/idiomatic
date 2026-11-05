import ast, hashlib, json, sys, tokenize
from pathlib import Path
cases=json.loads(Path(sys.argv[1]).read_text())
rows=[]
for case in cases:
 source=case['files']['case.py']; native=ast.parse(source); compile(source,'case.py','exec')
 annotation=native.body[0].annotation; expression=ast.parse(annotation.value,mode='eval').body
 row={'name':case['name'],'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'compile':'accepted','annotation_host_span':[annotation.col_offset,annotation.end_col_offset],'decoded':annotation.value,'operator':type(expression.op).__name__,'left':expression.left.value}
 if isinstance(expression.op,ast.Mod):
  previous=eval(compile(ast.Expression(expression),'<decoded>','eval'),{'value':'item'})
  replacement=expression.left.value.replace('%s','{}').format('item')
  assert previous==replacement
  row.update({'percent_result':previous,'format_result':replacement,'same_result':True})
 else:
  assert isinstance(expression.op,ast.Add)
  row['percent_rule_scope']=False
 rows.append(row)
print(json.dumps({'python':sys.version,'executable_sha256':hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),'purpose':'CPython establishes source acceptance and decoded expression semantics; diagnostic host mapping is the existing Ryo source-map contract, not compiler diagnostic policy','controls':rows},ensure_ascii=False,indent=2))
