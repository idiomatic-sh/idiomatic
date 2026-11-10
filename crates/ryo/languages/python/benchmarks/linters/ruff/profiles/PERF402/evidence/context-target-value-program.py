import sys,json,typing,builtins,asyncio
results=[]
for x in json.load(sys.stdin):
 name=x['name'];source=next(iter(x['files'].values()));generic='[dict]' in source
 if generic and sys.version_info<(3,12):continue
 ns={'key':'x','custom':int};error=None
 try:exec(compile(source,name+'.py','exec',dont_inherit=True),ns)
 except Exception as e:error=type(e).__name__
 if 'function-definition' in name or 'function-type-parameter-return' in name or 'future-function-generic-return' in name:
  assert error=='TypeError';assert callable(ns['d']) and not isinstance(ns['d'],builtins.dict)
 elif name=='ruf051-with-target':assert error==('TypeError' if sys.version_info>=(3,11) else 'AttributeError') and 'd' not in ns
 elif name=='ruf051-async-with-target':
  try:asyncio.run(ns['f']())
  except Exception as e:assert type(e).__name__==("TypeError" if sys.version_info>=(3,11) else "AttributeError")
  else:raise AssertionError('dict is not an async context manager')
 elif name=='ruf051-builtin-variable-name-shadow':assert error is None and isinstance(ns['dict'],builtins.dict)
 elif name=='ruf051-constructor-before-current-rebind':
  assert error is None and type(ns['d']) is builtins.dict;ns['d']['x']=1;ns['f']();assert ns['d']=={}
 elif name=='ruf051-future-type-parameter':
  assert error is None;h=typing.get_type_hints(ns['f']);assert h['d'] is ns['f'].__type_params__[0] and h['d'] is not builtins.dict
 else:
  assert error is None and typing.get_type_hints(ns['f'])['d'] is builtins.dict;arg={'x':1};ns['f'](arg,object()) if name=='ruf051-future-parameter-shadow' else ns['f'](arg);assert arg=={}
 results.append({'name':name,'source':source,'passed':True,'observed_error':error,'namespace_model':'key=x and custom=int supplied explicitly; source unchanged. Function calls with built-in dict are separate live controls; type annotation is static evidence, not runtime argument enforcement.'})
class Manager:
 def __enter__(self):return ['entered']
 def __exit__(self,*args):pass
with Manager() as target:assert target==['entered'] and not isinstance(target,Manager)
results.append({'name':'context_manager_result_is_not_manager','passed':True,'source':'class Manager: __enter__ returns list, __exit__ does nothing; with Manager() as target yields list'})
class SelfManager:
 def __enter__(self):return self
 def __exit__(self,*args):pass
manager=SelfManager()
with manager as target:assert target is manager
results.append({'name':'context_manager_self_return','passed':True})
json.dump({'python':sys.version,'results':results},sys.stdout,indent=2)
