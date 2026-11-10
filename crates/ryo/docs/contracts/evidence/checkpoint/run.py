# ignore: idiomatic.arch/file-length - the checkpoint evidence directory holds one validation script, run.py, beside its fixture cases in run_test.py; the census and the checkpoint diff checks are modes of it over the same validation.json, commits and raw logs, and a second script would be a file role the directory does not have.
import json,os,re,subprocess,sys,tempfile,time,fcntl,tomllib
from datetime import date
from functools import lru_cache
from pathlib import Path
from typing import NamedTuple
try:
 import tree_sitter,tree_sitter_rust
 os.environ.pop('RYO_RUN_REEXEC',None)
except (ImportError,OSError):
 # Rust is read through tree-sitter-rust, pinned with its runtime in tools/install.py's pytest environment: a script started by another
 # Python (run.py, or run_test.py importing it) starts again there, once, and only when both import there; otherwise it says what to run.
 LAUNCH=['uv','tool','run','--offline','--from','pytest','python','-I']; script=Path(sys.argv[0])
 try: ready=not os.environ.get('RYO_RUN_REEXEC') and script.suffix=='.py' and script.is_file() and subprocess.run(LAUNCH+['-c','import tree_sitter, tree_sitter_rust'],capture_output=True).returncode==0
 except OSError: ready=False
 if not ready: sys.exit('run tools/install.py first')
 os.environ['RYO_RUN_REEXEC']='1'
 os.execvp(LAUNCH[0],LAUNCH+[str(script.resolve()),*sys.argv[1:]])
p=Path(__file__).resolve().parent
contracts=p.parents[1]
readme=contracts/'README.md'
validation_json=p/'validation.json'
tests_json=p/'tests.json'
commands_json=p/'commands.json'
TM_TARGETS=('tm_source','tm_cli_source')
def raw_dir(v):
 """The validation's raw directory: `raw_identity` names it under `{temp}`, the system temporary directory of the machine reading it,
 so the record carries no machine path."""
 return Path(str(v.get('raw_identity') or '/nonexistent').replace('{temp}',tempfile.gettempdir()))
def raw_identity(out_dir):
 """`out_dir` as the record names it: under `{temp}` when it is in the system temporary directory, else as given."""
 temp=Path(tempfile.gettempdir()).resolve(); out=Path(out_dir).resolve()
 return '{temp}/'+out.relative_to(temp).as_posix() if out.is_relative_to(temp) else str(out_dir)
# The record's blocks that hold one item per command or per proof: each item is one line of validation.json, so the record grows by a
# line per command and per red_green entry and stays a file of one screenful per block.
ONE_LINE_ITEMS=('checks','red_green')
def dumps(v,pad=''):
 """JSON with two-space indent where a list of scalars or a short dict of scalars (four keys, 300 characters) stays on one line, so a command line or a red-then-green entry is one line; at the top level, each item of a block named in ONE_LINE_ITEMS is one line whatever its length."""
 if pad=='' and isinstance(v,dict) and any(k in ONE_LINE_ITEMS for k in v):
  def block(k,x):
   if k in ONE_LINE_ITEMS and isinstance(x,list): return '[\n'+',\n'.join('    '+json.dumps(i) for i in x)+'\n  ]'
   if k in ONE_LINE_ITEMS and isinstance(x,dict): return '{\n'+',\n'.join('    '+json.dumps(n)+': '+json.dumps(i) for n,i in x.items())+'\n  }'
   return dumps(x,'  ')
  return '{\n'+',\n'.join('  '+json.dumps(k)+': '+block(k,x) for k,x in v.items())+'\n}'
 if isinstance(v,list) and all(not isinstance(x,(list,dict)) for x in v): return json.dumps(v)
 if isinstance(v,list): return '[\n'+',\n'.join(pad+'  '+dumps(x,pad+'  ') for x in v)+'\n'+pad+']'
 if isinstance(v,dict) and len(v)<=4 and all(not isinstance(x,(list,dict)) for x in v.values()) and len(json.dumps(v))<=300: return json.dumps(v)
 if isinstance(v,dict): return '{\n'+',\n'.join(pad+'  '+json.dumps(k)+': '+dumps(x,pad+'  ') for k,x in v.items())+'\n'+pad+'}'
 return json.dumps(v)
def tm_target(command,log):
 """Finding count, production/test split and findings per scoped directory of one `tm check` log."""
 report=json.loads(Path(log).read_text())
 root=command[command.index('--root')+1]
 skip=set(); words=[]
 for i,w in enumerate(command[command.index('check')+1:]):
  if w in ('--format',): skip.add(i+1)
  if not w.startswith('--') and i not in skip: words.append(w)
 findings=[f['path'] for f in report['findings']]
 directories={d:sum(1 for f in findings if f.startswith(root+'/'+d.rstrip('/')+'/') or f==root+'/'+d) for d in words}
 test=sum(1 for f in findings if is_test_file(f))
 return {'findings':len(findings),'suppressed':len(report.get('suppressed',[])),'production':len(findings)-test,'test':test,'scoped_directories':directories}
TM_VALIDATE='tm_validate_all'
def tm_validate_counts(log):
 """The versions `tm validate` proves and fails per workspace, from one log in which a `workspace <name>` line precedes each workspace's JSON document; a workspace whose run printed no document is absent."""
 counts={}; where=None
 for line in Path(log).read_text(errors='replace').splitlines():
  if line.startswith('workspace '): where=line.split()[1]
  elif where and line.startswith('{') and '"proven"' in line: d=json.loads(line); counts[where]={'passed':d['proven'],'failed':len(d['failed'])}
 return counts
def record(v,commands,out_dir):
 """The checks block (each command's exit, source and time) from the run's raw `<name>.json`, the tm target splits, the `tm validate` counts and the change-amplification census."""
 v['checks']={n:json.loads((out_dir/(n+'.json')).read_text()) for n,c in commands if (out_dir/(n+'.json')).exists()}
 v['tm_targets']={n:tm_target(c,out_dir/(n+'.log')) for n,c in commands if n in TM_TARGETS and (out_dir/(n+'.log')).exists()}
 if (out_dir/(TM_VALIDATE+'.log')).exists(): v['results'][TM_VALIDATE]=tm_validate_counts(out_dir/(TM_VALIDATE+'.log'))
 if (out_dir/'census.log').exists(): v['census']=json.loads((out_dir/'census.log').read_text())
def scope_delta(v):
 """The scoped directories that are new since the validation.json at HEAD, each with its finding count."""
 r=subprocess.run(['git','-C',str(p),'show','HEAD:./validation.json'],capture_output=True,text=True)
 old=json.loads(r.stdout).get('tm_targets',{}) if r.returncode==0 else {}
 lines=[]
 for n,t in v.get('tm_targets',{}).items():
  before=old.get(n,{}).get('scoped_directories',{})
  for d,c in t['scoped_directories'].items():
   if d not in before: lines.append('newly scoped for '+n+': '+d+' ('+str(c)+' findings)')
  for d in before:
   if d not in t['scoped_directories']: lines.append('no longer scoped for '+n+': '+d)
 return lines or ['scoped directories unchanged since HEAD']
def fmt(x):
 if isinstance(x,dict) and 'passed' in x: return str(x['passed'])+' PASS / '+str(x['failed'])+' failed'+(' / '+str(x['not_run'])+' not run' if 'not_run' in x else '')+(' ('+x['note']+')' if 'note' in x else '')
 if isinstance(x,dict): return ', '.join(k.replace('_',' ')+' '+fmt(y) for k,y in x.items())
 if isinstance(x,list): return str(len(x))+': '+'; '.join('`'+str(y)+'`' for y in x) if x else '0'
 return str(x)
def render(v):
 rows=[('Source','`'+v['source'][:10]+'`'),('Validation','validation'+str(v['validation'])+' (raw logs in the directory named `'+raw_dir(v).name+'`, kept outside the repository)')]
 rows+=[(k.replace('_',' '),fmt(x)) for k,x in v['results'].items()]
 for n,t in v.get('tm_targets',{}).items():
  rows.append((n+' findings',str(t['findings'])+' ('+str(t['production'])+' production, '+str(t['test'])+' test), '+str(t['suppressed'])+' suppressed, '+str(len(t['scoped_directories']))+' scoped directories'))
 w=v['tm_whole_tree']
 rows.append(('Whole-tree TM',str(w['total'])+' findings, '+str(w['suppressed'])+' suppressed ('+str(w['at_checkpoint_318'])+' at 318)'))
 g=v['ownership_graph']
 rows.append(('File-level ownership cycles',str(g['other_cycles'])+' ('+fmt(g['targeted_cycles'])+' targeted)'))
 c=v.get('census')
 if c: rows.append(('Change amplification','replace the evaluator '+str(c['replace_the_evaluator']['total_files'])+' files ('+str(c['replace_the_evaluator']['production']['files'])+' production in '+str(c['replace_the_evaluator']['production']['crates'])+' crates, '+str(c['replace_the_evaluator']['test']['files'])+' test); Operation exhaustive matches '+str(c['operation_exhaustive_matches']['files'])+' files in '+str(c['operation_exhaustive_matches']['crates'])+' crates; per-file fact '+str(c['per_file_fact_touch_points']['files'])+' files in '+str(c['per_file_fact_touch_points']['crates'])+' crates; cross-crate tests naming runtime internals '+str(c['cross_crate_tests_naming_runtime_internals']['files'])))
 rows+=[('Red test',x) for x in red_rows(v)]
 rows+=[('Not run test',x) for x in not_run_rows(v)]
 table=['| Check | Result |','|---|---|']+['| '+k+' | '+fmt(x).replace('|','\\|')+' |' for k,x in rows]
 head='### Current ownership checkpoint — source `'+v['source'][:10]+'`, validation'+str(v['validation'])
 return '\n'.join([head,'','Written by `evidence/checkpoint/run.py` from [validation.json](evidence/checkpoint/validation.json); do not edit by hand. Review: [checkpoint](audits/2026-11-06-derived/ownership/checkpoint.json).','']+table)+'\n\n'
TICKED=re.compile(r'^(- \[x\]|\|[^|]*\|\s*\[x\]|\|\s*☑\s*\||\|.*\|\s*☑\s*\|)',re.I)
CHECKED_CELL=re.compile(r'(?<=\|)(\s*)☑(?=\s*\|)')
PENDING=re.compile(r'pending (?:independent )?(?:re-?)?review|PENDING-REVIEW|review[^.|]{0,80} has not run',re.I)
CITED=re.compile(r'(?:validation|proof|review|reviewed|pass|runtime|reuse|reused)\s?(\d{3})\b|\b(?:in|at)(\d{3})\b',re.I)
def ticked_rows_without_proof(text,produced):
 """Ticked rows citing no validation, or one newer than the validation produced."""
 bad=[]
 for n,line in enumerate(text.splitlines(),1):
  if not TICKED.match(line): continue
  cited=[int(a or b) for a,b in CITED.findall(line)]
  if not cited: bad.append(str(n)+': cites no validation: '+line[:110])
  elif max(cited)>produced: bad.append(str(n)+': cites validation'+str(max(cited))+' beyond '+str(produced)+': '+line[:110])
 return bad
RAN=re.compile(r'^\s+(?:Running (.+?) \((?:[^)]*/)?(\w+?)-[0-9a-f]+\)|Doc-tests (\w+))')
TEST=re.compile(r'^test (.+?) \.\.\. (ok|FAILED|ignored)')
FAILURE=re.compile(r'^---- (.+?) stdout ----$')
RED_ROW=re.compile(r'^(.+?): (?:expected |a \w+ row expects the wrong kind|the subject panicked)')
NOT_AVAILABLE=re.compile(r'^not available\b')
SUMMARY=re.compile(r'^test result: ')
# Cargo's line after a test binary that failed, crashed or was killed; a binary it follows with no `test result:` line, or with no
# failed test, did not finish, and is recorded red under CRASHED, which no row owns.
CARGO_FAILED=re.compile(r'^error: test failed, to rerun pass ')
CRASHED='(the binary did not finish: no `test result:` line, or cargo failed it with no failed test)'
# cargo-semver-checks' comparison line and each lint it fails: the comparison is the `semver-checks` binary, each failed lint a failed test.
SEMVER_RAN=re.compile(r'^\s+Checking \S+ v\S+ -> v\S+ ')
SEMVER_FAILURE=re.compile(r'^--- failure (\w+): ')
def test_results(log):
 """Per test binary of one cargo test log: its passed, ignored and not-run tests, and each failed test with the case rows its runner names; `collected` when the runner ran every row and named every red one. A binary cargo failed that printed no `test result:` line, or no failed test, in its own run crashed: its failure is CRASHED. Binaries of one name in several packages (each crate's `allocation` or `bdd`) share a key, and each run is judged on its own, so a crash after a sibling's red is never absorbed by it. A test that prints before its status has the status on a line of its own; one that prints `not available` there measured nothing, so its `ok` is recorded as not run, never as passed."""
 out={}; binary=None; failing=None; pending=[]; summarized=False; failed_here=0
 for line in Path(log).read_text(errors='replace').splitlines():
  if binary and SUMMARY.match(line): summarized=True
  if binary and CARGO_FAILED.match(line):
   if not summarized or not failed_here: out[binary]['failed'].setdefault(CRASHED,{'rows':[],'collected':False})
   continue
  if SEMVER_RAN.match(line): binary='semver-checks'; out.setdefault(binary,{'passed':[],'ignored':[],'not_run':[],'failed':{}}); failing=None; summarized=False; failed_here=0; continue
  m=SEMVER_FAILURE.match(line)
  if m and binary=='semver-checks': out[binary]['failed'].setdefault(m.group(1),{'rows':[],'collected':False}); continue
  m=RAN.match(line)
  if m: binary=m.group(2) or m.group(3); out.setdefault(binary,{'passed':[],'ignored':[],'not_run':[],'failed':{}}); failing=None; summarized=False; failed_here=0; continue
  m=TEST.match(line) or (pending and re.match(r'^()(ok|FAILED|ignored)$',line))
  if not m and binary and line.startswith('test ') and ' ... ' in line: pending.append((line[5:line.index(' ... ')],bool(NOT_AVAILABLE.match(line[line.index(' ... ')+5:])))); continue
  if m and binary:
   name,status=m.groups(); unavailable=False
   if not name: name,unavailable=pending.pop(0)
   if status=='FAILED': out[binary]['failed'].setdefault(name,{'rows':[],'collected':False}); failed_here+=1
   else: out[binary]['not_run' if unavailable and status=='ok' else 'passed' if status=='ok' else 'ignored'].append(name)
   continue
  m=FAILURE.match(line)
  if m and binary: failing=out[binary]['failed'].setdefault(m.group(1),{'rows':[],'collected':False}); continue
  if failing is None: continue
  if re.search(r'\d+ row\(s\) of \w+ failed:',line): failing['collected']=True
  m=RED_ROW.match(line.strip())
  if m and m.group(1) not in failing['rows']: failing['rows'].append(m.group(1))
 return {b:t for b,t in out.items() if t['passed'] or t['failed'] or t['ignored'] or t['not_run']}
def stored(tests):
 """The recorded tests as tests.json keeps them, one line per list: a failed test is `[collected or aborted, red row...]`, an empty list is left out."""
 return {n:{b:{k:x for k,x in (('passed',t['passed']),('ignored',t['ignored']),('not_run',t['not_run']),('failed',{name:['collected' if f['collected'] else 'aborted']+f['rows'] for name,f in t['failed'].items()})) if x} for b,t in bs.items()} for n,bs in tests.items()}
def loaded(kept):
 """tests.json read back into the shape `test_results` gives."""
 return {n:{b:{'passed':t.get('passed',[]),'ignored':t.get('ignored',[]),'not_run':t.get('not_run',[]),'failed':{name:{'collected':f[0]=='collected','rows':f[1:]} for name,f in t.get('failed',{}).items()}} for b,t in bs.items()} for n,bs in kept.items()}
def record_tests(v,commands,out_dir):
 """Every cargo test target's tests, from its raw log, into tests.json beside validation.json (which stays within its line budget)."""
 v['tests']={n:test_results(out_dir/(n+'.log')) for n,c in commands if (c[:2]==['cargo','test'] or n in SEMVER_COMMANDS) and (out_dir/(n+'.log')).exists()}
 tests_json.write_text(dumps(stored(v['tests']))+'\n')
def red_rows(v):
 """Each failed test with the case rows it reports: every red row when its runner collected them, else the first only."""
 lines=[]
 for target,binaries in v.get('tests',{}).items():
  for binary,t in binaries.items():
   for name,f in t['failed'].items():
    if f['collected']: said='red rows '+str(len(f['rows']))+': '+', '.join(f['rows'])
    elif f['rows']: said='red at row '+f['rows'][0]+'; the rows after it did not run'
    else: said='red; its failure names no case row'
    lines.append(target+' '+name+': '+said)
 return lines
def not_run_rows(v):
 """Each test that printed `not available` and so measured nothing, though its runner said ok."""
 return [target+' '+name+': it printed `not available` and measured nothing' for target,binaries in v.get('tests',{}).items() for t in binaries.values() for name in t.get('not_run',[])]
SENTENCE=re.compile(r'^[a-z][a-z0-9]*(?:_[a-z0-9]+){3,}\*?$')
SEGMENT=re.compile(r'^[a-z_*][a-z0-9_*]*$')
TESTISH=re.compile(r'^(?:tests?|\w+_test|controls)$')
def expand(span):
 """`a::{b,c}::d` as `a::b::d` and `a::c::d`; one level of braces, any number of groups."""
 m=re.search(r'\{([^{}]*)\}',span)
 if not m: return [span]
 return [x for part in m.group(1).split(',') for x in expand(span[:m.start()]+part.strip()+span[m.end():])]
def cited_tests(line):
 """The tests and case rows a row cites: a backticked lowercase `::` path through a test module or ending in a sentence-like name, or a bare sentence-like name."""
 cited=[]
 for span in re.findall(r'`([^`]+)`',line):
  for c in expand(span):
   segs=c.split('::')
   if not all(SEGMENT.match(s) for s in segs): continue
   if len(segs)==1 and SENTENCE.match(c): cited.append(c)
   elif len(segs)>1 and (any(TESTISH.match(s) for s in segs) or SENTENCE.match(segs[-1])): cited.append(c)
 return cited
def all_tests(v):
 """(binary, test, red rows or None when it passed, collected) over every recorded target; a test red in one target is red, and one not run in a target is not run unless red."""
 merged={}
 for binaries in v.get('tests',{}).values():
  for binary,t in binaries.items():
   for name in t['passed']: merged.setdefault((binary,name),None)
   for name in t.get('not_run',[]):
    if merged.get((binary,name)) is None: merged[(binary,name)]={'rows':[],'collected':False,'not_run':True}
   for name,f in t['failed'].items(): merged[(binary,name)]=f
 return merged
def glob_rx(segs): return '::'.join('.+' if s=='*' else re.escape(s).replace(r'\*','[^:]*') for s in segs)
def row_verdict(row,tests):
 """A case row's verdict from the tests that run its table: passed when they passed or collected without naming it."""
 if not tests: return 'absent'
 if any(f and row in f['rows'] for _,f in tests): return 'red'
 if all(f is None or f['collected'] for _,f in tests): return 'passed'
 return 'not run'
def binaries_of(crate):
 """The test binaries of a crate directory: its library's unit tests and each integration test, as cargo names them."""
 manifest=tomllib.loads((crate/'Cargo.toml').read_text())
 names={manifest.get('lib',{}).get('name',manifest['package']['name'].replace('-','_'))}
 names|={t['name'] for t in manifest.get('test',[])}
 names|={x.stem for x in (crate/'tests').glob('*.rs')} if manifest.get('package',{}).get('autotests',True) else set()
 return names
def sources():
 """Every `CaseName("...")` in the product's sources: its crate's test binaries and its file's module path, read once."""
 if hasattr(sources,'found'): return sources.found
 found={}; product=p.parents[3]
 for path in product.rglob('*.rs'):
  rel=path.relative_to(product).parts
  if 'target' in rel or 'fixtures' in rel and 'tests' in rel[:rel.index('fixtures')]: continue
  try: names=re.findall(r'CaseName\("([^"]+)"\)',path.read_text())
  except (UnicodeDecodeError,OSError): continue
  if not names: continue
  crate=next(d for d in path.parents if (d/'Cargo.toml').is_file())
  inner=path.relative_to(crate).with_suffix('').parts
  module=[x for x in inner[1:] if x not in ('mod','lib','main')]
  for name in names: found.setdefault(name,[]).append((binaries_of(crate),module))
 sources.found=found; return found
def verdict(cited,tests):
 """passed, red, not run (its table stopped at an earlier red row) or absent (no recorded test runs it). A cited path is a test, the tests of a module, or a case row of a module's table."""
 segs=cited.split('::'); rx=re.compile('(?:^|::)'+glob_rx(segs)+'$')
 hit=[(k,f) for k,f in tests.items() if rx.search(k[1])]
 if not hit and '*' not in segs[-1]:
  under=re.compile('(?:^|::)'+glob_rx(segs)+'::')
  hit=[(k,f) for k,f in tests.items() if under.search(k[1])]
 if hit: return 'passed' if all(f is None for _,f in hit) else 'not run' if all(f is None or f.get('not_run') for _,f in hit) else 'red'
 if '*' in segs[-1]: return 'absent'
 if len(segs)>1:
  if segs[-1] not in sources(): return 'absent'
  under=re.compile('(?:^|::)'+glob_rx(segs[:-1])+'::')
  return row_verdict(segs[-1],[(k,f) for k,f in tests.items() if under.search(k[1])])
 for binaries,module in sources().get(cited,[]):
  scoped=[(k,f) for k,f in tests.items() if k[0] in binaries]
  for prefix in range(len(module),0,-1):
   for start in range(prefix):
    under=re.compile('(?:^|::)'+'::'.join(map(re.escape,module[start:prefix]))+'::')
    found=[(k,f) for k,f in scoped if under.search(k[1])]
    if found: return row_verdict(cited,found)
 if any(f and cited in f['rows'] for f in tests.values()): return 'red'
 return 'absent'
def untick(line):
 """The row with its `[x]` box, or every `☑` cell of a table row, unchecked."""
 if re.search(r'\[x\]',line,re.I) and not CHECKED_CELL.search(line): return re.sub(r'\[x\]','[ ]',line,count=1,flags=re.I)
 return CHECKED_CELL.sub(lambda m: m.group(1)+'☐',line)
def untick_unproven(text,v):
 """The text with every ticked row (a `[x]` box or a `☑` cell, in the README, matrix.md or checklist.md) un-ticked that cites no test, says its own review is pending, or cites a test or case row red, not run or absent in the validation's recorded tests, and one line per such row."""
 tests=all_tests(v); bad=[]; lines=text.splitlines(keepends=True)
 for n,line in enumerate(lines):
  if not TICKED.match(line): continue
  cited=cited_tests(line); found={c:verdict(c,tests) for c in cited}
  unproven={c:x for c,x in found.items() if x!='passed'}
  pending=PENDING.search(line)
  if cited and not unproven and not pending: continue
  said='cites no test' if not cited else 'says its review is pending (`'+pending.group(0)+'`)' if not unproven else str(len(cited)-len(unproven))+' of '+str(len(cited))+' cited tests passed; '+'; '.join(x+' `'+c+'`' for c,x in unproven.items())
  bad.append(str(n+1)+': '+said+': '+line[:80])
  lines[n]=untick(line)
 return ''.join(lines),bad
RERUN=re.compile(r'\b(?:narrow(?:ed)? runs?|re-?runs?)\b',re.I)
def reruns_without_log(texts,v):
 """A result cited from a rerun names its raw log, and that log is in the validation directory."""
 bad=[]
 for where,text in texts:
  if not RERUN.search(text): continue
  logs=re.findall(r'[\w.-]+\.log\b',text)
  if not logs or not all((raw_dir(v)/x).is_file() for x in logs): bad.append(where+': cites a rerun with no raw log in '+str(raw_dir(v))+': '+text[:110])
 return bad
RUST_REGRESSIONS='Measure full-Django cold/warm/edit and Rust regressions; meet final objective.'
CLOSE='Close full TM, contracts, independent review and normal CI.'
SDK_RELEASE='The next SDK release is breaking (semver 0.x major, i.e. 0.2.0), owner-started via release-plz.'
# The commands whose log carries cargo-semver-checks' report, read for its failed lints as `test_results` reads a test binary.
SEMVER_COMMANDS={'sdk_api'}
# One entry per result or replay test that is red now, chosen when it turned red; `own()` refuses an entry whose result or
# test is green (a stale owner would pre-own a red that has not happened yet).
OWNERS={
 'benchmark_library':CLOSE,
 'core_allocation':RUST_REGRESSIONS,
 'cli_cost':'Acceptance: 413/413 on the current engine, full Django (2,875 files, all 413 rules) with unknown 0 and unread 0, Rust regression guards within bounds.',
 'behavior::ruff_findings_equal_the_expected_findings':CLOSE,
 'behavior::every_recorded_rule_gives_the_recorded_rows':CLOSE,
 'peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound':'Decode lazy packed frontiers once per population, not once per root, and compile ordered existence without the eager request-level pass.',
 'tastemaker_workspace':CLOSE,
}
# The exit's owners: each red test a command recorded, by its exact (command, test binary, test) id, and the open README row that
# owns it. A red no entry names, and a red command that recorded no failed test, are unowned and fail the run.
RED_OWNERS_ALL_FEATURES={
 ('libraries','ryo_benchmark','inbound::use_cases::compare_corpus::use_case_test::refuses'):CLOSE,
 ('libraries','ryo_benchmark','inbound::use_cases::compare_corpus::use_case_test::succeeds'):CLOSE,
 ('libraries','ryo_benchmark','inbound::use_cases::verify_cases::use_case_test::process_capture::test::succeeds'):CLOSE,
 ('cli_tests','replay','behavior::ruff_findings_equal_the_expected_findings'):CLOSE,
 ('cli_tests','replay','behavior::every_recorded_rule_gives_the_recorded_rows'):CLOSE,
 ('cli_tests','replay','peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound'):OWNERS['peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound'],
 ('cli_cost','replay','cost::a_cold_run_over_ruff_holds_the_peak_ratchet'):OWNERS['cli_cost'],
 ('cli_cost','replay','cost::a_warm_run_and_an_edit_run_over_ruff_hold_their_ratchets'):OWNERS['cli_cost'],
 ('cli_cost','replay','cost::every_live_rule_together_holds_the_cold_ratchet_of_ruff'):OWNERS['cli_cost'],
 ('core_allocation','allocation','allocation::core::reading_allocates_as_the_parser_library_does'):OWNERS['core_allocation'],
}
RED_OWNERS={**RED_OWNERS_ALL_FEATURES,
 # default_features runs the same tests without --all-features: each red it shares with a command above has that command's owner.
 **{('default_features',binary,test):RED_OWNERS_ALL_FEATURES[(command,binary,test)] for command,binary,test in RED_OWNERS_ALL_FEATURES if command in ('libraries','cli_tests','cli_cost','core_allocation')},
 # The python language's allocation runner reports its case by id (the SDK's harness_case); red under every feature set, as core's is.
 ('default_features','allocation','allocation::python::reading_allocates_as_the_parser_library_does'):RUST_REGRESSIONS,
 **{('sdk_api','semver-checks',lint):SDK_RELEASE for lint in ('constructible_struct_adds_field','enum_missing','enum_no_repr_variant_discriminant_changed','struct_missing')},
}
# A command that cannot run on this machine says so on its first output line (`not available: <why>`) and exits non-zero; the row
# named here owns that refusal, and only that refusal. Every run prints it as not run (`ruff_parity`: parity not run).
UNAVAILABLE_OWNERS={'ruff_parity':CLOSE}
NOT_RUN={'ruff_parity':'parity not run'}
NOT_AVAILABLE_LINE=re.compile(r'^not available: (.+)$')
def unavailable(log):
 """The reason a command's raw log gives on its first line for not running here, or None."""
 lines=Path(log).read_text(errors='replace').splitlines() if Path(log).is_file() else []
 m=NOT_AVAILABLE_LINE.match(lines[0]) if lines else None
 return m.group(1) if m else None
def command_owners(name,check,tests):
 """The rows that own a red command: its UNAVAILABLE_OWNERS row for a recorded `not available` refusal, else the RED_OWNERS rows of
 its recorded red tests; [] when it recorded no red test or one of them has no exact entry."""
 if check.get('not_available'): return [UNAVAILABLE_OWNERS[name]] if name in UNAVAILABLE_OWNERS else []
 owners=[RED_OWNERS.get((name,binary,test)) for binary,t in tests.get(name,{}).items() for test in t['failed']]
 return sorted(set(owners)) if owners and all(owners) else []
def red_commands(v):
 """(name, exit, owning rows) of every command the validation recorded red."""
 return [(n,c['exit'],command_owners(n,c,v.get('tests',{}))) for n,c in v.get('checks',{}).items() if c.get('exit',0)!=0]
def open_row_problems(readme_text):
 """Each owner of the exit (RED_OWNERS, UNAVAILABLE_OWNERS) that is no open README row: a ticked row owns nothing."""
 bad=[]
 for key,title in [*RED_OWNERS.items(),*UNAVAILABLE_OWNERS.items()]:
  if '[ ] '+title in readme_text: continue
  bad.append('owner of '+str(key)+(' is a ticked README row: ' if '[x] '+title in readme_text else ' is no README row: ')+title[:80])
 return bad
OWNED=re.compile(r'(?:; )?(?:[a-z_]+ )?owned by "[^"]*"')
def owned(note,clauses):
 """The note without its owner clauses, then one `owned by "<row title>"` clause per owner."""
 return '; '.join([x for x in [OWNED.sub('',note).strip('; ')] if x]+clauses)
LISTED=('libraries_total','cli_replay')
def red(x):
 """A result that failed a row or exited non-zero; `libraries_total` and `cli_replay` total reds that `core_library`, `benchmark_library` and `cli_replay_reds` list one by one."""
 return isinstance(x,dict) and (x.get('failed',0)>0 or x.get('exit',0)!=0)
def own(v,readme_text):
 """Each red result carries its owning row's exact title in its validation.json note, from OWNERS; a green result carries none. A red with no owner and a title that is no README row are reported. Returns (changed, problems)."""
 results=v['results']; bad=[]; before=json.dumps(results)
 reds=results.get('cli_replay_reds',[])
 for key,title in OWNERS.items():
  if not (key in reds or red(results.get(key))): bad.append('stale owner: '+key+' is not red, so OWNERS may not pre-own it')
  if '[ ] '+title in readme_text: continue
  bad.append('owner of '+key+(' is a ticked README row: ' if '[x] '+title in readme_text else ' is no README row: ')+title[:80])
 for key,x in results.items():
  if not isinstance(x,dict) or key in LISTED: continue
  if red(x):
   if key in OWNERS: x['note']=owned(x.get('note',''),['owned by "'+OWNERS[key]+'"'])
   else: bad.append('red '+key+' has no owner in OWNERS')
  elif 'note' in x: x['note']=owned(x['note'],[])
 if reds: results['cli_replay_note']=owned(results.get('cli_replay_note',''),[r.split('::')[0]+' owned by "'+OWNERS[r]+'"' for r in reds if r in OWNERS])
 bad+=['red '+r+' has no owner in OWNERS' for r in reds if r not in OWNERS]
 return json.dumps(results)!=before,bad
def write_block(text,v):
 """Replace the block from the `### Current ownership checkpoint` heading to the first following bullet."""
 lines=text.splitlines(keepends=True)
 start=next(i for i,l in enumerate(lines) if l.startswith('### Current ownership checkpoint'))
 end=next(i for i in range(start+1,len(lines)) if lines[i].startswith('- ['))
 return ''.join(lines[:start])+render(v)+''.join(lines[end:])
def red_line(name,code,owners):
 """A red command as `--check` prints it."""
 return 'RED '+name+' exit '+str(code)+(', owned by '+'; '.join('"'+x+'"' for x in owners) if owners else ', owned by no row')
# The results measured by a run outside commands.json (a rerun the record cites by its log), which the Known red section lists after the
# red commands.
RERUN_RESULTS=('tastemaker_workspace',)
def known_red(v):
 """The review README's `## Known red` section, from the validation alone: each red command as `--check` prints it with its red tests
 by exact id, then each result of RERUN_RESULTS that records a failure, with its note."""
 out=['## Known red\n','\n','Written by `run.py` from `validation.json` (validation'+str(v['validation'])+', validated source `'+v['source'][:10]+'`); a run rewrites it, so it\n','names exactly the red commands `run.py --check` prints, each red test by its exact id, and their owners.\n','\n']
 for name,code,owners in red_commands(v):
  out.append('- `'+red_line(name,code,owners)+'`\n')
  for binary,t in sorted(v.get('tests',{}).get(name,{}).items()):
   for test in t.get('failed',{}): out.append('  - `'+binary+'` `'+test+'`\n')
 for key in RERUN_RESULTS:
  x=v.get('results',{}).get(key)
  if isinstance(x,dict) and x.get('failed'): out.append('- `'+key+'` '+str(x.get('passed',0))+' / '+str(x['failed'])+': '+x['note']+'\n')
 return out
def write_known_red(text,v):
 """Replace the review README's `## Known red` section, up to the next `## ` heading, with known_red(v)."""
 lines=text.splitlines(keepends=True)
 start=next(i for i,l in enumerate(lines) if l.startswith('## Known red'))
 end=next((i for i in range(start+1,len(lines)) if lines[i].startswith('## ')),len(lines))
 return ''.join(lines[:start])+''.join(known_red(v))+'\n'+''.join(lines[end:])
def count_problems(v):
 """Each result of a cargo test command that gives counts must be what its recorded tests give: passed, failed and not run, so a
 test that printed `not available` and measured nothing is counted as not run, never as passed."""
 bad=[]
 for name,binaries in v.get('tests',{}).items():
  x=v['results'].get(name)
  if not binaries or not (isinstance(x,dict) and 'passed' in x): continue
  given={'passed':sum(len(t['passed']) for t in binaries.values()),'failed':sum(len(t['failed']) for t in binaries.values()),'not_run':sum(len(t.get('not_run',[])) for t in binaries.values())}
  typed={k:x.get(k,0) for k in given}
  if typed!=given: bad.append('result '+name+' gives '+str(typed)+', its recorded tests give '+str(given))
 return bad
def red_green_problems(v):
 """Each red_green entry must be {test, base_sha, merge_sha} with 40-hex shas."""
 return ['red_green entry '+str(n)+' is not {test, base_sha, merge_sha} with 40-hex shas' for n,x in enumerate(v.get('red_green',[])) if not (isinstance(x,dict) and set(x)=={'test','base_sha','merge_sha'} and isinstance(x['test'],str) and x['test'] and all(isinstance(x[s],str) and re.fullmatch('[0-9a-f]{40}',x[s]) for s in ('base_sha','merge_sha')))]
CONSOLIDATION='origin/claude/ryo-consolidation'
def git(repo,*args):
 r=subprocess.run(['git','-C',str(repo),*args],capture_output=True,text=True)
 return r.stdout if r.returncode==0 else None
def checkpoint_base(repo):
 """The previous landed head this checkpoint's diff is taken against: the merge base of HEAD and the consolidation branch, or None."""
 base=git(repo,'merge-base','HEAD',CONSOLIDATION)
 return base.strip() if base else None
def is_test_file(path):
 """The one test-file classifier (tm finding splits, FIX, DOC and the census): a test, fixture, bench, example or acceptance file, in such a directory (a fixtures*, *_test or *acceptance* one included), or named test.rs, tests.rs, fixtures.rs, *_test.rs or *_tests.rs; a Python test is test_*.py, *_test.py or run_test.py."""
 parts=path.split('/'); name=parts[-1]
 return any(x in ('tests','test','benches','benchmarks','examples') or x.startswith('fixtures') or x.endswith('_test') or 'acceptance' in x for x in parts[:-1]) or name in ('test.rs','tests.rs','fixtures.rs','run_test.py') or name.endswith(('_test.rs','_tests.rs','_test.py')) or name.endswith('.py') and name.startswith('test_')
HARNESS=('.claude/hooks/','ci/','crates/ryo/docs/contracts/evidence/checkpoint/')
def is_production(path):
 """A Rust source, or the Python of the harness (the hooks, ci/ and this validation script), that is no test, fixture or bench: a change to it can change behaviour."""
 return not is_test_file(path) and (path.endswith('.rs') or path.startswith(HARNESS) and path.endswith('.py'))
PERF=re.compile(r'^perf(\([^)]*\))?!?:')
PERF_EVIDENCE='crates/ryo/docs/contracts/evidence/perf/'
NAMED_TEST=re.compile(r'\bgate:[a-z][a-z0-9_]*|\b[a-z][a-z0-9]*\.[a-z][a-z0-9]*(?:/[a-z][a-z0-9-]*[a-z0-9])+|\b(?:[a-z_][a-z0-9_]*::)+[a-z_][a-z0-9_]*\b|\btest_\w+|\b[a-z][a-z0-9]*(?:_[a-z0-9]+){3,}\b')
def named_tests(message):
 """The tests a commit message names; a pytest node id's `<file>.py::<test>` names `<test>` (the `py` is the file's extension)."""
 return [t[len('py::'):] if t.startswith('py::') else t for t in NAMED_TEST.findall(message) if not t.startswith(('crates::','std::','self::','super::'))]
CARGO_LINE=re.compile(r'^test (\S+) \.\.\. (ok|FAILED)\b')
# nextest prints `PASS [ 0.003s] ( 12/153) <binary> <test>`; older versions leave the counter out.
NEXTEST_LINE=re.compile(r'^\s*(PASS|FAIL|SIGSEGV|SIGABRT|TIMEOUT)\s+\[[^\]]*\]\s+(?:\(\s*\d+/\d+\)\s+)?\S+\s+(\S+)')
UNITTEST_LINE=re.compile(r'^(\w+) \(([\w.]+)\)\S*.* \.\.\. (ok|FAIL|ERROR)\b')
UNITTEST_FAILURE=re.compile(r'^(FAIL|ERROR): (\w+) \(([\w.]+)\)')
# pytest -v: `path::test PASSED` per test, and the short summary's `FAILED path::test` or `ERROR path::test`.
PYTEST_LINE=re.compile(r'^(\S+::[\w\[\]-]+) (PASSED|FAILED|ERROR)\b')
PYTEST_SUMMARY=re.compile(r'^(FAILED|ERROR) (\S+::[\w\[\]-]+)')
# The runner's own frame around its results: a log without it was not written by the runner. A forger can still type
# these lines; the check catches a hand-written result, not a deliberate fake.
RUNNER_FRAMES=((re.compile(r'(?m)^running \d+ tests?$'),re.compile(r'(?m)^test result: (ok|FAILED)\. \d+ passed')),
               (re.compile(r'(?m)^\s*Starting \d+ tests? across'),re.compile(r'(?m)^\s*Summary \[')),
               (re.compile(r'(?m)^Ran \d+ tests? in '),re.compile(r'(?m)^(OK|FAILED \()')),
               (re.compile(r'(?m)^=+ test session starts =+$'),re.compile(r'(?m)^=+ .*\b\d+ (passed|failed|error)\b.* in [\d.]+s')))
def runner_framed(text): return any(head.search(text) and tail.search(text) for head,tail in RUNNER_FRAMES)
CLIPPY_LINT=re.compile(r'^(?:clippy|rustc)::[a-z_]+$')
GATE=re.compile(r'^gate:[a-z][a-z0-9_]*$')
def gate_status(log):
 """A validation command's status from its raw log: the command's own result beside it (`<log stem>.json`, as validate() writes
 `{"exit": n}`): 'passed' for exit 0, 'failed' for a non-zero exit whose log holds its refusal, else None."""
 result=Path(log).with_suffix('.json')
 try: code=json.loads(result.read_text()).get('exit')
 except (OSError,ValueError,AttributeError): return None
 if not isinstance(code,int): return None
 if code==0: return 'passed'
 return 'failed' if Path(log).read_text(errors='replace').strip() else None
TM_RULE=re.compile(r'^[a-z][a-z0-9]*\.[a-z][a-z0-9]*(?:/[a-z][a-z0-9-]*)+$')
def tm_status(text,rule,files):
 """A Tastemaker rule's status in one `tm check --format json` log: 'failed' when a finding of the rule (or of one of its parts,
 `rule#part`) is at one of the files (repository paths) or at a directory holding one, 'passed' when the document holds no such finding, else None."""
 try: report=json.loads(text)
 except ValueError: return None
 if not isinstance(report,dict) or not isinstance(report.get('findings'),list): return None
 at=lambda path: isinstance(path,str) and any(x==path or x.startswith(path.rstrip('/')+'/') for x in files)
 hit=any(isinstance(f,dict) and str(f.get('rule','')).split('#')[0]==rule and at(f.get('path')) for f in report['findings'])
 return 'failed' if hit else 'passed'
def clippy_status(text,lint,files):
 """A clippy lint's status in one `cargo clippy --message-format=json` log: 'failed' when it reports the lint at a primary span in one of
 the files (repository paths; a span path is relative to its workspace), 'passed' when the run finished without it there, else None."""
 finished=False; hit=False
 for line in text.splitlines():
  if not line.startswith('{'): continue
  try: m=json.loads(line)
  except ValueError: continue
  if m.get('reason')=='build-finished': finished=True
  if m.get('reason')!='compiler-message' or ((m.get('message') or {}).get('code') or {}).get('code')!=lint.removeprefix('rustc::'): continue
  spans=[x['file_name'] for x in m['message'].get('spans',[]) if x.get('is_primary')]
  hit|=any(f==x or f.endswith('/'+x.lstrip('./')) for x in spans for f in files)
 return 'failed' if hit else 'passed' if finished else None
COMPILE_RED=re.compile(r'(?m)^error: could not compile `[^`]+` \((?:lib test|test "[^"]+"|bin test)\)')
def log_status(log,test,files=()):
 """'failed' when the log shows the test failing, 'passed' when it shows it passing and never failing, else None: cargo test, cargo nextest, unittest -v and pytest -v lines inside the runner's own frame, a test named by its path or a suffix of it; a `clippy::<lint>` test is read from clippy's JSON at the given files (clippy_status)."""
 named=lambda path,sep: path==test or path.endswith(sep+test)
 failed=passed=False; text=Path(log).read_text(errors='replace')
 if CLIPPY_LINT.match(test): return clippy_status(text,test,files)
 if TM_RULE.match(test): return tm_status(text,test,files)
 if GATE.match(test): return gate_status(log)
 # A test that does not build is red: the overlaid test file names what its commit adds (cargo's `could not compile` a test target).
 if not runner_framed(text): return 'failed' if COMPILE_RED.search(text) else None
 for line in text.splitlines():
  m=CARGO_LINE.match(line)
  if m and named(m.group(1),'::'): failed|=m.group(2)=='FAILED'; passed|=m.group(2)=='ok'; continue
  m=NEXTEST_LINE.match(line)
  if m and named(m.group(2),'::'): failed|=m.group(1)!='PASS'; passed|=m.group(1)=='PASS'; continue
  m=UNITTEST_LINE.match(line)
  if m and (named(m.group(1),'.') or named(m.group(2),'.')): failed|=m.group(3)!='ok'; passed|=m.group(3)=='ok'; continue
  m=UNITTEST_FAILURE.match(line)
  if m and (named(m.group(2),'.') or named(m.group(3),'.')): failed=True
  m=PYTEST_LINE.match(line)
  if m and named(m.group(1).split('[')[0],'::'): failed|=m.group(2)!='PASSED'; passed|=m.group(2)=='PASSED'; continue
  m=PYTEST_SUMMARY.match(line)
  if m and named(m.group(2).split('[')[0],'::'): failed=True
 return 'failed' if failed else 'passed' if passed else None
def covered(repo,entry,ranges):
 """The commits the red_green entry's range holds: in its merge sha and not in its base sha (read once per range)."""
 key=(entry.get('base_sha',''),entry.get('merge_sha','')) if isinstance(entry,dict) else ('','')
 if key not in ranges: ranges[key]=set((git(repo,'rev-list',key[0]+'..'+key[1]) or '').split()) if all(key) else set()
 return ranges[key]
def proves(entry,test,raw,files=()):
 """The entry names the test and two distinct logs, `red <name>.log` showing it fail and `green <name>.log` showing it pass, both in the validation directory."""
 text=entry.get('test','')
 if not re.search(r'(?<![\w:])'+re.escape(test)+r'(?![\w])',text): return False
 red,green=re.search(r'\bred ([\w.-]+\.log)\b',text),re.search(r'\bgreen ([\w.-]+\.log)\b',text)
 if not red or not green or red.group(1)==green.group(1) or not (raw/red.group(1)).is_file() or not (raw/green.group(1)).is_file(): return False
 # A cases-table row is proven by its table's test: `<row> (row of <test>; ...)` reads the logs for that test.
 row=re.search(r'(?<![\w:])'+re.escape(test)+r'\s*\(row of ([\w:]+)',text)
 target=row.group(1) if row else test
 return log_status(raw/red.group(1),target,files)=='failed' and log_status(raw/green.group(1),target,files)=='passed'
def perf_identical(repo,base,head,sha):
 """A perf commit's proof: an after.json under evidence/perf/ added or changed in the checkpoint, with identical_output true and a source that is the perf commit or a commit built on it."""
 for path in (git(repo,'diff','--name-only','--diff-filter=AM',base,head,'--',':(top)'+PERF_EVIDENCE) or '').split():
  if path.endswith('/after.json'):
   try: record=json.loads(git(repo,'show',head+':'+path) or '')
   except ValueError: continue
   if isinstance(record,dict) and record.get('identical_output') is True and isinstance(record.get('source'),str) and re.fullmatch('[0-9a-f]{40}',record['source']) and subprocess.run(['git','-C',str(repo),'merge-base','--is-ancestor',sha,record['source']],capture_output=True).returncode==0: return True
 return False
NO_BASE='no checkpoint base: the merge base of HEAD and '+CONSOLIDATION+' is unknown; fetch it'
# FIX took effect when the checkpoint-loop branch merged: its merge commit and every commit it reaches are before the rule.
FIX_BASELINE='f783d87661635edc4fc14c2d28926323830f7164'
def fix_problems(v,repo=None,base=None,head='HEAD',baseline=FIX_BASELINE):
 """Each commit of the checkpoint not reachable from the baseline that changes production code (a merge by its own changes), other than a perf commit (which needs an after.json with identical_output true), names a test in its message that a red_green entry over it (its base_sha excluded, its merge_sha included) proves: the entry names that test, its red log shows it fail and its green log shows it pass."""
 repo=repo or p; base=base or checkpoint_base(repo)
 if not base: return [NO_BASE]
 bad=[]; raw=raw_dir(v); ranges={}; proven=set(); unproven={}; merges={}
 before=['^'+baseline] if baseline and git(repo,'cat-file','-e',baseline+'^{commit}') is not None else []
 for sha in (git(repo,'rev-list',base+'..'+head,*before) or '').split():
  message=git(repo,'show','-s','--format=%B',sha) or ''
  # A merge's own changes are the files its result changes against every parent (the combined diff).
  own=['diff-tree','--cc','--no-commit-id','--name-only',sha] if len((git(repo,'rev-list','--parents','-n1',sha) or '').split())>2 else ['diff-tree','--root','--no-commit-id','-r','--name-only',sha]
  files=(git(repo,*own) or '').split()
  if not any(is_production(f) and not comment_only(repo,sha,f,own) and not test_only(repo,sha,f,own) and not visibility_only(repo,sha,f,own) for f in files): continue
  short=sha[:10]+' '+message.splitlines()[0][:70]
  if PERF.match(message):
   if not perf_identical(repo,base,head,sha): bad.append('perf '+short+': no evidence/perf/*/after.json added or changed in the checkpoint with identical_output true and a source built on it')
   continue
  tests=named_tests(message)
  entries=[x for x in v.get('red_green',[]) if sha in covered(repo,x,ranges)]
  tables=[t for x in entries for t in [table_of(x,raw)] if t is not None]
  # A range proven by a validation command (gate:<command>, red then green): every commit the entry covers is proven by it.
  if any(gate_proves(x,raw) for x in entries): proven.add(sha); continue
  # A range with no possible red (a refactor of the harness) is proven by the same answer before and after it (identical_proves).
  if any(identical_proves(x,raw) for x in entries): proven.add(sha); continue
  # A range proven by a table: a commit naming no test is covered by the whole table, one naming tests by a row of it.
  # A commit of the range with no possible red (a move behind the seam, no behaviour change) is covered by a table entry that says
  # so and cites the logs that show its guard green before and after.
  moved=any(MOVE in x.get('test','') and table_of(x,raw) is not None and (cited:=re.findall(r'\b[\w.-]+\.log\b',x['test'])) and all((raw/c).is_file() for c in cited) for x in entries)
  if tables and (not tests or moved or any(t in rows for rows in tables for t in tests)): proven.add(sha); continue
  if len((git(repo,'rev-list','--parents','-n1',sha) or '').split())>2: merges[sha]=short; continue
  if not tests: unproven[sha]='fix '+short+': its message names no test that fails before it and passes after'
  elif not entries: unproven[sha]='fix '+short+': no red_green entry has it between its base_sha and merge_sha'
  elif not any(proves(x,t,raw,files) for x in entries for t in tests):
   unproven[sha]='fix '+short+': no red_green entry over it names a test its message names ('+', '.join(tests[:4])+') with a red log showing it fail and a green log showing it pass, in '+str(raw)
  else: proven.add(sha)
 # Identical change, identical proof: a commit whose stable patch-id is that of a proven commit (a reworded or re-applied twin) is proven.
 twins={patch_id(repo,sha) for sha in proven}-{None}
 unproven={sha:why for sha,why in unproven.items() if patch_id(repo,sha) not in twins}
 # A merge is proven when every commit its merged-in parent brings (after the baseline) is proven and the validated tree passes the
 # tests those commits name that a validation command runs.
 for sha,short in merges.items():
  parents=(git(repo,'rev-list','--parents','-n1',sha) or '').split()[1:]
  brought=(git(repo,'rev-list',*parents[1:],'^'+parents[0],*before) or '').split()
  named=[t for c in brought for t in named_tests(git(repo,'show','-s','--format=%B',c) or '')]
  failing=[t for t in named if validated_status(v,t)=='failed']
  if any(c in unproven for c in brought): unproven[sha]='merge '+short+': a commit it merges in is unproven'
  elif failing: unproven[sha]='merge '+short+': the validated tree fails a test its merged commits name ('+', '.join(failing[:4])+')'
 return bad+list(unproven.values())
TABLE=re.compile(r'\btable ([\w.-]+\.json)\b')
MOVE='move behind the seam, no behaviour change'
MANIFEST=re.compile(r'\bmanifest ([\w./-]+\.toml)\b')
# The repository's root: evidence/checkpoint is crates/ryo/docs/contracts/evidence/checkpoint.
REPO=p.parents[5]
TABLE_STATUSES=('caught','equivalent','unobservable')
def table_of(entry,raw):
 """The tests a range entry's table proves, or None: the entry names `table <file>.json` in the validation directory, a list of
 rows {semantic, status, ...} accounting for every mutant the entry's `manifest <file>.toml` (a repository file, `mutants = [...]`)
 lists, no more and no fewer. A `caught` row names a test whose red log shows it fail under its planted
 bug and whose green log shows it pass on the landed code; an `equivalent` or `unobservable` row carries a non-empty argument and
 the review evidence that accepted it (a file of the repository or of the validation directory)."""
 text=entry.get('test','') if isinstance(entry,dict) else ''
 m=TABLE.search(text); listed=MANIFEST.search(text)
 if not m or not (raw/m.group(1)).is_file() or not listed or not (REPO/listed.group(1)).is_file(): return None
 import tomllib
 try: mutants=tomllib.loads((REPO/listed.group(1)).read_text()).get('mutants')
 except tomllib.TOMLDecodeError: return None
 if not isinstance(mutants,list) or not mutants: return None
 try: rows=json.loads((raw/m.group(1)).read_text())
 except ValueError: return None
 if not isinstance(rows,list) or not rows: return None
 if sorted(r.get('semantic') for r in rows if isinstance(r,dict))!=sorted(mutants): return None
 caught=set()
 for row in rows:
  status=row.get('status','caught') if isinstance(row,dict) else None
  if status not in TABLE_STATUSES or not isinstance(row.get('semantic'),str): return None
  if status!='caught':
   evidence=row.get('evidence')
   if not (isinstance(row.get('argument'),str) and row['argument'].strip() and isinstance(evidence,str) and evidence and ((REPO/evidence).is_file() or (raw/evidence).is_file())): return None
   continue
  if not all(isinstance(row.get(k),str) for k in ('test','red','green')): return None
  if not ((raw/row['red']).is_file() and (raw/row['green']).is_file()): return None
  if log_status(raw/row['red'],row['test'])!='failed' or log_status(raw/row['green'],row['test'])!='passed': return None
  caught.add(row['test'])
 return caught
GATE_NAMED=re.compile(r'(?<![\w:])(gate:[a-z][a-z0-9_]*)\b')
def gate_proves(entry,raw):
 """The entry names `gate:<command>` with a `red <log>` whose command exited non-zero with its refusal and a `green <log>` whose
 command exited 0."""
 text=entry.get('test','') if isinstance(entry,dict) else ''
 m=GATE_NAMED.search(text)
 return bool(m) and proves(entry,m.group(1),raw)
IDENTICAL=re.compile(r'\bidentical ([\w.-]+\.log) ([\w.-]+\.log)\b')
def identical_proves(entry,raw):
 """The entry says `identical <before>.log <after>.log`: two distinct, non-empty logs in the validation directory with the same bytes,
 the answer of the code before the range and of the code after it to one question over one tree (a refactor's proof: no red exists)."""
 m=IDENTICAL.search(entry.get('test','') if isinstance(entry,dict) else '')
 if not m or m.group(1)==m.group(2): return False
 a,b=raw/m.group(1),raw/m.group(2)
 return a.is_file() and b.is_file() and a.stat().st_size>0 and a.read_bytes()==b.read_bytes()
def validated_status(v,test):
 """'failed' or 'passed' for a test the validation's recorded tests hold (by its path or a suffix of it), else None."""
 found=None
 for binaries in v.get('tests',{}).values():
  for t in binaries.values():
   for name in t.get('failed',{}):
    if name==test or name.endswith('::'+test): return 'failed'
   for name in t.get('passed',[]):
    if name==test or name.endswith('::'+test): found='passed'
 return found
LANGUAGE=tree_sitter.Language(tree_sitter_rust.language())
RUST=tree_sitter.Parser(LANGUAGE)
@lru_cache(maxsize=64)
def parsed(text):
 """The tree-sitter tree of a Rust text and its UTF-8 bytes, which the tree's offsets index."""
 src=text.encode(); return RUST.parse(src),src
def captured(query,node):
 """The `@n` captures of a tree-sitter query under node, in source order."""
 return sorted(tree_sitter.QueryCursor(query).captures(node).get('n',[]),key=lambda x:(x.start_byte,-x.end_byte))
def quiet_lines(text):
 """The 0-based lines of a Rust text made only of comments, blank space and lint attributes (`#[expect(clippy::x, reason = "...")]`,
 `#![deny(y)]`, over any number of lines): no other token touches them."""
 tree,_=parsed(text); loud=set(); stack=[tree.root_node]
 while stack:
  n=stack.pop()
  if n.type in ('line_comment','block_comment') or n.type in ('attribute_item','inner_attribute_item') and n.named_children[0].children[0].text in (b'expect',b'allow',b'warn',b'deny',b'forbid'): continue
  if n.child_count: stack.extend(n.children)
  elif n.end_byte>n.start_byte: loud.update(range(n.start_point.row,n.end_point.row+(n.end_point.column>0)))
 return set(range(text.count('\n')+1))-loud
HUNK=re.compile(r'^@@+ ((?:-\d+(?:,\d+)? )+)\+(\d+)')
def comment_only(repo,sha,path,own):
 """A Rust file whose change in the commit adds and removes only quiet lines (quiet_lines: comments, blank space, lint attributes), each
 judged on its side, the commit's file for an added line and the parent it leaves for a removed one: lint configuration and
 comments change no behaviour, so FIX does not count it. A Cargo.toml lints table is no production file to begin with."""
 if not path.endswith('.rs'): return False
 diff=git(repo,*([x for x in own if x!='--name-only']+['-p','--',':(top)'+path])) or ''
 width=2 if own[1]=='--cc' else 1
 sides={}; at=[0]*(width+1); changed=False
 for l in diff.splitlines():
  if hunk:=HUNK.match(l): at=[int(hunk.group(2))]+[int(x.split(',')[0]) for x in hunk.group(1)[1:].split(' -') if x]; continue
  if l.startswith(('+++','---')) or len(l)<width or l[:width].strip('+- '): continue
  mark=l[:width]; removed='-' in mark; side=mark.index('-')+1 if removed else 0
  if mark.strip():
   if side not in sides: sides[side]=quiet_lines(git(repo,'show',(sha if side==0 else sha+'^'+str(side))+':'+path) or '')
   if at[side]-1 not in sides[side]: return False
   changed=True
  at=[at[0]+(not removed)]+[x+((c=='-') if removed else (c==' ')) for x,c in zip(at[1:],mark)]
 return changed
def test_only(repo,sha,path,own):
 """A Rust file whose production text (split_tests: without its #[cfg(test)] items, the census's one definition) is the same before
 and after the commit, its quiet lines aside: the commit changed only its test code. A merge is judged by its combined diff instead
 (comment_only)."""
 if not path.endswith('.rs') or own[1]=='--cc': return False
 before=git(repo,'show',sha+'^:'+path); after=git(repo,'show',sha+':'+path)
 if before is None or after is None: return False
 def plain(text):
  production=split_tests(text).production; quiet=quiet_lines(production)
  return [x for n,x in enumerate(production.split('\n')) if n not in quiet]
 return plain(before)==plain(after)
# A visibility modifier: `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`.
VISIBILITY=re.compile(r'\bpub(?:\s*\([^)]*\))?\s+')
def visibility_only(repo,sha,path,own):
 """A Rust file whose change in the commit is only visibility modifiers (a seal, `pub` to `pub(crate)`, or its reverse), comments
 and lint attributes aside: what compiles runs the same, so FIX does not count it; the compiler decides whether the seal holds.
 A merge is judged by its combined diff instead (comment_only)."""
 if not path.endswith('.rs') or own[1]=='--cc': return False
 before=git(repo,'show',sha+'^:'+path); after=git(repo,'show',sha+':'+path)
 if before is None or after is None: return False
 def plain(text):
  quiet=quiet_lines(text)
  return [VISIBILITY.sub('',x) for n,x in enumerate(text.split('\n')) if n not in quiet]
 return plain(before)==plain(after)
def patch_id(repo,sha):
 """The commit's `git patch-id --stable`, or None for a commit with no diff against its first parent."""
 diff=subprocess.run(['git','-C',str(repo),'show','--format=','--no-color',sha],capture_output=True)
 found=subprocess.run(['git','-C',str(repo),'patch-id','--stable'],input=diff.stdout,capture_output=True).stdout.decode().split()
 return found[0] if found else None
# The items the Rust API guidelines give an example (C-EXAMPLE), by node, and by `pub` token in a macro body; a `Sealed` supertrait is the
# seal, not API.
ITEM_KINDS={'function_item':'fn','function_signature_item':'fn','struct_item':'struct','enum_item':'enum','trait_item':'trait','type_item':'type','union_item':'union'}
ITEMS=tree_sitter.Query(LANGUAGE,'['+' '.join('('+x+')' for x in ITEM_KINDS)+'] @n (token_tree "pub" @n)')
PubItem=NamedTuple('PubItem',[('owner',str|None),('kind',str),('name',str),('line',int),('doc',str)])
def type_name(node):
 """The name of a type, its generics, path and reference stripped: `Vec<T>` and `&'a crate::x::Vec<T>` -> Vec."""
 while node.type in ('generic_type','reference_type','pointer_type','scoped_type_identifier'): node=node.child_by_field_name('name' if node.type=='scoped_type_identifier' else 'type')
 return node.text.decode()
def item_owner(node):
 """The impl type, module or trait that holds an item (`impl Vec`, `mod tests`), or, in a macro body, the header tokens before the
 nearest `{ }` tree around it (`impl DerivedKind`); None at the top."""
 while (node:=node.parent) is not None:
  if node.type in ('impl_item','mod_item','trait_item'): return node.type[:-5]+' '+type_name(node.child_by_field_name('type' if node.type=='impl_item' else 'name'))
  if node.type=='token_tree' and node.text.startswith(b'{') and node.parent is not None:
   tokens=node.parent.children; i=tokens.index(node); header=[]
   while i>0 and tokens[i-1].type not in (';','token_tree'): i-=1; header.insert(0,tokens[i].text.decode())
   if header[:1]==['impl'] or header[:1] in (['mod'],['trait']): return ' '.join(header)
 return None
def doc_above(tokens,i,src):
 """The outer doc comments (`///`) and attributes (`#[..]`, as a node or as tokens) directly above tokens[i] among its siblings, with no
 blank line between them, top first, one stripped line each."""
 spans=[]; end=tokens[i].start_byte
 while i>0:
  t=tokens[i-1]; first=i-1
  if t.type=='token_tree' and t.text.startswith(b'['): first-=1+(i>2 and tokens[i-2].type=='!')
  elif not (t.type in ('attribute_item','inner_attribute_item') or t.type=='line_comment' and t.text.startswith(b'///')): break
  if first<0 or tokens[first].type!='#' and t.type=='token_tree' or src[t.end_byte:end].count(b'\n')+t.text.endswith(b'\n')>1: break
  spans.insert(0,src[tokens[first].start_byte:t.end_byte]); end=tokens[first].start_byte; i=first
 return '\n'.join(x.strip() for span in spans for x in span.decode().split('\n') if x.strip())
def items(text):
 """Each pub item of a Rust source: its owner, kind, name, 0-based line and doc."""
 tree,src=parsed(text); found=[]
 for n in captured(ITEMS,tree.root_node):
  siblings=n.parent.children if n.parent is not None else [n]
  if n.type in ITEM_KINDS and (v:=next((c for c in n.children if c.type=='visibility_modifier'),None)) is not None and v.text==b'pub':
   found.append(PubItem(item_owner(n),ITEM_KINDS[n.type],n.child_by_field_name('name').text.decode(),v.start_point.row,doc_above(siblings,siblings.index(n),src)))
  elif n.type=='pub':
   j=siblings.index(n)+1
   while j<len(siblings) and siblings[j].type in ('const','async','unsafe'): j+=1
   if j+1<len(siblings) and siblings[j].type in ITEM_KINDS.values() and siblings[j+1].type=='identifier':
    found.append(PubItem(item_owner(n),siblings[j].type,siblings[j+1].text.decode(),n.start_point.row,doc_above(siblings,siblings.index(n),src)))
 return found
def published(repo,path,rev):
 """The file's crate is published: its nearest Cargo.toml with a [package] does not say publish = false."""
 crate=crate_dir(repo,path,rev)
 return crate is not None and not re.search(r'(?m)^publish\s*=\s*false',git(repo,'show',rev+':'+crate+'/Cargo.toml'))
CRATES={}
def crate_dir(repo,path,rev):
 """The directory of the file's nearest Cargo.toml with a [package], or None."""
 parts=path.split('/'); key=(str(repo),rev,'/'.join(parts[:-1]))
 if key not in CRATES:
  CRATES[key]=None
  for n in range(len(parts)-1,0,-1):
   text=git(repo,'show',rev+':'+'/'.join(parts[:n])+'/Cargo.toml')
   if text is not None and '[package]' in text: CRATES[key]='/'.join(parts[:n]); break
 return CRATES[key]
def module_path(crate,path):
 """The module path of a source file in its crate: src/a/b.rs, src/a/b/mod.rs -> a::b; src/lib.rs, src/main.rs -> the crate root."""
 parts=path[len(crate)+1:].removesuffix('.rs').split('/')
 parts=parts[1:] if parts[:1]==['src'] else parts
 return '::'.join(x for x in parts if x not in ('lib','main','mod'))
def base_items(repo,base,crate):
 """(module path, owner, kind, name) of every pub item in the crate's production files at the base."""
 paths=[x for x in (git(repo,'ls-tree','-r','--full-name','--name-only',base,'--',':(top)'+crate) or '').split() if x.endswith('.rs') and not is_test_file(x)]
 return {(module_path(crate,x),)+i[:3] for x in paths for i in items(git(repo,'show',base+':'+x) or '')}
def has_example(doc):
 """The doc comment has an `# Examples` section holding a fenced code block before the next heading."""
 lines=[l.strip().removeprefix('///').strip() for l in doc.splitlines()]
 if '# Examples' not in lines: return False
 for line in lines[lines.index('# Examples')+1:]:
  if line.startswith('#') and not line.startswith('#['): return False
  if line.startswith('```'): return True
 return False
def doc_problems(repo=None,base=None,head='HEAD'):
 """Each pub item a published crate gains in the checkpoint (no item of its module path, owner, kind and name in that crate's production files at the base) has an `# Examples` section, and `# Errors` when it returns a Result."""
 repo=repo or p; base=base or checkpoint_base(repo)
 if not base: return [NO_BASE]
 bad=[]; olds={}
 for path in (git(repo,'diff','--name-only','--no-renames','--diff-filter=AM',base,head,'--',':(top,glob)**/*.rs') or '').split():
  if is_test_file(path) or '/src/' not in '/'+path or not published(repo,path,head): continue
  crate=crate_dir(repo,path,head)
  if crate not in olds: olds[crate]=base_items(repo,base,crate)
  text=git(repo,'show',head+':'+path) or ''; lines=text.split('\n')
  for item in items(text):
   if (module_path(crate,path),)+item[:3] in olds[crate] or (item.kind,item.name)==('trait','Sealed'): continue
   n=item.line; doc=item.doc; where=path+':'+str(n+1)+' pub '+item.kind+' '+item.name
   if not has_example(doc): bad.append(where+': no `# Examples` section with a code block')
   signature=' '.join(lines[n:n+12]).split('{')[0].split(';')[0]
   if item.kind=='fn' and re.search(r'->\s*(?:[\w:]+::)?Result\b',signature) and '# Errors' not in doc: bad.append(where+': returns a Result with no `# Errors` section')
 return bad
def finish(v,check=False):
 """Write the matrix numbers block from validation.json, print the scope delta, un-tick each ticked row that cites no test or a test the recorded logs do not prove, check each ticked row cites a validation and each rerun result names its raw log. Returns the exit code, from the validation's results alone: 1 for a red command no row owns, a ticked row the results do not prove, or a record that breaks its own rules. Rewriting the numbers block is what a full run does; under `check` (`--check`, over the committed record) a rewrite means the committed record is stale, which fails with its reason."""
 old=readme.read_text(); owners_changed,owner_bad=own(v,old); text=write_block(old,v)
 if owners_changed: validation_json.write_text(dumps({k:x for k,x in v.items() if k!='tests'})+'\n')
 for line in scope_delta(v): print(line)
 bad=list(owner_bad)+open_row_problems(old)+red_green_problems(v)+count_problems(v)+census_problems()+lock_problems(); diff=['FIX '+x for x in fix_problems(v)]+['DOC '+x for x in doc_problems()]; texts=[]; changed=['validation.json'] if owners_changed else []; unticked_any=False
 for part in sorted(contracts.glob('*.md')):
  before=old if part==readme else part.read_text(); body,unticked=untick_unproven(text if part==readme else before,v); where='' if part==readme else part.name+' '
  if body!=before: part.write_text(body); changed.append(part.name)
  for line in unticked: print('UNTICKED ROW '+where+line)
  unticked_any=unticked_any or bool(unticked)
  bad+=[where+x for x in ticked_rows_without_proof(body,v['validation'])]
  texts+=[(part.name+':'+str(n),line) for n,line in enumerate(body.splitlines(),1) if TICKED.match(line)]
 texts+=[('validation.json results '+k,OWNED.sub('',x['note'])) for k,x in v['results'].items() if isinstance(x,dict) and 'note' in x]
 bad+=reruns_without_log(texts,v)
 for line in bad: print('TICKED ROW '+line)
 unowned=[]
 for name,code,owners in red_commands(v):
  reason=v['checks'][name].get('not_available')
  if reason: print('NOT RUN '+name+': '+NOT_RUN.get(name,name+' not run')+': '+reason)
  print(red_line(name,code,owners))
  if not owners: unowned.append(name)
 review=contracts/'review'/'README.md'
 if review.exists():
  before=review.read_text(); after=write_known_red(before,v)
  if after!=before: review.write_text(after); changed.append('review/README.md')
 stale=result_problems(v)
 for line in stale: print('RESULT '+line)
 for line in diff: print(line)
 if changed: print(('STALE RECORD: the committed record is not what validation.json gives: ' if check else 'rewritten from validation.json (numbers block, owner notes, un-ticked rows): ')+', '.join(changed)+'; stage them')
 return 1 if bad or diff or unowned or unticked_any or stale or (check and changed) else 0
EVALUATOR='crates/execution_runtime/'
STORE='crates/execution_store/'
SEAM_PORTS={'DigestPort','DigestBytes','FactsPort','OperatorsPort','FactsError'}
RUNTIME_INTERNALS=('FileRunInput','FileSession','FileInput','RanRule','RuleFile','SourceRows','FileRules','FileFinished','EmittedRoot','KindSupport','SemanticSupport','CaptureContext','RuleSelection','AdmittedInput','InputAdmissions','RelationOwner','InvocationWorkspace')
PER_FILE_FACT=re.compile(r'binding_read|binding-read|BindingRead')
def census_production(path):
 """A production source of the product: under crates/*/src, cli/src or languages/*/src, and no test file (is_test_file)."""
 return bool(re.match(r'(crates/[^/]+/src|cli/src|languages/[^/]+/src)/',path)) and not is_test_file(path)
def census_crate(path): return path.split('/')[0] if path.startswith('cli/') else '/'.join(path.split('/')[:2])
CFG_TEST=b'#[cfg(test)]'
CFG_TESTS=tree_sitter.Query(LANGUAGE,'(attribute_item) @n (token_tree "#" @n)')
# Where a #[cfg(test)] attribute sits beside the item it gates; anywhere else it starts the item that holds it (a field initializer, a
# match arm).
CFG_HOLDERS=('source_file','declaration_list','block','field_declaration_list','enum_variant_list','ordered_field_declaration_list','parameters','arguments','array_expression','match_block','ERROR')
SplitText=NamedTuple('SplitText',[('production',str),('tests',str)])
def cut_end(attribute,src):
 """The end of the item a #[cfg(test)] attribute node gates: through its comma, or up to the closing brace of a list it ends."""
 target=attribute.parent if attribute.parent.type not in CFG_HOLDERS else attribute.next_named_sibling
 while target is not None and (target.type in ('line_comment','block_comment') or target.type=='attribute_item' and target.parent.type in CFG_HOLDERS): target=target.next_named_sibling
 if target is None: return attribute.end_byte
 after=target.next_sibling
 return after.end_byte if after is not None and after.type==',' else after.start_byte if after is not None and after.type=='}' and src[target.end_byte-1:target.end_byte] not in (b'}',b';') else target.end_byte
def token_cut_end(hash_token):
 """The end of the item `#` `[cfg(test)]` tokens gate in a macro body: through the first `;` or `,` outside its header's generics, or its
 `{ }` body, or up to the closing delimiter of the tree."""
 tokens=hash_token.parent.children; depth=0; header=True
 for t in tokens[tokens.index(hash_token)+2:-1]:
  body=t.type=='token_tree' and t.text.startswith(b'{')
  if depth==0 and (t.type in (',',';') or body): return t.end_byte
  if body or t.type=='=' and depth==0: header=False
  elif header and t.type in ('<','<<'): depth+=len(t.type)
  elif header and t.type in ('>','>>'): depth-=len(t.type)
 return tokens[-1].start_byte
def split_tests(text):
 """A production file's text without its #[cfg(test)] items, and the text of those items: each item's own extent in the tree-sitter
 tree (an attribute node beside its item or starting the item that holds it, or `#` `[cfg(test)]` tokens in a macro body), from the
 start of the attribute's line, the blank lines just above it included."""
 tree,src=parsed(text); cuts=[]
 for n in captured(CFG_TESTS,tree.root_node):
  if n.type=='attribute_item' and n.text==CFG_TEST: end=cut_end(n,src)
  elif n.type=='#' and (t:=n.next_sibling) is not None and t.text==CFG_TEST[1:]: end=token_cut_end(n)
  else: continue
  i=n.start_byte
  while i>0 and src[i-1:i].isspace(): i-=1
  line=src.find(b'\n',i,n.start_byte); cuts.append((0 if i==0 else n.start_byte if line<0 else line+1,end))
 production,tests,at=[],[],0
 for start,end in sorted(cuts):
  if start<at: continue
  production.append(src[at:start]); tests.append(src[start:end]); at=end
 return SplitText(b''.join(production+[src[at:]]).decode(),b''.join(tests).decode())
USES=tree_sitter.Query(LANGUAGE,'(use_declaration argument: (_) @n)')
TYPE_ALIASES=tree_sitter.Query(LANGUAGE,'(type_item) @n')
def path_parts(node):
 """The segments of a path node, first to last: `a::b::C` -> [a, b, C]; `::a::b` -> [a, b]; `crate::x` -> [crate, x]."""
 return [node.text.decode()] if node.child_count==0 else [x for c in node.children if c.type!='::' for x in path_parts(c)]
@lru_cache(maxsize=64)
def uses(text):
 """(path, alias) of each path a `use` brings in, its first segment its root: `use a::{b::C as D, *}` -> ([a, b, C], D), ([a, *], None)."""
 tree,_=parsed(text); found=[]
 def walk(node,prefix):
  if node.type=='use_as_clause': found.append((prefix+path_parts(node.child_by_field_name('path')),node.child_by_field_name('alias').text.decode()))
  elif node.type=='use_wildcard': found.append((prefix+[x for c in node.named_children for x in path_parts(c)]+['*'],None))
  elif node.type=='scoped_use_list': walk(node.child_by_field_name('list'),prefix+[x for c in node.children if c.type!='use_list' and c.type!='::' for x in path_parts(c)])
  elif node.type=='use_list': [walk(c,prefix) for c in node.named_children]
  elif node.is_named: found.append((prefix+path_parts(node),None))
 for argument in captured(USES,tree.root_node): walk(argument,[])
 return found
def operation_variants(text,variants):
 """The distinct Operation variants a text names: through `Operation::`, an alias of Operation (`use ...Operation as X`, `type X = ...Operation;`), or a glob import of its variants."""
 tree,_=parsed(text)
 aliases={'Operation'}|{alias for path,alias in uses(text) if alias and path[-1:]==['Operation']}
 aliases|={n.child_by_field_name('name').text.decode() for n in captured(TYPE_ALIASES,tree.root_node) if n.child_by_field_name('type').type in ('type_identifier','scoped_type_identifier') and type_name(n.child_by_field_name('type'))=='Operation'}
 named=set(re.findall(r'\b(?:'+'|'.join(sorted(aliases))+r')::([A-Z]\w*)',text))
 if any(path[-2:]==[a,'*'] for path,_ in uses(text) for a in aliases): named|={v for v in variants if re.search(r'\b'+v+r'\b',text)}
 return named
def resolves_to_runtime(text,name):
 """A name a file uses is the runtime's unless every `use` path that brings it in (names it, or aliases it) starts at another crate or at this one (crate, self, super); a name no `use` brings in counts."""
 roots=[path[0] for path,alias in uses(text) if path and (name in path or alias==name)]
 return not roots or any(r in ('ryo_execution_runtime','ryo_execution') for r in roots)
REPLACED_LIST=p/'rewrite-replaced.toml'
# The two entry functions rev 8 keeps as the declared seam (7.3 L13): `begin` and `prepare`, by path and signature.
SEAM_ENTRIES=re.compile(r'request_execution::service::begin\b|prepared_execution::service::prepare\b')
def replaced_set(path=REPLACED_LIST):
 """The fused rewrite's replaced files (rewrite-replaced.toml, its `files` per crate), crates/ryo-relative."""
 import tomllib
 return {f for crate in tomllib.loads(path.read_text()).values() for f in crate['files']} if path.exists() else set()
def module_of(path):
 """(crate library name, module path) of a crate source file: crates/execution_domain/src/domain/x/types.rs -> (ryo_execution_domain, domain::x::types)."""
 parts=path.split('/'); crate='ryo_'+parts[1] if parts[0]=='crates' else 'ryo_'+parts[0]
 rest=[x.removesuffix('.rs') for x in (parts[3:] if parts[2:3]==['src'] else parts[2:])]
 return crate,'::'.join(x for x in rest if x not in ('mod','lib','main'))
def edit_footprint(prod_text,replaced):
 """The production files outside the runtime crate and outside the replaced set that name an item of a replaced module, by its
 crate path (`ryo_<crate>::<module>`, or `crate::<module>` inside that crate): the files the rewrite must edit (rev 8 section 7.3),
 the seam's two entry functions excepted."""
 modules=sorted({module_of(f) for f in replaced if f in prod_text},key=lambda x:len(x[1]))
 roots=[]
 for crate,module in modules:
  if not any(c==crate and (module==m or module.startswith(m+'::')) for c,m in roots): roots.append((crate,module))
 hits=[]
 for f,t in prod_text.items():
  if f in replaced or f.startswith(EVALUATOR): continue
  own=module_of(f)[0]; t=SEAM_ENTRIES.sub('',t)
  if any(re.search(r'\b'+re.escape(c)+'::'+re.escape(m)+r'\b',t) or (c==own and re.search(r'\bcrate::'+re.escape(m)+r'\b',t)) for c,m in roots): hits.append(f)
 return sorted(hits)
# Exit 7's ratchet (PM decision): the rows measured outside rewrite-replaced.toml never rise between checkpoints; the rows that count
# files inside it are informational, since the rewrite deletes those files.
RATCHETED=['rewrite_edit_footprint','cross_crate_tests_outside_the_replaced_set','operation_exhaustive_matches_outside_the_replaced_set','per_file_fact_touch_points']
INFORMATIONAL=['replace_the_evaluator','operation_exhaustive_matches','cross_crate_tests_naming_runtime_internals']
census_json=p/'census.json'
def census_rises(current,previous):
 """Each ratcheted census row whose file count is above the previous checkpoint's."""
 return ['census row '+k+' rose from '+str(previous[k]['files'])+' to '+str(current[k]['files'])+' files' for k in RATCHETED if k in previous and k in current and current[k]['files']>previous[k]['files']]
def census_problems(repo=None,base=None,measure=None,committed=None):
 """The tree's census, measured now, against the previous checkpoint's: census.json at the checkpoint base, or, where the base
 holds none (the first checkpoint with a census), the committed census.json this checkpoint recorded as its baseline. A
 ratcheted row may not rise. The committed file alone is never the subject: it is not re-measured, so comparing it would pass
 any rise made after it was written."""
 repo=repo or p; base=base or checkpoint_base(repo)
 old=git(repo,'show',base+':./census.json') if base else None
 previous=json.loads(old) if old else (committed if committed is not None else json.loads(census_json.read_text()) if census_json.exists() else None)
 if previous is None: return []
 current=measure() if measure else census(p.parents[3])
 return census_rises(current,previous)
LOCK=p.parents[3]/'.asymptote'/'operations'
# Sections of a lock operation that hold no accepted finding: its header, perf bests, and in-source ignores (their reason is
# in the source, which the zero baseline allows).
LOCK_KEPT=('version','parents','perf','suppressed')
def lock_problems(lock=LOCK,replaced=None):
 """The zero baseline, scoped by owner decision (option A): an accepted finding in Asymptote's lock (crates/ryo/.asymptote) is allowed only for
 a file of rewrite-replaced.toml; a record for any other file fails. Removals cancel the records they name."""
 import tomllib
 replaced=replaced_set() if replaced is None else replaced
 key=lambda r:(r.get('tool'),r.get('rule'),r.get('path'),r.get('fingerprint'))
 added={}; removed=set()
 for op in sorted(lock.glob('*.lock')) if lock.is_dir() else []:
  doc=tomllib.loads(op.read_text())
  for section,value in doc.items():
   if section in LOCK_KEPT or section.endswith('-removed') and section[:-8] in LOCK_KEPT or not isinstance(value,list): continue
   entries=[r for x in value if isinstance(x,dict) for r in (x.get('records') if isinstance(x.get('records'),list) else [x]) if isinstance(r,dict)]
   for r in entries:
    if section.endswith('-removed'): removed.add(key(r))
    elif 'path' in r: added[key(r)]=(op.name,r)
 return sorted('lock record '+str(r.get('tool'))+' '+str(r.get('rule'))+' at '+str(r.get('path'))+' ('+name+') is for a file outside rewrite-replaced.toml' for k,(name,r) in added.items() if k not in removed and r.get('path') not in replaced)
def census(product):
 """Change-amplification counts over the product's tracked Rust files, read statically: what a change of each kind must touch outside its owner. A #[cfg(test)] part of a production file counts as test."""
 files=(git(product,'ls-files','*.rs') or '').split()
 text={f:(Path(product)/f).read_text(errors='replace') for f in files}
 prod_text={f:split_tests(text[f]).production for f in files if census_production(f)}
 test_text={f:(split_tests(text[f]).tests if f in prod_text else text[f]) for f in files}
 test_text={f:t for f,t in test_text.items() if t}
 declared=re.compile(r'\bpub (?:struct|enum|trait|type) ([A-Z]\w*)')
 vocabulary={n for f,t in prod_text.items() if f.startswith(EVALUATOR) for n in declared.findall(t)}-{'Demand'}
 # A name resolves to the runtime's unless the file's own crate declares it: a declaration elsewhere hides nothing.
 local={}
 for f,t in prod_text.items(): local.setdefault(census_crate(f),set()).update(declared.findall(t))
 named=re.compile(r'\b(?:'+'|'.join(map(re.escape,sorted(vocabulary|SEAM_PORTS)))+r')\b')
 # A name after `Type::` is that type's variant or item, not the runtime's type.
 types_named=lambda t: {m.group(0) for m in named.finditer(t) if not re.search(r'[A-Z]\w*::$',t[max(0,m.start()-80):m.start()])}
 names_evaluator=lambda f,t: bool({n for n in types_named(t)-SEAM_PORTS-local.get(census_crate(f),set())-set(declared.findall(t)) if resolves_to_runtime(t,n)}) or 'ryo_execution_runtime' in t
 ev_prod=[f for f,t in prod_text.items() if not f.startswith(EVALUATOR) and names_evaluator(f,t)]
 ev_test=[f for f,t in test_text.items() if not f.startswith(EVALUATOR) and names_evaluator(f,t)]
 variants=set(re.findall(r'(?m)^\s{4}([A-Z]\w*)\b',''.join(re.findall(r'pub enum Operation\b[^{]*\{(.*?)\n\}',''.join(prod_text.values()),re.S))))
 exhaustive=[f for f,t in prod_text.items() if len(operation_variants(t,variants))>=8]
 fact=[f for f,t in prod_text.items() if PER_FILE_FACT.search(t)]
 internals=re.compile(r'\b(?:'+'|'.join(RUNTIME_INTERNALS)+r')\b')
 leaking=[f for f,t in test_text.items() if not f.startswith((EVALUATOR,STORE)) and internals.search(t)]
 replaced=replaced_set()
 count=lambda fs:{'files':len(fs),'crates':len({census_crate(f) for f in fs})}
 edits=edit_footprint(prod_text,replaced)
 return {'replace_the_evaluator':{'production':count(ev_prod),'test':count(ev_test),'total_files':len(set(ev_prod)|set(ev_test)),'vocabulary':len(vocabulary)},
  'operation_exhaustive_matches':count(exhaustive),'per_file_fact_touch_points':count(fact),'cross_crate_tests_naming_runtime_internals':count(leaking),
  'cross_crate_tests_outside_the_replaced_set':count([f for f in leaking if f not in replaced]),
  'operation_exhaustive_matches_outside_the_replaced_set':count([f for f in exhaustive if f not in replaced]),
  'rewrite_edit_footprint':dict(count(edits),list=edits,replaced_files=len(replaced)),
  'ratcheted':RATCHETED,'informational':INFORMATIONAL}
def placed(word,places):
 """One word of a listed command with each place (`{target}`, `{temp}`, `{worktree}`) replaced by its value on this machine."""
 for name,value in places.items(): word=word.replace(name,value)
 return word
# What each command checked, read from its tool's own summary line: cargo's `test result:` lines (tests run), the `Executable` lines of
# a `--no-run` build (test binaries built), and for the other tools the line named here. A count of 0 is a run that checked nothing,
# as `sdk_api`'s semver step was when it skipped all 260 of its checks; it is refused like a missing or stale result.
TEST_RESULT=re.compile(r'^test result: \w+\. (\d+) passed; (\d+) failed',re.M)
EXECUTABLE=re.compile(r'^\s+Executable ',re.M)
# The harness = false runners of a cargo test command say their own count: cucumber's scenario summary, the activation runner's cases.
RUNNER_COUNTS=(re.compile(r'^(\d+) scenarios? \(',re.M),re.compile(r'^activation: (\d+) actual cases passed',re.M))
COUNTED={
 'sdk_api':(re.compile(r'^\s+Checked \[[^\]]*\] (\d+) checks',re.M),'semver checks run'),
 'ruff_rule_phases':(re.compile(r'^native standalone guards passed: .*?(\d+) cases',re.M),'self-test cases run'),
 'ruff_parity':(re.compile(r'^ruff parity: \d+ identical of (\d+) rules',re.M),'rules compared'),
 'tm_guards':(re.compile(r'"proven":(\d+)'),'definitions proven'),
 'tm_validate_all':(re.compile(r'"proven":(\d+)'),'definitions proven'),
 'tm_validate_no_heap_accounting':(re.compile(r'"proven":(\d+)'),'definitions proven'),
 'harness_hooks':(re.compile(r'^Ran (\d+) tests?\b',re.M),'tests run'),
 'checkpoint_check':(re.compile(r'^Ran (\d+) tests?\b',re.M),'tests run'),
 'ownership_edges':(None,'edge rows answered'),
 'census':(re.compile(r'"replaced_files": (\d+)'),'replaced files measured'),
}
# The commands whose tool prints no count of what it checked: each is named in every report, never given an invented count.
NO_COUNT={
 'checkout_tools':'cargo build prints no count of what it built when its units are fresh; the binaries it leaves are what tm_source, tm_guards, tm_validate_all and ownership_edges run',
 'no_print':'cargo clippy prints no count of the crates or files it checked when its units are fresh',
 'tm_source':"tm check's JSON names its findings and suppressions, not the files it read",
 'tm_cli_source':"tm check's JSON names its findings and suppressions, not the files it read",
 'conformance':'the language conformance runner prints only the relations a language does not yet read, no count of the rows it proved',
}
def checked(name,cmd,log):
 """What one command checked, {'count': n, 'what': ...}, from its tool's own summary line in its raw log; None when its tool prints none."""
 text=Path(log).read_text(errors='replace') if Path(log).is_file() else ''
 if cmd[:1]==['cargo'] and '--no-run' in cmd: return {'count':len(EXECUTABLE.findall(text)),'what':'test binaries built'}
 if cmd[:2]==['cargo','test']: return {'count':sum(int(a)+int(b) for a,b in TEST_RESULT.findall(text))+sum(int(x) for pattern in RUNNER_COUNTS for x in pattern.findall(text)),'what':'tests run'}
 if name=='ownership_edges':
  rows=[json.loads(x).get('rows',[]) for x in text.splitlines() if x.startswith('{"columns"')]
  return {'count':sum(len(r) for r in rows),'what':COUNTED[name][1]}
 if name in COUNTED:
  pattern,what=COUNTED[name]
  return {'count':sum(int(x) for x in pattern.findall(text)),'what':what}
 return None
def result_problems(v):
 """Each command of commands.json whose result is missing, is for another source than the validated one, or checked nothing (a count
 of 0, or no count from a tool that prints one); a command refused as `not available` is reported as not run instead, and a command
 whose tool prints no count is named (NO COUNT), never failed for it."""
 bad=[]
 for name,_ in json.loads(commands_json.read_text()):
  c=v.get('checks',{}).get(name)
  if c is None: bad.append(name+': no result for the validated source'); continue
  if c.get('source')!=v.get('source'): bad.append(name+': its result is for '+str(c.get('source'))[:10]+', not the validated source '+str(v.get('source'))[:10]); continue
  if c.get('not_available'): continue
  if name in NO_COUNT: print('NO COUNT '+name+': '+NO_COUNT[name]); continue
  count=(c.get('checked') or {}).get('count')
  if not count: bad.append(name+': checked nothing ('+('no count in its log' if count is None else '0 '+c['checked']['what'])+')')
 return bad
if __name__=='__main__' and sys.argv[1:]==['--census']: print(json.dumps(census(p.parents[3]),indent=1)); sys.exit(0)
if __name__=='__main__' and sys.argv[1:]==['--check']: sys.exit(finish(dict(json.loads(validation_json.read_text()),tests=loaded(json.loads(tests_json.read_text())) if tests_json.exists() else {}),check=True))
def validate():
 """Runs every command of commands.json at a frozen head, records its results and tests, and checks the matrix."""
 # Every machine path a command needs is derived from where the tree is checked out, never written in commands.json: `{target}`
 # is the tree's own target directory, idiomatic-target/<tree name> beside the checkout (cargo judges freshness by source mtimes, so a
 # target shared across worktrees served validation327 units another tree built), `{temp}` the system temporary directory, and the
 # build locks sit beside the checkout, where every worktree of the mount takes them.
 top=Path(subprocess.check_output(['git','rev-parse','--show-toplevel'],text=True).strip()); temp=Path(tempfile.gettempdir())
 places={'{target}':str(top.parent/'idiomatic-target'/top.name),'{temp}':str(temp),'{worktree}':top.name}
 locks=[open(top.parent/'.cargo-build.lock','a'),open(temp/'ryo-cargo-serial.lock','a')]
 def locked(run):
  """Runs one command under the build locks, so other agents' short runs interleave between this batch's commands."""
  for lock in locks: fcntl.flock(lock,fcntl.LOCK_EX)
  try: return run()
  finally:
   for lock in locks: fcntl.flock(lock,fcntl.LOCK_UN)
 assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip(), 'freeze source before validation'
 head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
 def finding_split(cmd,log):
  # `tm check` prints its findings as one JSON document; the split is per scoped directory, test trees apart from production.
  scoped=cmd[cmd.index('check')+1:cmd.index('--format')]
  found=json.loads(log.read_text())
  by_directory={x:{'production':0,'test':0} for x in scoped}
  for finding in found['findings']:
   owner=[x for x in scoped if finding['path'].startswith('crates/ryo/'+x+'/') or finding['path']=='crates/ryo/'+x]
   by_directory[owner[0]]['test' if is_test_file(finding['path']) else 'production']+=1
  return {'total':len(found['findings']),'suppressed':len(found['suppressed']),'production':sum(v['production'] for v in by_directory.values()),'test':sum(v['test'] for v in by_directory.values()),'by_directory':by_directory}
 # What is committed names the commands as commands.json lists them: an expanded place is a machine path, and a worktree named
 # for its checkpoint reads as tracker text in the committed identity and checks.
 listed=dict(json.loads((p/'commands.json').read_text()))
 commands=[[n,[placed(w,places) for w in c]] for n,c in listed.items()]
 (p/'identity.json').write_text(dumps({'source':head,'commands':[[n,c] for n,c in listed.items()]})+'\n')
 # Raw outputs (.log/.json per command) never live in the repository: they go to the directory given as the first argument,
 # by default a directory named for the source head and the date under the system temporary directory.
 out_dir=Path(sys.argv[1]) if len(sys.argv)>1 else temp/('ryo-contract-validation-'+head[:10]+'-'+date.today().strftime('%Y%m%d'))
 out_dir.mkdir(parents=True,exist_ok=True)
 for name,cmd in commands:
  print('START '+name,flush=True)
  started=time.time()
  with (out_dir/(name+'.log')).open('w') as out: r=locked(lambda: subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT))
  result={'source':head,'command':listed[name],'exit':r.returncode,'elapsed_seconds':time.time()-started}
  if r.returncode and (reason:=unavailable(out_dir/(name+'.log'))): result['not_available']=reason
  if name=='tm_source': result['findings']=finding_split(cmd,out_dir/(name+'.log'))
  if (counted:=checked(name,cmd,out_dir/(name+'.log'))) is not None: result['checked']=counted
  (out_dir/(name+'.json')).write_text(json.dumps(result,indent=2))
  print('END '+name+' exit='+str(r.returncode),flush=True)
  if name=='build' and r.returncode: break
 print('BATCH COMPLETE',flush=True)
 v=json.loads(validation_json.read_text())
 v.update({'source':head,'validation':v['validation']+1,'raw_identity':raw_identity(out_dir)})
 record(v,commands,out_dir)
 record_tests(v,commands,out_dir)
 validation_json.write_text(dumps({k:x for k,x in v.items() if k!='tests'})+'\n')
 sys.exit(finish(v))
if __name__=='__main__': validate()
