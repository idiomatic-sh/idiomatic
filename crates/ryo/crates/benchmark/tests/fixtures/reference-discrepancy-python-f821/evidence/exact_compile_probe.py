import sys,json
cases=json.load(sys.stdin);rows=[]
for case in cases:
 for filename,source in case['files'].items():
  row={'name':case['name'],'filename':filename,'source':source}
  try: compile(source,filename,'exec',dont_inherit=True);row['accepted']=True
  except SyntaxError as e: row.update(accepted=False,error={'message':e.msg,'line':e.lineno,'column':e.offset,'end_line':e.end_lineno,'end_column':e.end_offset})
  rows.append(row)
json.dump({'python':sys.version,'executable':sys.executable,'dont_inherit':True,'cases':rows},sys.stdout,ensure_ascii=False)
