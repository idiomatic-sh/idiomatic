import sys,json,types
cases={"module_simple":("x: tick()","module"),"module_parenthesized":("(x): tick()","module"),"module_attribute":("obj.x: tick()","module"),"class_simple":("class C:\n x: tick()","class"),"class_attribute":("class C:\n obj.x: tick()","class"),"local_annotation":("def f():\n x: tick()\nf()","function"),"signature":("def f(x: tick())->tick():pass","function"),"lambda_body":("def f(x:(lambda: tick())):pass","function"),"lambda_default":("def f(x:(lambda y=tick():y)):pass","function"),"generator_body":("def f(x:(tick() for x in items)):pass","function"),"generator_first_iterable":("def f(x:(x for x in iterable())):pass","function"),"generator_later_iterable":("def f(x:(x for x in items for y in iterable())):pass","function")}
rows=[]
for future in [False,True]:
 for name,(body,owner) in cases.items():
  events=[]
  def tick():events.append("tick");return int
  def iterable():events.append("iterable");return [1]
  module=types.ModuleType("contract_case");module.__dict__.update(tick=tick,iterable=iterable,items=[1],obj=types.SimpleNamespace())
  source=("from __future__ import annotations\n"if future else "")+body+"\n"
  try:
   exec(compile(source,"case.py","exec",dont_inherit=True),module.__dict__)
   before=list(events);target=module if owner=="module"else module.C if owner=="class"else module.f
   annotations=target.__annotations__;after=list(events)
   error=None
  except Exception as e:before=list(events);after=list(events);annotations={};error={"type":type(e).__name__,"message":str(e)}
  rows.append({"name":name,"future":future,"source":source,"definition_events":before,"after_annotations_access":after,"annotation_keys":sorted(annotations),"error":error})
print(json.dumps({"version":sys.version,"cases":rows}))
