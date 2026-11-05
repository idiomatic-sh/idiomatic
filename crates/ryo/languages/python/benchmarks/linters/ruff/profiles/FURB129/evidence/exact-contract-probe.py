"""Observe only exact bounded authored IO definitions/receivers, never loop consumers."""
import ast,hashlib,inspect,io,json,os,re,sys,tempfile,typing
from pathlib import Path
input_path=Path(__file__).with_name('inputs.json');inputs=json.loads(input_path.read_text());observations={}
class DivergentReader:
    def readlines(self):return ['method-result\n']
    def __iter__(self):return iter(['iterator-result\n'])
allowed_modules={'io','os','builtins','pathlib','typing','codecs'}
for item in inputs:
 source=item['source'];frame=item['framed_source_sha256'];assert hashlib.sha256(b'case.py\0'+source.encode()).hexdigest()==frame
 if frame in observations:observations[frame]['cases'].append(item['case']);continue
 tree=compile(source,'case.py','exec',flags=ast.PyCF_ONLY_AST,dont_inherit=True)
 for node in ast.walk(tree):
  if isinstance(node,ast.Import):assert all(alias.name in allowed_modules for alias in node.names)
  if isinstance(node,ast.ImportFrom):assert node.level==0 and node.module in allowed_modules
  if isinstance(node,(ast.Attribute,ast.Subscript)):assert isinstance(node.ctx,ast.Load)
  if isinstance(node,ast.FunctionDef):
   annotations=[arg.annotation for arg in node.args.posonlyargs+node.args.args+node.args.kwonlyargs]+[node.returns]
   for annotation in annotations:
    if isinstance(annotation,ast.Constant) and isinstance(annotation.value,str):assert re.fullmatch(r'[A-Za-z_][A-Za-z0-9_.]*(?:\[[A-Za-z_][A-Za-z0-9_.]*\])?',annotation.value)
  if isinstance(node,ast.Call):
   if isinstance(node.func,ast.Name):assert node.func.id in {'factory','alias','other','open','o','PurePath','PurePosixPath','PureWindowsPath'},ast.dump(node)
   else:assert isinstance(node.func,ast.Attribute) and node.func.attr in {'StringIO','BytesIO','open','readlines'},ast.dump(node)
   if isinstance(node.func,ast.Name) and node.func.id in {'factory','alias','other'}:assert not node.args and not node.keywords
   if isinstance(node.func,ast.Name) and node.func.id in {'open','o','PurePath','PurePosixPath','PureWindowsPath'}:assert node.args and isinstance(node.args[0],ast.Constant) and node.args[0].value=='x'
   if isinstance(node.func,ast.Attribute) and node.func.attr=='open' and isinstance(node.func.value,ast.Name) and node.func.value.id=='os':assert len(node.args)==2 and isinstance(node.args[0],ast.Constant) and node.args[0].value=='x' and isinstance(node.args[1],ast.Constant) and node.args[1].value==0
 top_loop=next((node for node in tree.body if isinstance(node,ast.For)),None)
 prefix=ast.Module(body=tree.body[:tree.body.index(top_loop)] if top_loop is not None else tree.body,type_ignores=[])
 scenarios=[]
 for cond in [False,True]:
  previous=Path.cwd();resources=[]
  with tempfile.TemporaryDirectory() as directory:
   os.chdir(directory);Path('x').write_text('first\nsecond\n');env={'cond':cond,'unknown':DivergentReader()};record={'cond':cond,'unknown_input':'DivergentReader:readlines and iteration intentionally differ'}
   try:
    exec(compile(prefix,'case.py','exec',dont_inherit=True),env)
    if top_loop is not None:
     iterator=top_loop.iter;assert isinstance(iterator,ast.Call) and isinstance(iterator.func,ast.Attribute) and iterator.func.attr=='readlines';value=eval(compile(ast.Expression(body=iterator.func.value),'receiver.py','eval',dont_inherit=True),env);resources.append(value)
     record.update(receiver_type=type(value).__module__+'.'+type(value).__qualname__,is_io_base=isinstance(value,io.IOBase),is_function_object=inspect.isfunction(value),has_readlines=hasattr(value,'readlines'),has_iter=hasattr(value,'__iter__'))
     if isinstance(value,io.IOBase):
      position=value.tell();via_method=value.readlines();value.seek(position);via_iterator=list(value);record['readlines_matches_iteration']=via_method==via_iterator
     elif isinstance(value,DivergentReader):record.update(via_readlines=value.readlines(),via_iteration=list(value),readlines_matches_iteration=False)
    else:
     function=next(value for key,value in env.items() if key in {'f','use'} and inspect.isfunction(value));annotation=typing.get_type_hints(function).get('fp');origin=typing.get_origin(annotation) or annotation;assert origin is not None
     nominal=origin in {typing.IO,typing.TextIO,typing.BinaryIO} or inspect.isclass(origin) and issubclass(origin,io.IOBase)
     record.update(resolved_annotation=repr(annotation),annotation_origin_module=origin.__module__,annotation_origin_name=origin.__qualname__,nominal_standard_io_annotation=nominal,annotation_has_readlines=hasattr(origin,'readlines'),annotation_has_iter=hasattr(origin,'__iter__'),interpretation='Resolved declared interface only: Python annotations do not enforce runtime argument values.')
   except (AttributeError,NameError) as error:record['error']={'type':type(error).__name__,'message':str(error)}
   finally:
    if 'f' in env and isinstance(env['f'],(int,io.IOBase)):resources.append(env['f'])
    closed=set()
    for value in resources:
     identity=id(value)
     if identity in closed:continue
     closed.add(identity)
     if isinstance(value,int) and not isinstance(value,bool):
      assert value>=3;os.close(value)
     elif isinstance(value,io.IOBase):value.close()
    os.chdir(previous)
   scenarios.append(record)
 observations[frame]={'cases':[item['case']],'source':source,'source_sha256':item['source_sha256'],'framed_source_sha256':frame,'scenarios':scenarios}
print(json.dumps({'version':sys.version,'executable':sys.executable,'interpreter_sha256':hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),'probe_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'inputs_sha256':hashlib.sha256(input_path.read_bytes()).hexdigest(),'typing_source_sha256':hashlib.sha256(Path(typing.__file__).read_bytes()).hexdigest(),'typing_io_doc':inspect.getdoc(typing.IO),'dont_inherit':True,'target_configuration':'python-target=3.10','scope':'35exact authored sources, grouped only by identical framed source hash; imports/definitions/receiver initialization only; no corpus loop consumers executed; conditional unknown receives one explicit nonIO counterexample; interface annotations are static rule policy, not runtime guarantees','rows':list(observations.values())},indent=2))
