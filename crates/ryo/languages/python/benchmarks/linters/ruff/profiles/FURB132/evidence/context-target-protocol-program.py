import sys,json,asyncio,io
results=[]
for row in json.load(sys.stdin):
 ns={'key':'x','item':'x','x':'x','xs':[]};source=row['source'];error=None
 try:
  exec(compile(source,row['rule']+'-'+row['name']+'.py','exec',dont_inherit=True),ns)
  if row['name']=='ruf051-async-with-target':asyncio.run(ns['f']())
 except Exception as e:error=type(e).__name__
 expected='TypeError' if sys.version_info>=(3,11) else 'AttributeError'
 assert error==expected,(row,error)
 results.append({**row,'error':error,'expected':expected,'passed':True,'namespace_model':'key=item=x="x",xs=[] supplied; original source unchanged; async function invoked separately after declaration.'})
for cls in (dict,set,list):
 assert not hasattr(cls,'__enter__') and not hasattr(cls,'__aenter__')
 results.append({'name':'builtin_manager_protocol_absent','type':cls.__name__,'passed':True})
stream=io.StringIO('a')
with stream as target:assert target is stream
assert stream.closed
results.append({'name':'resolved_standard_iobase_returns_self','passed':True})
json.dump({'python':sys.version,'results':results},sys.stdout,indent=2)
