import json,sys
cases={
'plain-assignment':'x:(value:=int)\n',
'future-assignment':'from __future__ import annotations\nx:(value:=int)\n',
'plain-signature':'def f(x:(value:=int)): pass\n',
'future-signature':'from __future__ import annotations\ndef f(x:(value:=int)): pass\n',
'future-docstring':'"module docstring"\nfrom __future__ import annotations\ndef f(x:Missing): pass\n',
'relative':'from .__future__ import annotations\ndef f(x:(value:=int)): pass\n',
'late':'x=1\nfrom __future__ import annotations\ndef f(x:(value:=int)): pass\n',
'nested':'def outer():\n from __future__ import annotations\n def f(x:(value:=int)): pass\n',
'unknown-feature':'from __future__ import annotations, not_a_feature\ndef f(x:Missing): pass\n',
'malformed':'from __future__ import (\n',
'lazy':'lazy from __future__ import annotations\ndef f(x:(value:=int)): pass\n',
'generic-signature':'def f[T](x:(value:=int)): pass\n',
'generic-class':'class C[T]((value:=int)): pass\n',
'generic-bound':'def f[T:(value:=int)](): pass\n',
'alias-value':'type Alias=(value:=int)\n',
'generic-lambda-body':'type Alias=(lambda:(value:=int))\n',
'generic-lambda-default':'type Alias=(lambda x=(value:=int): x)\n',
'future-lambda-body':'from __future__ import annotations\ndef f(x:(lambda:(value:=int))): pass\n',
'future-lambda-default':'from __future__ import annotations\ndef f(x:(lambda y=(value:=int):y)): pass\n',
}
rows=[]
for name,source in cases.items():
 for filename in ['case.py','case.pyi']:
  row={'name':name,'source':source,'filename':filename}
  try:compile(source,filename,'exec',dont_inherit=True);row['accepted']=True
  except SyntaxError as error:row.update(accepted=False,error=dict(message=error.msg,line=error.lineno,column=error.offset,end_line=error.end_lineno,end_column=error.end_offset))
  rows.append(row)
print(json.dumps({'python':sys.version,'executable':sys.executable,'cases':rows},indent=2))
