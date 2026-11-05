import sys,json,__future__
out=[]
for r in json.load(sys.stdin):
 d=dict(r)
 try:
  c=compile(r['source'],'case.py','exec',dont_inherit=True);d.update(accepted=True,annotations_future=bool(c.co_flags & __future__.annotations.compiler_flag))
 except SyntaxError as e:d.update(accepted=False,message=e.msg,line=e.lineno,offset=e.offset,end_line=e.end_lineno,end_offset=e.end_offset)
 out.append(d)
json.dump(dict(version=sys.version,results=out),sys.stdout,indent=2)
