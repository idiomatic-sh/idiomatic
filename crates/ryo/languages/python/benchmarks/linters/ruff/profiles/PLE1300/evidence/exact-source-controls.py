"""Execute unchanged named source files; no AST rewriting or fixture filtering."""
import json,hashlib,sys
from pathlib import Path
controls=json.loads(Path(sys.argv[1]).read_text());results=[]
for control in controls:
    for filename,source in sorted(control['files'].items()):
        error=None;message=None
        try: exec(compile(source,filename,'exec'),{})
        except Exception as refusal: error=type(refusal).__name__;message=str(refusal)
        results.append(dict(name=control['name'],file=filename,source_sha256=hashlib.sha256(source.encode()).hexdigest(),error=error,message=message,expected_error=control['expected_error'],matches=error==control['expected_error']))
print(json.dumps(dict(version=sys.version,executable=sys.executable,results=results),indent=2,ensure_ascii=False))
if not all(row['matches'] for row in results):sys.exit(1)
