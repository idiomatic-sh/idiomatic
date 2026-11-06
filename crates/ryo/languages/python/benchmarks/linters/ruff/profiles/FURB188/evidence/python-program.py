from pathlib import Path
import json,sys
observations=[]
for suffix in ['', '.txt']:
 text='data.txt';original=text[:-len(suffix)]if text.endswith(suffix)else text;replacement=text.removesuffix(suffix)
 observations.append({'text':text,'suffix':suffix,'original':original,'replacement':replacement,'equal':original==replacement})
prefix='';text='data.txt';original=text[len(prefix):]if text.startswith(prefix)else text
report={'python':sys.version,'executable':sys.executable,'suffix_controls':observations,'empty_prefix':{'original':original,'replacement':text.removeprefix(prefix),'equal':original==text.removeprefix(prefix)}}
Path(__file__).with_name('python-results.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
