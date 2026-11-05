import contextlib,hashlib,json,sys,threading
source='import threading\nwith resource(), threading.Lock(): pass\n'
factory=threading.Lock; made=[]; events=[]
def observed_lock(*args,**kwargs):
 lock=factory(*args,**kwargs);made.append(lock);events.append('fresh-lock-created');return lock
@contextlib.contextmanager
def resource():
 events.append('first-resource-entered');yield;events.append('first-resource-exited')
threading.Lock=observed_lock
try: exec(compile(source,'case.py','exec',dont_inherit=True),{'resource':resource})
finally: threading.Lock=factory
assert len(made)==1 and not made[0].locked()
print(json.dumps({'python':sys.version,'source':source,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'events':events,'created_locks':len(made),'lock_released_after_with':not made[0].locked(),'conclusion':'All context items execute from left to right; an unrelated earlier context call does not prevent constructing the subsequent fresh lock.'},indent=2))
