import json,sys
samples={"module_simple":"x: tick()", "parenthesized_name":"(x): tick()", "module_attribute":"obj.x: tick()", "module_subscript":"obj[0]: tick()", "class_simple":"class C:\n x: tick()", "class_attribute":"class C:\n obj.x: tick()", "function_local":"def f():\n x: tick()\nf()", "function_attribute":"def f():\n obj.x: tick()\nf()", "function_subscript":"def f():\n obj[0]: tick()\nf()", "function_parenthesized":"def f():\n (x): tick()\nf()", "signature":"def f(x: tick())->tick(): pass", "defaults":"def f(x=tick()): pass", "decorator":"@tick()\ndef f(): pass"}
rows=[]
for future in [False,True]:
 for name,body in samples.items():
  events=[]
  def tick(): events.append("tick");return lambda f:f
  class Obj:
   def __setitem__(self,key,value):pass
  source=("from __future__ import annotations\n" if future else "")+body+"\n"
  env={"tick":tick,"obj":Obj()}
  try: exec(compile(source,"case.py","exec",dont_inherit=True),env);error=None
  except Exception as e:error={"type":type(e).__name__,"message":str(e)}
  rows.append({"name":name,"future":future,"source":source,"events_at_definition":events,"error":error})
print(json.dumps({"version":sys.version,"cases":rows}))
