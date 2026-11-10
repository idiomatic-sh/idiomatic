# ignore: idiomatic.arch/file-length - the checkpoint evidence directory holds one validation script, run.py, beside its fixture cases in run_test.py; the census and the checkpoint diff checks are modes of it over the same validation.json, commits and raw logs, and a second script would be a file role the directory does not have.
import json,os,re,subprocess,sys,time,fcntl,tomllib
from datetime import date
from pathlib import Path
p=Path(__file__).resolve().parent
contracts=p.parents[1]
readme=contracts/'README.md'
validation_json=p/'validation.json'
tests_json=p/'tests.json'
TM_TARGETS=('tm_source','tm_cli_source')
def dumps(v,pad=''):
 """JSON with two-space indent where a list of scalars or a short dict of scalars (four keys, 300 characters) stays on one line, so a command line or a red-then-green entry is one line."""
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
 rows=[('Source','`'+v['source'][:10]+'`'),('Validation','validation'+str(v['validation'])+' (raw logs in the directory named `'+Path(v['raw_identity']).name+'`, kept outside the repository)')]
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
def test_results(log):
 """Per test binary of one cargo test log: its passed, ignored and not-run tests, and each failed test with the case rows its runner names; `collected` when the runner ran every row and named every red one. A binary cargo failed that printed no `test result:` line, or no failed test, crashed: its failure is CRASHED. A test that prints before its status has the status on a line of its own; one that prints `not available` there measured nothing, so its `ok` is recorded as not run, never as passed."""
 out={}; binary=None; failing=None; pending=[]; summarized=set()
 for line in Path(log).read_text(errors='replace').splitlines():
  if binary and SUMMARY.match(line): summarized.add(binary)
  if binary and CARGO_FAILED.match(line):
   if binary not in summarized or not out[binary]['failed']: out[binary]['failed'].setdefault(CRASHED,{'rows':[],'collected':False})
   continue
  m=RAN.match(line)
  if m: binary=m.group(2) or m.group(3); out.setdefault(binary,{'passed':[],'ignored':[],'not_run':[],'failed':{}}); failing=None; continue
  m=TEST.match(line) or (pending and re.match(r'^()(ok|FAILED|ignored)$',line))
  if not m and binary and line.startswith('test ') and ' ... ' in line: pending.append((line[5:line.index(' ... ')],bool(NOT_AVAILABLE.match(line[line.index(' ... ')+5:])))); continue
  if m and binary:
   name,status=m.groups(); unavailable=False
   if not name: name,unavailable=pending.pop(0)
   if status=='FAILED': out[binary]['failed'].setdefault(name,{'rows':[],'collected':False})
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
 v['tests']={n:test_results(out_dir/(n+'.log')) for n,c in commands if c[:2]==['cargo','test'] and (out_dir/(n+'.log')).exists()}
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
  if not logs or not all((Path(v['raw_identity'])/x).is_file() for x in logs): bad.append(where+': cites a rerun with no raw log in '+v['raw_identity']+': '+text[:110])
 return bad
RUST_REGRESSIONS='Measure full-Django cold/warm/edit and Rust regressions; meet final objective.'
CLOSE='Close full TM, contracts, independent review and normal CI.'
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
RED_OWNERS={
 ('libraries','ryo_benchmark','inbound::use_cases::compare_corpus::use_case_test::refuses'):CLOSE,
 ('libraries','ryo_benchmark','inbound::use_cases::compare_corpus::use_case_test::succeeds'):CLOSE,
 ('libraries','ryo_benchmark','inbound::use_cases::verify_cases::use_case_test::process_capture::test::succeeds'):CLOSE,
 ('cli_tests','replay','behavior::ruff_findings_equal_the_expected_findings'):CLOSE,
 ('cli_tests','replay','behavior::every_recorded_rule_gives_the_recorded_rows'):CLOSE,
 ('cli_tests','replay','peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound'):OWNERS['peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound'],
 ('cli_cost','replay','cost::a_cold_run_over_ruff_holds_the_peak_ratchet'):OWNERS['cli_cost'],
 ('cli_cost','replay','cost::a_warm_run_and_an_edit_run_over_ruff_hold_their_ratchets'):OWNERS['cli_cost'],
 ('cli_cost','replay','cost::every_live_rule_together_holds_the_cold_ratchet_of_ruff'):OWNERS['cli_cost'],
 ('core_allocation','allocation','per_file_path::allocates_as_the_parser_library_does'):OWNERS['core_allocation'],
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
def named_tests(message): return [t for t in NAMED_TEST.findall(message) if not t.startswith(('crates::','std::','self::','super::'))]
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
 bad=[]; raw=Path(v.get('raw_identity') or '/nonexistent'); ranges={}; proven=set(); unproven={}; merges={}
 before=['^'+baseline] if baseline and git(repo,'cat-file','-e',baseline+'^{commit}') is not None else []
 for sha in (git(repo,'rev-list',base+'..'+head,*before) or '').split():
  message=git(repo,'show','-s','--format=%B',sha) or ''
  # A merge's own changes are the files its result changes against every parent (the combined diff).
  own=['diff-tree','--cc','--no-commit-id','--name-only',sha] if len((git(repo,'rev-list','--parents','-n1',sha) or '').split())>2 else ['diff-tree','--root','--no-commit-id','-r','--name-only',sha]
  files=(git(repo,*own) or '').split()
  if not any(is_production(f) and not comment_only(repo,sha,f,own) and not test_only(repo,sha,f,own) for f in files): continue
  short=sha[:10]+' '+message.splitlines()[0][:70]
  if PERF.match(message):
   if not perf_identical(repo,base,head,sha): bad.append('perf '+short+': no evidence/perf/*/after.json added or changed in the checkpoint with identical_output true and a source built on it')
   continue
  tests=named_tests(message)
  entries=[x for x in v.get('red_green',[]) if sha in covered(repo,x,ranges)]
  tables=[t for x in entries for t in [table_of(x,raw)] if t is not None]
  # A range proven by a validation command (gate:<command>, red then green): every commit the entry covers is proven by it.
  if any(gate_proves(x,raw) for x in entries): proven.add(sha); continue
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
RUST_COMMENT=re.compile(r'^\s*(//.*)?$')
# A lint attribute, inner or outer, over one or more lines: `#[expect(clippy::x, reason = "...")]`, `#![deny(y)]`.
LINT_ATTRIBUTE=re.compile(r'#!?\[\s*(?:expect|allow|warn|deny|forbid)\s*\((?:[^()"]|"(?:[^"\\]|\\.)*"|\((?:[^()"]|"(?:[^"\\]|\\.)*")*\))*\)\s*\]')
def comment_only(repo,sha,path,own):
 """A Rust file whose change in the commit adds and removes only comment lines (`//`, `///`, `//!`), blank lines and lint
 attributes (`#[expect(..)]`, `#[allow(..)]`, `#![deny(..)]`, over any number of lines): lint configuration and comments change no
 behaviour, so FIX does not count it. A Cargo.toml lints table is no production file to begin with."""
 if not path.endswith('.rs'): return False
 diff=git(repo,*([x for x in own if x!='--name-only']+['-p','--',':(top)'+path])) or ''
 width=2 if own[1]=='--cc' else 1
 sides={'+':[],'-':[]}
 for l in diff.splitlines():
  if l.startswith(('+++','---')) or len(l)<width: continue
  mark=l[:width]
  if mark.strip()=='' : continue
  if mark.strip('+- ')!='': continue
  sides['+' if '+' in mark else '-'].append(l[width:])
 if not sides['+'] and not sides['-']: return False
 for lines in sides.values():
  text='\n'.join(x for x in lines if not RUST_COMMENT.match(x) or x.strip()=='')
  if LINT_ATTRIBUTE.sub('',text).strip(): return False
 return True
def test_only(repo,sha,path,own):
 """A Rust file whose production text (split_tests: without its #[cfg(test)] items, the census's one definition) is the same before
 and after the commit, comment and lint-attribute lines aside: the commit changed only its test code. A merge is judged by its
 combined diff instead (comment_only)."""
 if not path.endswith('.rs') or own[1]=='--cc': return False
 before=git(repo,'show',sha+'^:'+path); after=git(repo,'show',sha+':'+path)
 if before is None or after is None: return False
 plain=lambda text:[x for x in LINT_ATTRIBUTE.sub('',split_tests(text)[0]).splitlines() if not RUST_COMMENT.match(x)]
 return plain(before)==plain(after)
def patch_id(repo,sha):
 """The commit's `git patch-id --stable`, or None for a commit with no diff against its first parent."""
 diff=subprocess.run(['git','-C',str(repo),'show','--format=','--no-color',sha],capture_output=True)
 found=subprocess.run(['git','-C',str(repo),'patch-id','--stable'],input=diff.stdout,capture_output=True).stdout.decode().split()
 return found[0] if found else None
# The items the Rust API guidelines give an example (C-EXAMPLE); a `Sealed` supertrait is the seal, not API.
ITEM=re.compile(r'^(\s*)pub\s+(?:const\s+|async\s+|unsafe\s+)*(fn|struct|enum|trait|type|union)\s+([A-Za-z_]\w*)')
HEADER=re.compile(r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(impl|mod|trait)\b(.*)')
def owner(lines,at,indent):
 """The impl type, module or trait that holds an indented item: the nearest header line above it with less indent, else None."""
 for i in range(at-1,-1,-1):
  m=HEADER.match(lines[i])
  if m and len(m.group(1))<indent:
   rest=re.sub(r'<[^<>]*(?:<[^<>]*>[^<>]*)*>','',m.group(3)); rest=rest.split(' for ',1)[1] if ' for ' in rest else rest
   name=re.findall(r'[A-Za-z_]\w*',rest.split('{')[0].split(' where ')[0])
   return m.group(2)+' '+(name[-1] if name else '')
  if lines[i].strip() and not lines[i].startswith((' ','\t')) and not lines[i].lstrip().startswith(('//','#','}')): return None
 return None
def items(text):
 """(owner, kind, name, line index) of each pub item in a Rust source."""
 lines=text.splitlines(); found=[]
 for n,line in enumerate(lines):
  m=ITEM.match(line)
  if m: found.append((owner(lines,n,len(m.group(1))) if m.group(1) else None,m.group(2),m.group(3),n))
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
def doc_above(lines,at):
 doc=[]; i=at-1
 while i>=0 and (lines[i].strip().startswith(('///','#[','#!['))): doc.append(lines[i]); i-=1
 return '\n'.join(doc)
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
 lines=[l.strip().removeprefix('///').strip() for l in reversed(doc.splitlines())]
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
  lines=(git(repo,'show',head+':'+path) or '').splitlines()
  for item in items('\n'.join(lines)):
   if (module_path(crate,path),)+item[:3] in olds[crate] or item[1:3]==('trait','Sealed'): continue
   n=item[3]; doc=doc_above(lines,n); where=path+':'+str(n+1)+' pub '+item[1]+' '+item[2]
   if not has_example(doc): bad.append(where+': no `# Examples` section with a code block')
   signature=' '.join(lines[n:n+12]).split('{')[0].split(';')[0]
   if item[1]=='fn' and re.search(r'->\s*(?:[\w:]+::)?Result\b',signature) and '# Errors' not in doc: bad.append(where+': returns a Result with no `# Errors` section')
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
  print('RED '+name+' exit '+str(code)+(', owned by '+'; '.join('"'+x+'"' for x in owners) if owners else ', owned by no row'))
  if not owners: unowned.append(name)
 for line in diff: print(line)
 if changed: print(('STALE RECORD: the committed record is not what validation.json gives: ' if check else 'rewritten from validation.json (numbers block, owner notes, un-ticked rows): ')+', '.join(changed)+'; stage them')
 return 1 if bad or diff or unowned or unticked_any or (check and changed) else 0
EVALUATOR='crates/execution_runtime/'
STORE='crates/execution_store/'
SEAM_PORTS={'DigestPort','DigestBytes','FactsPort','OperatorsPort','FactsError'}
RUNTIME_INTERNALS=('FileRunInput','FileSession','FileInput','RanRule','RuleFile','SourceRows','FileRules','FileFinished','EmittedRoot','KindSupport','SemanticSupport','CaptureContext','RuleSelection','AdmittedInput','InputAdmissions','RelationOwner','InvocationWorkspace')
PER_FILE_FACT=re.compile(r'binding_read|binding-read|BindingRead')
def census_production(path):
 """A production source of the product: under crates/*/src, cli/src or languages/*/src, and no test file (is_test_file)."""
 return bool(re.match(r'(crates/[^/]+/src|cli/src|languages/[^/]+/src)/',path)) and not is_test_file(path)
def census_crate(path): return path.split('/')[0] if path.startswith('cli/') else '/'.join(path.split('/')[:2])
CFG_TEST=re.compile(r'^\s*#\[cfg\(test\)\]',re.M)
CHAR=re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'])'")
RAW=re.compile(r'r(#*)"')
def literal_end(text,i):
 """The index after the comment, string, raw string or char literal starting at i, or None when none starts there (a lifetime is no literal)."""
 if text.startswith('//',i): j=text.find('\n',i); return len(text) if j<0 else j
 if text.startswith('/*',i): j=text.find('*/',i+2); return len(text) if j<0 else j+2
 m=RAW.match(text,i)
 if m and (i==0 or not (text[i-1].isalnum() or text[i-1]=='_')): j=text.find('"'+m.group(1),m.end()); return len(text) if j<0 else j+1+len(m.group(1))
 if text[i]=='"':
  j=i+1
  while j<len(text) and text[j]!='"': j+=2 if text[j]=='\\' else 1
  return j+1
 m=CHAR.match(text,i) if text[i]=="'" else None
 return m.end() if m else None
def item_end(text,start):
 """The end of the item that starts at start: its body's closing brace, matched; its first `;`, or, for a struct field or enum variant, its `,` or the enclosing `}`. Angle brackets count only in the header, before the body or a top-level `=` (an initializer or discriminant); comments, strings and char literals are skipped."""
 depth=0; header=True; i=start
 while i<len(text):
  skip=literal_end(text,i) if text[i] in '/"\'r' else None
  if skip is not None: i=skip; continue
  c=text[i]
  if depth==0 and c in ';,': return i+1
  if header and depth==0 and c=='=' and text[i+1:i+2] not in ('=','>') and text[i-1:i] not in ('=','!','<','>'): header=False
  if c=='{': depth+=1; header=False
  elif c in '([' or header and c=='<' and text[i+1:i+2]!='=': depth+=1
  elif c in ')]' or header and c=='>' and text[i-1:i] not in ('-','='): depth-=1
  elif c=='}':
   if depth==0: return i
   depth-=1
   if depth==0: return i+1
  i+=1
 return len(text)
def split_tests(text):
 """A production file's text without its #[cfg(test)] items, and the text of those items: each item's own extent."""
 production,tests,at=[],[],0
 for m in CFG_TEST.finditer(text):
  if m.start()<at: continue
  end=item_end(text,m.end()); production.append(text[at:m.start()]); tests.append(text[m.start():end]); at=end
 return ''.join(production)+text[at:],''.join(tests)
def operation_variants(text,variants):
 """The distinct Operation variants a text names: through `Operation::`, an alias of Operation (`use ...Operation as X`, `type X = ...Operation;`), or a glob import of its variants."""
 aliases={'Operation'}|set(re.findall(r'\bOperation\s+as\s+([A-Za-z_]\w*)',text))|set(re.findall(r'\btype\s+([A-Za-z_]\w*)\s*=\s*(?:[\w:]*::)?Operation\s*;',text))
 named=set(re.findall(r'\b(?:'+'|'.join(sorted(aliases))+r')::([A-Z]\w*)',text))
 if re.search(r'\b(?:'+'|'.join(sorted(aliases))+r')::\*|\bOperation::\{[^}]*\*',text): named|={v for v in variants if re.search(r'\b'+v+r'\b',text)}
 return named
USE=re.compile(r'\buse\s+(?:::)?([A-Za-z_]\w*)[^;]*;',re.S)
def resolves_to_runtime(text,name):
 """A name a file uses is the runtime's unless every `use` that brings it in starts at another crate or at this one (crate, self, super); a name no `use` brings in counts."""
 roots=[m.group(1) for m in USE.finditer(text) if re.search(r'\b'+name+r'\b',m.group(0))]
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
def census_problems(repo=None,base=None):
 """The committed census.json against the previous checkpoint's (census.json at the checkpoint base): a ratcheted row may not rise."""
 repo=repo or p; base=base or checkpoint_base(repo)
 if not census_json.exists() or not base: return []
 old=git(repo,'show',base+':./census.json')
 return census_rises(json.loads(census_json.read_text()),json.loads(old)) if old else []
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
 prod_text={f:split_tests(text[f])[0] for f in files if census_production(f)}
 test_text={f:(split_tests(text[f])[1] if f in prod_text else text[f]) for f in files}
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
if __name__=='__main__' and sys.argv[1:]==['--census']: print(json.dumps(census(p.parents[3]),indent=1)); sys.exit(0)
if __name__=='__main__' and sys.argv[1:]==['--check']: sys.exit(finish(dict(json.loads(validation_json.read_text()),tests=loaded(json.loads(tests_json.read_text())) if tests_json.exists() else {}),check=True))
def validate():
 """Runs every command of commands.json at a frozen head, records its results and tests, and checks the matrix."""
 locks=[open('/workspaces/.cargo-build.lock','a'),open('/tmp/ryo-cargo-serial.lock','a')]
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
 # Each worktree builds into its own target directory, idiomatic-target/<worktree name> in the workspace mount: cargo judges
 # freshness by source mtimes, so a target shared across worktrees served validation327 units another tree built.
 tree=Path(subprocess.check_output(['git','rev-parse','--show-toplevel'],text=True).strip()).name
 # What is committed names the commands as commands.json lists them: the expanded worktree name is a machine path, and a
 # worktree named for its checkpoint reads as tracker text in the committed identity and checks.
 listed=dict(json.loads((p/'commands.json').read_text()))
 commands=[[n,[w.replace('{worktree}',tree) for w in c]] for n,c in listed.items()]
 (p/'identity.json').write_text(dumps({'source':head,'commands':[[n,c] for n,c in listed.items()]})+'\n')
 # Raw outputs (.log/.json per command) never live in the repository: they go to the directory given as the first argument,
 # by default a directory named for the source head and the date under the system temporary directory.
 out_dir=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/tmp/ryo-contract-validation-'+head[:10]+'-'+date.today().strftime('%Y%m%d'))
 out_dir.mkdir(parents=True,exist_ok=True)
 for name,cmd in commands:
  print('START '+name,flush=True)
  started=time.time()
  with (out_dir/(name+'.log')).open('w') as out: r=locked(lambda: subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT))
  result={'source':head,'command':listed[name],'exit':r.returncode,'elapsed_seconds':time.time()-started}
  if r.returncode and (reason:=unavailable(out_dir/(name+'.log'))): result['not_available']=reason
  if name=='tm_source': result['findings']=finding_split(cmd,out_dir/(name+'.log'))
  (out_dir/(name+'.json')).write_text(json.dumps(result,indent=2))
  print('END '+name+' exit='+str(r.returncode),flush=True)
  if name=='build' and r.returncode: break
 print('BATCH COMPLETE',flush=True)
 v=json.loads(validation_json.read_text())
 v.update({'source':head,'validation':v['validation']+1,'raw_identity':str(out_dir)})
 record(v,commands,out_dir)
 record_tests(v,commands,out_dir)
 validation_json.write_text(dumps({k:x for k,x in v.items() if k!='tests'})+'\n')
 sys.exit(finish(v))
if __name__=='__main__': validate()
