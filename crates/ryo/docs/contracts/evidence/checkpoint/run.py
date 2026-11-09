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
def test_results(log):
 """Per test binary of one cargo test log: its passed, ignored and not-run tests, and each failed test with the case rows its runner names; `collected` when the runner ran every row and named every red one. A test that prints before its status has the status on a line of its own; one that prints `not available` there measured nothing, so its `ok` is recorded as not run, never as passed."""
 out={}; binary=None; failing=None; pending=[]
 for line in Path(log).read_text(errors='replace').splitlines():
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
REPAIR='Repair every deferred output failure; rerun all 413 and full corpus.'
RUST_REGRESSIONS='Measure full-Django cold/warm/edit and Rust regressions; meet final objective.'
CLOSE='Close full TM, contracts, independent review and normal CI.'
OWNERS={
 'core_library':REPAIR,
 'benchmark_library':CLOSE,
 'core_allocation':RUST_REGRESSIONS,
 'cli_cost':'Acceptance: 413/413 on the current engine, full Django (2,875 files, all 413 rules) with unknown 0 and unread 0, Rust regression guards within bounds.',
 'behavior::ruff_rules_feed_every_catalog_feature_a_thousand_rows':'Consolidate the kernels: run the all-413 parity run against Ruff with and without each of the five join/select kernels and delete every kernel the evidence shows superseded.',
 'behavior::ruff_findings_equal_the_expected_findings':CLOSE,
 'behavior::every_recorded_rule_gives_the_recorded_rows':CLOSE,
 'cache::the_store_every_recorded_rule_keeps_over_ruff_stays_within_its_bound':REPAIR,
 'peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound':'Decode lazy packed frontiers once per population, not once per root, and compile ordered existence without the eager request-level pass.',
 'tm_import_cli':CLOSE,
 'tastemaker_workspace':CLOSE,
 'cache::a_store_is_not_used_for_other_rules_and_is_kept_apart_by_root':CLOSE,
 'behavior::the_files_that_say_why_something_differs_name_only_recorded_rules_and_real_features':CLOSE,
 'behavior::recorded_recognition_gives_the_recorded_unread_files':CLOSE,
 'cache::a_warm_run_leaves_the_store_as_it_was_and_a_removed_file_does_not':CLOSE,
 'cache::a_run_whose_store_keeps_nothing_answers_what_one_that_keeps_does':CLOSE,
 'behavior::what_ryo_could_not_decide_over_ruff_is_the_golden_or_a_recorded_difference':CLOSE,
 'behavior::row_that_differs_is_reported_with_its_plan_id':CLOSE,
 'cache::a_warm_run_reads_nothing_and_an_edit_reads_what_it_must':CLOSE,
 'cache::an_edit_hands_the_store_the_words_of_the_file_read_alone':CLOSE,
 'damage::a_store_cut_short_damaged_or_removed_is_empty_and_the_run_answers_as_with_none':CLOSE,
 'cache::files_added_and_removed_before_others_move_what_the_others_handed_with_them':CLOSE,
 'damage::a_store_whose_table_of_contents_gives_a_section_longer_than_the_file_is_no_store':CLOSE,
 'damage::a_store_with_one_section_damaged_gives_the_findings_of_a_run_that_keeps_nothing':CLOSE,
 'finish::an_edit_that_leaves_what_a_file_shows_the_others_as_it_was_keeps_the_cross_file_findings':CLOSE,
 'frontier::no_rule_is_handed_words_in_proportion_to_the_elements_of_a_file':CLOSE,
 'frontier::the_finish_is_handed_a_row_for_each_function_and_none_for_a_token':CLOSE,
 'lowering::every_rule_gives_the_same_rows_on_two_runs_over_ruff':CLOSE,
 'manifests::a_package_the_scope_does_not_list_is_found_by_the_recorded_workspace_completeness_query':CLOSE,
 'manifests::a_member_the_listing_does_not_hold_is_found_as_a_file_after_the_listed_ones':CLOSE,
 'manifests_bun::ryo_says_what_bun_says_of_the_manifests_of_a_workspace':CLOSE,
 'interface::an_edit_inside_a_body_reads_one_file_and_an_edit_of_what_a_file_declares_or_imports_makes_the_cross_file_steps_again':CLOSE,
 'recordings::a_pending_txt_naming_exactly_the_refused_relation_waits':CLOSE,
 'manifests_go::ryo_says_what_go_says_of_the_manifests_of_a_workspace':CLOSE,
 'manifests_uv::ryo_says_what_uv_says_of_the_manifests_of_a_workspace':CLOSE,
 'manifests_pnpm::ryo_says_what_pnpm_says_of_the_manifests_of_a_workspace':CLOSE,
 'manifests_npm::ryo_says_what_npm_says_of_the_manifests_of_a_workspace':CLOSE,
 'manifests_cargo::ryo_says_what_cargo_says_of_the_manifests_of_every_target':CLOSE,
}
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
 for key,title in OWNERS.items():
  if '[ ] '+title not in readme_text and '[x] '+title not in readme_text: bad.append('owner of '+key+' is no README row: '+title[:80])
 for key,x in results.items():
  if not isinstance(x,dict) or key in LISTED: continue
  if red(x):
   if key in OWNERS: x['note']=owned(x.get('note',''),['owned by "'+OWNERS[key]+'"'])
   else: bad.append('red '+key+' has no owner in OWNERS')
  elif 'note' in x: x['note']=owned(x['note'],[])
 reds=results.get('cli_replay_reds',[])
 if reds: results['cli_replay_note']=owned(results.get('cli_replay_note',''),[r.split('::')[0]+' owned by "'+OWNERS[r]+'"' for r in reds if r in OWNERS])
 bad+=['red '+r+' has no owner in OWNERS' for r in reds if r not in OWNERS]
 return json.dumps(results)!=before,bad
def write_block(text,v):
 """Replace the block from the `### Current ownership checkpoint` heading to the first following bullet."""
 lines=text.splitlines(keepends=True)
 start=next(i for i,l in enumerate(lines) if l.startswith('### Current ownership checkpoint'))
 end=next(i for i in range(start+1,len(lines)) if lines[i].startswith('- ['))
 return ''.join(lines[:start])+render(v)+''.join(lines[end:])
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
NAMED_TEST=re.compile(r'\b(?:[a-z_][a-z0-9_]*::)+[a-z_][a-z0-9_]*\b|\btest_\w+|\b[a-z][a-z0-9]*(?:_[a-z0-9]+){3,}\b')
def named_tests(message): return [t for t in NAMED_TEST.findall(message) if not t.startswith(('crates::','std::','self::','super::'))]
CARGO_LINE=re.compile(r'^test (\S+) \.\.\. (ok|FAILED)\b')
NEXTEST_LINE=re.compile(r'^\s*(PASS|FAIL|SIGSEGV|SIGABRT|TIMEOUT)\s+\[[^\]]*\]\s+\S+\s+(\S+)')
UNITTEST_LINE=re.compile(r'^(\w+) \(([\w.]+)\)\S*.* \.\.\. (ok|FAIL|ERROR)\b')
UNITTEST_FAILURE=re.compile(r'^(FAIL|ERROR): (\w+) \(([\w.]+)\)')
# The runner's own frame around its results: a log without it was not written by the runner. A forger can still type
# these lines; the check catches a hand-written result, not a deliberate fake.
RUNNER_FRAMES=((re.compile(r'(?m)^running \d+ tests?$'),re.compile(r'(?m)^test result: (ok|FAILED)\. \d+ passed')),
               (re.compile(r'(?m)^\s*Starting \d+ tests? across'),re.compile(r'(?m)^\s*Summary \[')),
               (re.compile(r'(?m)^Ran \d+ tests? in '),re.compile(r'(?m)^(OK|FAILED \()')))
def runner_framed(text): return any(head.search(text) and tail.search(text) for head,tail in RUNNER_FRAMES)
def log_status(log,test):
 """'failed' when the log shows the test failing, 'passed' when it shows it passing and never failing, else None: cargo test, cargo nextest and unittest -v lines inside the runner's own frame, a test named by its path or a suffix of it."""
 named=lambda path,sep: path==test or path.endswith(sep+test)
 failed=passed=False; text=Path(log).read_text(errors='replace')
 if not runner_framed(text): return None
 for line in text.splitlines():
  m=CARGO_LINE.match(line)
  if m and named(m.group(1),'::'): failed|=m.group(2)=='FAILED'; passed|=m.group(2)=='ok'; continue
  m=NEXTEST_LINE.match(line)
  if m and named(m.group(2),'::'): failed|=m.group(1)!='PASS'; passed|=m.group(1)=='PASS'; continue
  m=UNITTEST_LINE.match(line)
  if m and (named(m.group(1),'.') or named(m.group(2),'.')): failed|=m.group(3)!='ok'; passed|=m.group(3)=='ok'; continue
  m=UNITTEST_FAILURE.match(line)
  if m and (named(m.group(2),'.') or named(m.group(3),'.')): failed=True
 return 'failed' if failed else 'passed' if passed else None
def covered(repo,entry,ranges):
 """The commits the red_green entry's range holds: in its merge sha and not in its base sha (read once per range)."""
 key=(entry.get('base_sha',''),entry.get('merge_sha','')) if isinstance(entry,dict) else ('','')
 if key not in ranges: ranges[key]=set((git(repo,'rev-list',key[0]+'..'+key[1]) or '').split()) if all(key) else set()
 return ranges[key]
def proves(entry,test,raw):
 """The entry names the test and two distinct logs, `red <name>.log` showing it fail and `green <name>.log` showing it pass, both in the validation directory."""
 text=entry.get('test','')
 if not re.search(r'(?<![\w:])'+re.escape(test)+r'(?![\w])',text): return False
 red,green=re.search(r'\bred ([\w.-]+\.log)\b',text),re.search(r'\bgreen ([\w.-]+\.log)\b',text)
 if not red or not green or red.group(1)==green.group(1) or not (raw/red.group(1)).is_file() or not (raw/green.group(1)).is_file(): return False
 return log_status(raw/red.group(1),test)=='failed' and log_status(raw/green.group(1),test)=='passed'
def perf_identical(repo,base,head,sha):
 """A perf commit's proof: an after.json under evidence/perf/ added or changed in the checkpoint, with identical_output true and a source that is the perf commit or a commit built on it."""
 for path in (git(repo,'diff','--name-only','--diff-filter=AM',base,head,'--',':(top)'+PERF_EVIDENCE) or '').split():
  if path.endswith('/after.json'):
   try: record=json.loads(git(repo,'show',head+':'+path) or '')
   except ValueError: continue
   if isinstance(record,dict) and record.get('identical_output') is True and isinstance(record.get('source'),str) and re.fullmatch('[0-9a-f]{40}',record['source']) and subprocess.run(['git','-C',str(repo),'merge-base','--is-ancestor',sha,record['source']],capture_output=True).returncode==0: return True
 return False
NO_BASE='no checkpoint base: the merge base of HEAD and '+CONSOLIDATION+' is unknown; fetch it'
def fix_problems(v,repo=None,base=None,head='HEAD'):
 """Each commit of the checkpoint that changes production code (a merge by its own changes), other than a perf commit (which needs an after.json with identical_output true), names a test in its message that a red_green entry over it (its base_sha excluded, its merge_sha included) proves: the entry names that test, its red log shows it fail and its green log shows it pass."""
 repo=repo or p; base=base or checkpoint_base(repo)
 if not base: return [NO_BASE]
 bad=[]; raw=Path(v.get('raw_identity') or '/nonexistent'); ranges={}
 for sha in (git(repo,'rev-list',base+'..'+head) or '').split():
  message=git(repo,'show','-s','--format=%B',sha) or ''
  # A merge's own changes are the files its result changes against every parent (the combined diff).
  own=['diff-tree','--cc','--no-commit-id','--name-only',sha] if len((git(repo,'rev-list','--parents','-n1',sha) or '').split())>2 else ['diff-tree','--root','--no-commit-id','-r','--name-only',sha]
  files=(git(repo,*own) or '').split()
  if not any(is_production(f) for f in files): continue
  short=sha[:10]+' '+message.splitlines()[0][:70]
  if PERF.match(message):
   if not perf_identical(repo,base,head,sha): bad.append('perf '+short+': no evidence/perf/*/after.json added or changed in the checkpoint with identical_output true and a source built on it')
   continue
  tests=named_tests(message)
  if not tests: bad.append('fix '+short+': its message names no test that fails before it and passes after'); continue
  entries=[x for x in v.get('red_green',[]) if sha in covered(repo,x,ranges)]
  if not entries: bad.append('fix '+short+': no red_green entry has it between its base_sha and merge_sha'); continue
  if not any(proves(x,t,raw) for x in entries for t in tests):
   bad.append('fix '+short+': no red_green entry over it names a test its message names ('+', '.join(tests[:4])+') with a red log showing it fail and a green log showing it pass, in '+str(raw))
 return bad
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
 paths=[x for x in (git(repo,'ls-tree','-r','--name-only',base,'--',':(top)'+crate) or '').split() if x.endswith('.rs') and not is_test_file(x)]
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
def finish(v):
 """Write the matrix numbers block from validation.json, print the scope delta, un-tick each ticked row that cites no test or a test the recorded logs do not prove, check each ticked row cites a validation and each rerun result names its raw log. Returns the exit code."""
 old=readme.read_text(); owners_changed,owner_bad=own(v,old); text=write_block(old,v)
 if owners_changed: validation_json.write_text(dumps({k:x for k,x in v.items() if k!='tests'})+'\n')
 for line in scope_delta(v): print(line)
 bad=list(owner_bad)+red_green_problems(v); diff=['FIX '+x for x in fix_problems(v)]+['DOC '+x for x in doc_problems()]; texts=[]; changed=['validation.json'] if owners_changed else []
 for part in sorted(contracts.glob('*.md')):
  before=old if part==readme else part.read_text(); body,unticked=untick_unproven(text if part==readme else before,v); where='' if part==readme else part.name+' '
  if body!=before: part.write_text(body); changed.append(part.name)
  for line in unticked: print('UNTICKED ROW '+where+line)
  bad+=[where+x for x in ticked_rows_without_proof(body,v['validation'])]
  texts+=[(part.name+':'+str(n),line) for n,line in enumerate(body.splitlines(),1) if TICKED.match(line)]
 texts+=[('validation.json results '+k,OWNED.sub('',x['note'])) for k,x in v['results'].items() if isinstance(x,dict) and 'note' in x]
 bad+=reruns_without_log(texts,v)
 for line in bad: print('TICKED ROW '+line)
 for line in diff: print(line)
 if changed: print('rewritten from validation.json (numbers block, un-ticked rows): '+', '.join(changed)+'; stage them')
 return 1 if bad or diff or changed else 0
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
def item_end(text,start):
 """The end of the item that starts at start: its closing brace, matched; its first `;`, or, for a struct field or enum variant, its `,` or the enclosing `}`, outside brackets."""
 depth=0
 for i in range(start,len(text)):
  c=text[i]
  if depth==0 and c in ';,': return i+1
  if c in '{([' or c=='<' and text[i+1:i+2]!='=': depth+=1
  elif c in ')]' or c=='>' and text[i-1:i] not in ('-','='): depth-=1
  elif c=='}':
   if depth==0: return i
   depth-=1
   if depth==0: return i+1
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
 count=lambda fs:{'files':len(fs),'crates':len({census_crate(f) for f in fs})}
 return {'replace_the_evaluator':{'production':count(ev_prod),'test':count(ev_test),'total_files':len(set(ev_prod)|set(ev_test)),'vocabulary':len(vocabulary)},
  'operation_exhaustive_matches':count(exhaustive),'per_file_fact_touch_points':count(fact),'cross_crate_tests_naming_runtime_internals':count(leaking)}
if __name__=='__main__' and sys.argv[1:]==['--census']: print(json.dumps(census(p.parents[3]),indent=1)); sys.exit(0)
if __name__=='__main__' and sys.argv[1:]==['--check']: sys.exit(finish(dict(json.loads(validation_json.read_text()),tests=loaded(json.loads(tests_json.read_text())) if tests_json.exists() else {})))
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
