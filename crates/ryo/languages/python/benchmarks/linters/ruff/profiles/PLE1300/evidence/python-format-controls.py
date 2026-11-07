import json,sys
class Custom:
    def __format__(self,spec): return 'accepted:' + spec
controls=[('unicode-width',lambda:format(1,'٣d')),('ascii-width',lambda:format(1,'3d')),('unicode-precision',lambda:format(1.2,'.٢f')),('ascii-precision',lambda:format(1.2,'.2f')),('custom-q',lambda:'{:q}'.format(Custom())),('builtin-q',lambda:'{:q}'.format(1)),('converted-custom-q',lambda:'{!s:q}'.format(Custom())),('zero-coercion',lambda:format(-0.0,'z.2f'))]
rows=[]
for name,control in controls:
    try: rows.append(dict(name=name,value=control(),error=None))
    except Exception as error: rows.append(dict(name=name,value=None,error=type(error).__name__,message=str(error)))
print(json.dumps(dict(version=sys.version,rows=rows),ensure_ascii=False))
