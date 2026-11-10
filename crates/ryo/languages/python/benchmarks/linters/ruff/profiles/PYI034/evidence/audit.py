from pathlib import Path
import ast,__future__,json,sys,hashlib
inputs=json.loads(Path(sys.argv[1]).read_text());results=[]
for item in inputs:
 rows=[]
 for filename,source in item['files'].items():
  row={'file':filename,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'source_kind':'stub' if filename.endswith('.pyi') else 'python','scope':'CPython compiler syntax/flags only; a .pyi filename does not establish static typing-checker semantics'}
  for phase in ['ast_parse','compile']:
   try:
    result=ast.parse(source,filename=filename,feature_version=(3,10)) if phase=='ast_parse' else compile(source,filename,'exec',dont_inherit=True)
    row[phase]={'accepted':True}
    if phase=='compile':row[phase]['future_annotations']=bool(result.co_flags & __future__.annotations.compiler_flag)
   except SyntaxError as e:row[phase]={'accepted':False,'message':e.msg,'line':e.lineno,'offset':e.offset,'end_line':e.end_lineno,'end_offset':e.end_offset}
  rows.append(row)
 results.append({'files':item['files'],'consumers':item['consumers'],'compiler':rows})
print(json.dumps({'python':sys.version,'results':results},indent=2))
