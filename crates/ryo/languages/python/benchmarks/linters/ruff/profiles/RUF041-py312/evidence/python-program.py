import typing,json,sys
results=[]
def check(name,source,assertion):
 ns={'custom':list,'typing':typing};exec(compile(source,name+'.py','exec',dont_inherit=True),ns);exec(assertion,ns);results.append({'name':name,'source':source,'assertion':assertion,'passed':True})
prefix='from __future__ import annotations\nfrom typing import Literal\n'
if sys.version_info>=(3,12):
 check('parameter_identity',prefix+'def f[Literal]()->Literal: ...\n','assert typing.get_type_hints(f)["return"] is f.__type_params__[0]')
 check('variadic_parameter_identity',prefix+'def f[Literal](*x:Literal,**y:Literal): ...\n','assert typing.get_type_hints(f)["x"] is f.__type_params__[0] and typing.get_type_hints(f)["y"] is f.__type_params__[0]')
 for name,annotation in [('shadowed_nested','Literal[Literal[1]]'),('quoted_shadowed_nested','"Literal[Literal[1]]"')]:
  check(name,prefix+'def f[Literal]()->'+annotation+': ...\n',"try:\n typing.get_type_hints(f)\nexcept TypeError:\n pass\nelse:\n raise AssertionError('type parameter was incorrectly interpreted as typing.Literal')")
 check('unrelated_parameter_positive',prefix+'def f[T]()->Literal[Literal[1]]: ...\n','assert typing.get_type_hints(f)["return"] == typing.Literal[1]')
 check('quoted_unrelated_parameter_positive',prefix+'def f[T]()->"Literal[Literal[1]]": ...\n','assert typing.get_type_hints(f)["return"] == typing.Literal[1]')
source='from __future__ import annotations\nLiteral=custom\ndef outer():\n from typing import Literal\n def f()->Literal[Literal[1]]: ...\n return f,locals()\n'
check('lexical_vs_default_runtime_scope',source,'f,localns=outer(); default=typing.get_type_hints(f)["return"]; explicit=typing.get_type_hints(f,localns=localns)["return"]; assert default == list[list[1]]; assert explicit == typing.Literal[1]')
json.dump({'python':sys.version,'results':results},sys.stdout,indent=2)
