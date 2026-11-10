import sys,json
results=[]
for row in json.load(sys.stdin):
 exec(compile(row['source'],row['rule']+'-'+row['name']+'.py','exec',dont_inherit=True),{})
 results.append({'rule':row['rule'],'name':row['name'],'passed':True})
json.dump({'python':sys.version,'results':results},sys.stdout,indent=2)
