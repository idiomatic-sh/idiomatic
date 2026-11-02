"""Execute exact fixture with explicitly modeled supported patch-version values.
These are modeled values, not claims about the interpreter's actual version.
"""
import json,sys
from pathlib import Path
inputs=json.loads(Path(sys.argv[1]).read_text())
observed=[]
actual=list(sys.version_info)
for version in inputs['modeled_versions']:
 for name,source in inputs['sources'].items():
  original=sys.version_info
  try:
   sys.version_info=tuple(version)
   namespace={}
   exec(compile(source,name,'exec',dont_inherit=True),namespace)
   observed.append({'modeled_version':version,'source':name,'branch':namespace['branch']})
  finally:sys.version_info=original
print(json.dumps({'actual_interpreter':actual,'modeled_not_actual_versions':True,'observed':observed},indent=2))
