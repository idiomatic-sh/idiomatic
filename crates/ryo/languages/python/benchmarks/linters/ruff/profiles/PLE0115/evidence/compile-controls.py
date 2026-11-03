import ast,json,sys
from pathlib import Path
inputs=json.loads(Path(sys.argv[1]).read_text());out=[]
for item in inputs:
 for name,source in item['files'].items():
  row={'expected_compile':item['expected_compile'],'rule':item['rule'],'case':item['name'],'file':name,'source':source,'ast_parse':None,'compile':None}
  for mode in ['ast_parse','compile']:
   try:
    if mode=='ast_parse':ast.parse(source,filename=name,feature_version=(3,10))
    else:compile(source,name,'exec')
    row[mode]={'accepted':True}
   except SyntaxError as e:row[mode]={'accepted':False,'message':e.msg,'line':e.lineno,'offset':e.offset,'end_line':e.end_lineno,'end_offset':e.end_offset,'text':e.text}
  assert row['ast_parse']['accepted'], row
  assert row['compile']['accepted'] == row['expected_compile'], row
  out.append(row)
print(json.dumps({'python':sys.version,'results':out},indent=2))
