import sys,json,hashlib
cases={
 'lambda-in-generator':'def f():\n    return ((lambda: await g()) for x in xs)\n',
 'lambda-in-async-function':'async def f():\n    return lambda: await g()\n',
 'lambda-in-sync-function':'def f():\n    return lambda: await g()\n',
 'class-base-yield-from-async':'async def f():\n    class C((yield from xs)): pass\n',
 'class-base-yield-from-sync':'def f():\n    class C((yield from xs)): pass\n',
 'class-base-await-async':'async def f():\n    class C(await g()): pass\n',
 'quoted-await-annotation':'def f(x: "await g()"): pass\n',
}
rows=[]
for name,source in cases.items():
 try:
  compile(source,'case.py','exec',dont_inherit=True);outcome={'accepted':True}
 except SyntaxError as error:
  outcome={'accepted':False,'type':'SyntaxError','message':error.msg,'lineno':error.lineno,'offset':error.offset,'end_lineno':getattr(error,'end_lineno',None),'end_offset':getattr(error,'end_offset',None)}
 rows.append({'name':name,'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'compile_mode':'exec','dont_inherit':True,'outcome':outcome})
print(json.dumps({'implementation':sys.implementation.name,'version':sys.version,'executable':sys.executable,'rows':rows},indent=2))
