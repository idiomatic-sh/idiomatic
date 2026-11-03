import sys,json,types
out=[]
for item in json.load(sys.stdin):
 external=types.ModuleType('external');external.Foo=type('ExternalFoo',(),{});sys.modules['external']=external
 ns={};name=item['name'];source=next(iter(item['files'].values()))
 try:
  exec(compile(source,name+'.py','exec',dont_inherit=True),ns)
  if name in ('runtime-fallback','shadowed-true'):assert ns['x'] is ns['Foo']
  if name=='local-shadow':assert ns['f']()==1
  if name=='quoted-annotation':assert ns['__annotations__']['x']=='Foo'
  error=None
 except Exception as e:error=type(e).__name__
 expected='NameError' if name=='eager-annotation' else None
 out.append({'name':name,'error':error,'expected':expected,'passed':error==expected,'source':source,'external_import_model':'Explicit sys.modules external with one real Foo class; source bytes unchanged. Fallback uses preexisting local Foo; true guard really imports external.Foo.'})
json.dump({'python':sys.version,'results':out},sys.stdout,indent=2)
