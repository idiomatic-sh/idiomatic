"""Fixture cases of `run.py --check`: a ticked row (a `[x]` box or a `☑` cell) stays ticked only when it cites a test, every test it cites passed in the recorded logs, and it does not say its review is pending."""
import importlib.util,json,re,subprocess,tempfile,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('run',Path(__file__).resolve().parent/'run.py'); run=importlib.util.module_from_spec(spec); spec.loader.exec_module(run)
LOG='''     Running unittests src/lib.rs (/target/debug/deps/ryo_values-0123abcd)
test domain::rows::service_test::succeeds ... ok
test domain::table::service_test::succeeds ... FAILED
test domain::table::service_test::refuses ... ok
test domain::early::service_test::succeeds ... FAILED
test domain::loud::service_test::succeeds ... printed while it ran
ok
test domain::ratchet::service_test::holds ... not available: RYO_PREFLIGHT_WORKER does not name a worker
ok

failures:

---- domain::table::service_test::succeeds stdout ----
thread 'domain::table::service_test::succeeds' panicked at crates/values/src/cases.rs:138:5:
2 row(s) of Succeeds failed:
the_first_row_is_red: expected Value(9), answered Ok(1)
the_third_row_is_red: expected Value(9), answered Ok(3)

---- domain::early::service_test::succeeds stdout ----
thread 'domain::early::service_test::succeeds' panicked at crates/values/src/cases.rs:116:9:
the_first_row_is_red: expected Value(9), answered Ok(1)

test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
'''
def validation(raw):
 log=Path(raw)/'libraries.log'; log.write_text(LOG)
 return {'validation':321,'raw_identity':raw,'tests':{'libraries':run.test_results(log)}}
class Check(unittest.TestCase):
 def setUp(self):
  self.raw=tempfile.TemporaryDirectory(); self.v=validation(self.raw.name)
  run.sources.found={'the_second_row_is_green':[({'ryo_values'},['domain','early','service_test'])]}
 def tearDown(self): self.raw.cleanup()
 def untick(self,line): return run.untick_unproven(line+'\n',self.v)
 def test_a_table_with_a_red_first_and_a_red_third_row_reports_both(self):
  failed=self.v['tests']['libraries']['ryo_values']['failed']['domain::table::service_test::succeeds']
  self.assertEqual(failed,{'rows':['the_first_row_is_red','the_third_row_is_red'],'collected':True})
  self.assertIn('red rows 2: the_first_row_is_red, the_third_row_is_red',' '.join(run.red_rows(self.v)))
 def test_a_status_printed_after_the_test_output_is_read(self):
  self.assertIn('domain::loud::service_test::succeeds',self.v['tests']['libraries']['ryo_values']['passed'])
 def test_a_ticked_row_whose_cited_test_passed_stays_ticked(self):
  self.assertEqual(self.untick('| 2 | [x] Rows. | validation321: `rows::service_test::succeeds` |'),('| 2 | [x] Rows. | validation321: `rows::service_test::succeeds` |\n',[]))
 def test_a_ticked_row_citing_a_red_test_is_unticked_with_the_row_and_the_test_named(self):
  text,bad=self.untick('| 3 | [x] Tables. | validation321: `table::service_test::succeeds` |')
  self.assertTrue(text.startswith('| 3 | [ ] Tables.')); self.assertIn('red `table::service_test::succeeds`',bad[0]); self.assertTrue(bad[0].startswith('1: 0 of 1'))
 def test_a_ticked_row_citing_an_absent_test_is_unticked(self):
  text,bad=self.untick('- [x] Deleted. (validation321: `walk_routes::selection::test::*`, `gone::service_test::succeeds`)')
  self.assertTrue(text.startswith('- [ ] Deleted.')); self.assertIn('absent `walk_routes::selection::test::*`; absent `gone::service_test::succeeds`',bad[0])
 def test_a_row_after_an_aborted_red_row_did_not_run(self):
  text,bad=self.untick('- [x] Early. (validation321: `early::service_test::the_second_row_is_green`)')
  self.assertIn('not run `early::service_test::the_second_row_is_green`',bad[0])
 def test_a_cited_module_passes_when_every_test_under_it_passed(self):
  self.assertEqual(self.untick('- [x] Rows. (validation321: `domain::rows::service_test`)')[1],[])
 def test_type_paths_and_short_names_are_not_cited_tests(self):
  self.assertEqual(run.cited_tests('`Fusibility::{Fusable, Barrier}` `executable_program::service::choose` `boundary_contract` `raw_scope`'),[])
 def test_a_ticked_row_citing_no_test_is_unticked(self):
  text,bad=self.untick('| 2 VERIFIED | [x] Seal. | validation321: all controls pass. |')
  self.assertTrue(text.startswith('| 2 VERIFIED | [ ] Seal.')); self.assertIn('cites no test',bad[0])
 def test_every_checked_cell_of_a_matrix_row_citing_no_test_is_unchecked(self):
  text,bad=self.untick('| 16. Stored selections | `ryo-execution` | ☑ | — | ☑ | Controls pass307. |')
  self.assertEqual(text,'| 16. Stored selections | `ryo-execution` | ☐ | — | ☐ | Controls pass307. |\n'); self.assertIn('cites no test',bad[0])
 def test_a_checklist_row_stays_checked_only_while_its_cited_test_passed(self):
  self.assertEqual(self.untick('| ☑ | A1 | Rows. | Proof321: `rows::service_test::succeeds`. |')[1],[])
  self.assertTrue(self.untick('| ☑ | A2 | Tables. | Proof321: `table::service_test::succeeds`. |')[0].startswith('| ☐ | A2 |'))
 def test_a_ticked_row_saying_its_review_is_pending_is_unticked_though_its_tests_passed(self):
  text,bad=self.untick('| 2 VERIFIED-PENDING-REVIEW | [x] Rows. | validation321: `rows::service_test::succeeds` |')
  self.assertTrue(text.startswith('| 2 VERIFIED-PENDING-REVIEW | [ ] Rows.')); self.assertIn('review is pending',bad[0])
 def test_a_test_that_printed_not_available_is_not_run_and_a_row_citing_it_is_unticked(self):
  t=self.v['tests']['libraries']['ryo_values']
  self.assertEqual(t['not_run'],['domain::ratchet::service_test::holds']); self.assertNotIn('domain::ratchet::service_test::holds',t['passed'])
  text,bad=self.untick('- [x] Ratchet. (validation321: `ratchet::service_test::holds`)')
  self.assertTrue(text.startswith('- [ ] Ratchet.')); self.assertIn('not run `ratchet::service_test::holds`',bad[0])
 def test_the_checks_block_and_the_tm_validate_counts_are_the_runs_own(self):
  raw=Path(self.raw.name)
  (raw/'libraries.json').write_text(json.dumps({'source':'new','exit':101,'elapsed_seconds':1.0}))
  (raw/'tm_validate_all.log').write_text('workspace crates_ryo\n{"proven":240,"kept":0,"failed":[]}\nworkspace repository_root\nerror: one\n{"proven":222,"kept":0,"failed":[{"id":"a"}]}\n')
  v={'checks':{'libraries':{'source':'old'},'gone':{'source':'old'}},'results':{}}
  run.record(v,[['libraries',['cargo','test']],['tm_validate_all',['sh']]],raw)
  self.assertEqual(v['checks'],{'libraries':{'source':'new','exit':101,'elapsed_seconds':1.0}})
  self.assertEqual(v['results']['tm_validate_all'],{'crates_ryo':{'passed':240,'failed':0},'repository_root':{'passed':222,'failed':1}})
 def test_a_rerun_result_without_its_raw_log_fails(self):
  self.assertEqual(len(run.reruns_without_log([('note','the row passes in its narrow run')],self.v)),1)
  self.assertEqual(len(run.reruns_without_log([('note','the row passes in its narrow run (libraries.log)')],self.v)),0)
  self.assertEqual(len(run.reruns_without_log([('note','rerun in boundary-rerun.log')],self.v)),1)
  self.assertEqual(len(run.reruns_without_log([('note','green on five reruns')],self.v)),1)
 def owned_reds(self):
  """Every OWNERS entry red: replay tests in cli_replay_reds, the rest as failed results."""
  results={k:{'exit':101} for k in run.OWNERS if '::' not in k}
  results['cli_replay_reds']=[k for k in run.OWNERS if '::' in k]
  return results
 def owned_record(self):
  """owned_reds() as own() leaves it: every OWNERS entry red and carrying its owner note, so finish() rewrites nothing for owners."""
  results=self.owned_reds(); run.own({'results':results},''); return results
 def test_every_red_result_carries_its_owner_and_a_green_one_carries_none(self):
  readme=''.join('- [ ] '+t+'\n' for t in set(run.OWNERS.values()))
  v={'results':{**self.owned_reds(),'ruff_rule_phases':{'exit':0,'note':'repaired; owned by "Close full TM, contracts, independent review and normal CI."'},'unowned_target':{'failed':1},'libraries_total':{'passed':1,'failed':1}}}
  changed,bad=run.own(v,readme)
  self.assertTrue(changed); self.assertEqual(bad,['red unowned_target has no owner in OWNERS'])
  self.assertEqual(v['results']['core_allocation']['note'],'owned by "'+run.OWNERS['core_allocation']+'"')
  self.assertEqual(v['results']['ruff_rule_phases']['note'],'repaired')
  self.assertNotIn('note',v['results']['libraries_total'])
 def test_a_green_result_with_only_a_stale_owner_fails_the_check(self):
  readme=''.join('- [ ] '+t+'\n' for t in set(run.OWNERS.values()))
  results=self.owned_reds(); results['core_allocation']={'exit':0}
  changed,bad=run.own({'results':results},readme)
  self.assertEqual(bad,['stale owner: core_allocation is not red, so OWNERS may not pre-own it'])
  replay=self.owned_reds(); replay['cli_replay_reds']=[]
  changed,bad=run.own({'results':replay},readme)
  self.assertEqual(sorted(bad),sorted('stale owner: '+k+' is not red, so OWNERS may not pre-own it' for k in run.OWNERS if '::' in k))
 def test_a_red_is_owned_only_by_its_exact_command_binary_and_test(self):
  benchmark=('libraries','ryo_benchmark','inbound::use_cases::verify_cases::use_case_test::process_capture::test::succeeds')
  tests={'libraries':{'ryo_benchmark':{'failed':{benchmark[2]:{}}}},'cli_cost':{'replay':{'failed':{'cost::a_cold_run_over_ruff_holds_the_peak_ratchet':{}}}}}
  v={'tests':tests,'checks':{'libraries':{'exit':101},'cli_cost':{'exit':101},'ruff':{'exit':0}}}
  self.assertEqual(run.red_commands(v),[('libraries',101,[run.RED_OWNERS[benchmark]]),('cli_cost',101,[run.OWNERS['cli_cost']])])
  tests['libraries']['ryo_core']={'failed':{'brand_new::regression':{}}}
  self.assertEqual(run.red_commands(v)[0],('libraries',101,[]))
 def test_the_allocation_runner_reports_its_case_as_cargo_does_and_its_red_is_owned_by_that_id(self):
  source=(Path(run.__file__).parents[4]/'crates/core/tests/allocation.rs').read_text()
  case=re.search(r'const CASE: &str = "([^"]+)";',source).group(1)
  log=Path(self.raw.name)/'core_allocation.log'
  log.write_text('     Running tests/allocation.rs (/t/debug/deps/allocation-7c079f4def874c49)\n\nrunning 1 test\nthread \'main\' panicked at measures.rs:115:5:\nassertion failed\ntest '+case+' ... FAILED\n\nfailures:\n    '+case+'\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n')
  tests={'core_allocation':run.test_results(log)}
  self.assertEqual(list(tests['core_allocation']['allocation']['failed']),[case])
  self.assertEqual(run.command_owners('core_allocation',{'exit':101},tests),[run.OWNERS['core_allocation']])
 def test_a_binary_cargo_failed_without_a_test_result_line_crashed_and_its_red_is_unowned(self):
  benchmark='inbound::use_cases::verify_cases::use_case_test::process_capture::test::succeeds'
  log=Path(self.raw.name)/'libraries.log'
  log.write_text('     Running unittests src/lib.rs (/t/debug/deps/ryo_benchmark-e4783985ddbec807)\n'
   'test '+benchmark+' ... FAILED\n\ntest result: FAILED. 227 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n\n'
   'error: test failed, to rerun pass `-p idiomatic-sh-ryo-benchmark --lib`\n'
   '     Running unittests src/lib.rs (/t/debug/deps/ryo_core-90ad667b7be178a2)\n\nrunning 120 tests\n'
   "thread 'main' has overflowed its stack\nfatal runtime error: stack overflow\n"
   'error: test failed, to rerun pass `-p idiomatic-sh-ryo-core --lib`\n'
   '     Running unittests src/lib.rs (/t/debug/deps/ryo_values-0bf301fa394de9a3)\n'
   'test domain::rows::service_test::succeeds ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n')
  tests={'libraries':run.test_results(log)}
  self.assertEqual(list(tests['libraries']['ryo_core']['failed']),[run.CRASHED])
  self.assertEqual(list(tests['libraries']['ryo_benchmark']['failed']),[benchmark])
  self.assertEqual(tests['libraries']['ryo_values']['failed'],{})
  self.assertEqual(run.command_owners('libraries',{'exit':101},tests),[])
 def test_a_red_command_that_recorded_no_failed_test_is_unowned_whatever_its_name(self):
  v={'tests':{},'checks':{'tm_import_cli':{'exit':101},'core_allocation':{'exit':101}}}
  self.assertEqual(run.red_commands(v),[('tm_import_cli',101,[]),('core_allocation',101,[])])
 def test_an_owner_row_that_is_ticked_owns_nothing(self):
  rows=''.join('- [ ] '+t+'\n' for t in set(run.OWNERS.values()))
  self.assertEqual(run.open_row_problems(rows),[])
  ticked=rows.replace('- [ ] '+run.CLOSE,'- [x] '+run.CLOSE)
  self.assertTrue(run.open_row_problems(ticked)); self.assertTrue(all('ticked README row' in x for x in run.open_row_problems(ticked)))
  self.assertTrue(any('ticked README row' in x for x in run.own({'results':{}},ticked)[1]))
 def test_a_refusal_to_run_here_is_owned_only_as_a_refusal(self):
  raw=Path(self.raw.name)
  (raw/'ruff_parity.log').write_text('not available: the pinned Django request is absent\n')
  (raw/'other.log').write_text('error: parity differs\n')
  self.assertEqual(run.unavailable(raw/'ruff_parity.log'),'the pinned Django request is absent')
  self.assertIsNone(run.unavailable(raw/'other.log')); self.assertIsNone(run.unavailable(raw/'absent.log'))
  self.assertEqual(run.command_owners('ruff_parity',{'exit':2,'not_available':'absent'},{}),[run.UNAVAILABLE_OWNERS['ruff_parity']])
  self.assertEqual(run.command_owners('ruff_parity',{'exit':1},{}),[])
  self.assertEqual(run.command_owners('bench_build',{'exit':2,'not_available':'absent'},{}),[])
 def test_a_rewritten_record_fails_only_the_check_of_the_committed_record_and_an_unowned_red_fails_both(self):
  folder=Path(self.raw.name)/'contracts'; folder.mkdir()
  rows=''.join('- [ ] '+t+'\n' for t in set(run.OWNERS.values()))
  saved=(run.readme,run.contracts,run.validation_json,run.fix_problems,run.doc_problems)
  run.readme,run.contracts,run.validation_json=folder/'README.md',folder,folder/'validation.json'
  # The record alone decides here: the checkpoint's own FIX and DOC lines over the real repository are their own tests' subject.
  run.fix_problems=run.doc_problems=lambda *a,**k: []
  try:
   def verdict(check,checks):
    run.readme.write_text('### Current ownership checkpoint — stale\n\n'+rows)
    v={'source':'a'*40,'validation':322,'raw_identity':self.raw.name,'results':self.owned_record(),'checks':checks,'tm_whole_tree':{'total':0,'suppressed':0,'at_checkpoint_318':0},'ownership_graph':{'other_cycles':0,'targeted_cycles':[]}}
    return run.finish(v,check=check)
   self.assertEqual((verdict(False,{}),verdict(True,{})),(0,1))
   refused={'ruff_parity':{'exit':2,'not_available':'the pinned Django request is absent'}}
   import io,contextlib
   said=io.StringIO()
   with contextlib.redirect_stdout(said): self.assertEqual(verdict(False,refused),0)
   self.assertIn('NOT RUN ruff_parity: parity not run: the pinned Django request is absent',said.getvalue())
   self.assertEqual((verdict(False,{'bench_build':{'exit':101}}),verdict(True,{'bench_build':{'exit':101}})),(1,1))
   # The leak review's three plants over the committed record: a new red test under an owned command, a red command with no
   # test, and the parity refusal.
   committed=json.loads((Path(run.__file__).parent/'validation.json').read_text())['checks']
   run_tests=run.loaded(json.loads((Path(run.__file__).parent/'tests.json').read_text()))
   run_tests['libraries'].setdefault('ryo_core',{'passed':[],'ignored':[],'not_run':[],'failed':{}})['failed']['brand_new::regression']={'rows':[],'collected':False}
   planted=dict(committed,tm_import_cli={'exit':101},**refused)
   # The re-check's plant: a ryo_core crash beside the owned reds of the same command.
   alone=dict(committed,**refused)
   crashed=run.loaded(json.loads((Path(run.__file__).parent/'tests.json').read_text()))
   crashed['libraries']['ryo_core']={'passed':[],'ignored':[],'not_run':[],'failed':{run.CRASHED:{'rows':[],'collected':False}}}
   crashed['core_allocation']={'allocation':{'passed':[],'ignored':[],'not_run':[],'failed':{'per_file_path::allocates_as_the_parser_library_does':{'rows':[],'collected':False}}}}
   run.readme.write_text('### Current ownership checkpoint — stale\n\n'+rows)
   v={'source':'a'*40,'validation':322,'raw_identity':self.raw.name,'results':self.owned_record(),'checks':alone,'tests':crashed,'tm_whole_tree':{'total':0,'suppressed':0,'at_checkpoint_318':0},'ownership_graph':{'other_cycles':0,'targeted_cycles':[]}}
   with contextlib.redirect_stdout(io.StringIO()): self.assertEqual(run.finish(v),1)
   del crashed['libraries']['ryo_core']
   run.readme.write_text('### Current ownership checkpoint — stale\n\n'+rows)
   with contextlib.redirect_stdout(io.StringIO()): self.assertEqual(run.finish(dict(v,tests=crashed)),0)
   run.readme.write_text('### Current ownership checkpoint — stale\n\n'+rows)
   v={'source':'a'*40,'validation':322,'raw_identity':self.raw.name,'results':self.owned_record(),'checks':planted,'tests':run_tests,'tm_whole_tree':{'total':0,'suppressed':0,'at_checkpoint_318':0},'ownership_graph':{'other_cycles':0,'targeted_cycles':[]}}
   with contextlib.redirect_stdout(io.StringIO()): self.assertEqual(run.finish(v),1)
  finally: run.readme,run.contracts,run.validation_json,run.fix_problems,run.doc_problems=saved
 def test_a_test_that_printed_not_available_counts_as_not_run_never_as_passed(self):
  tests={'cli_cost':{'replay':{'passed':['cost::typed'],'ignored':[],'not_run':['cost::django'],'failed':{'cost::ratchet':{'rows':[],'collected':False}}}}}
  self.assertEqual(run.count_problems({'results':{'cli_cost':{'passed':1,'failed':1,'not_run':1}},'tests':tests}),[])
  self.assertEqual(run.count_problems({'results':{'cli_cost':{'passed':2,'failed':1,'not_run':0}},'tests':tests}),["result cli_cost gives {'passed': 2, 'failed': 1, 'not_run': 0}, its recorded tests give {'passed': 1, 'failed': 1, 'not_run': 1}"])
  self.assertEqual(run.count_problems({'results':{'cli_cost':{'passed':2,'failed':1}},'tests':tests}),["result cli_cost gives {'passed': 2, 'failed': 1, 'not_run': 0}, its recorded tests give {'passed': 1, 'failed': 1, 'not_run': 1}"])
def commit(repo,files,message):
 """Writes the files (path -> text, None deletes) into the fixture repository and commits them; returns the sha."""
 for path,body in files.items():
  f=Path(repo)/path
  if body is None: f.unlink(); continue
  f.parent.mkdir(parents=True,exist_ok=True); f.write_text(body)
 subprocess.run(['git','-C',repo,'add','-A'],check=True); subprocess.run(['git','-C',repo,'commit','-q','-m',message],check=True)
 return subprocess.run(['git','-C',repo,'rev-parse','HEAD'],capture_output=True,text=True,check=True).stdout.strip()
RED='running 1 test\ntest domain::service_test::one_answers ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed\n'
GREEN='running 1 test\ntest domain::service_test::one_answers ... ok\n\ntest result: ok. 1 passed; 0 failed\n'
class Diff(unittest.TestCase):
 """The checkpoint-diff checks: a fix carries its red-then-green, and a published crate's new pub item its examples."""
 def setUp(self):
  self.dir=tempfile.TemporaryDirectory(); self.repo=self.dir.name; self.raw=Path(self.repo)/'raw'; self.raw.mkdir()
  subprocess.run(['git','init','-q','-b','claude/checkpoint-9',self.repo],check=True)
  for k,x in (('user.email','t@example.com'),('user.name','t')): subprocess.run(['git','-C',self.repo,'config',k,x],check=True)
  (Path(self.repo)/'.gitignore').write_text('raw/\n')
  self.base=commit(self.repo,{'a/Cargo.toml':'[package]\nname="a"\n','a/src/lib.rs':'/// One.\n///\n/// # Examples\n/// ```\n/// ```\npub fn one() {}\n','a/src/old.rs':'pub fn kept() {}\n','a/src/x.rs':'pub fn parse() {}\n','a/tests/t.rs':'pub fn shadow() {}\n','b/Cargo.toml':'[package]\nname="b"\npublish = false\n','b/src/lib.rs':''},'chore: base')
 def tearDown(self): self.dir.cleanup()
 def logs(self,**named):
  for name,text in named.items(): (self.raw/(name+'.log')).write_text(text)
 def fixes(self,entries,base=None): return run.fix_problems({'red_green':entries,'raw_identity':str(self.raw)},repo=self.repo,base=base or self.base)
 def entry(self,sha,text): return {'test':text,'base_sha':self.base,'merge_sha':sha}
 def test_a_fix_whose_entry_names_its_test_with_a_red_and_a_green_log_passes(self):
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): one answers\n\nTest: `service_test::one_answers` fails before, passes after.')
  self.logs(red=RED,green=GREEN)
  self.assertEqual(self.fixes([self.entry(sha,'a service_test::one_answers (red red.log, green green.log)')]),[])
 def test_a_fix_naming_no_test_or_with_no_entry_over_it_is_refused(self):
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): one answers')
  self.assertIn('names no test',self.fixes([])[0])
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() {  }\n'},'fix(a): one answers, `service_test::one_answers`')
  self.assertTrue(any('between its base_sha and merge_sha' in x and sha[:10] in x for x in self.fixes([{'test':'service_test::one_answers (red red.log, green green.log)','base_sha':sha,'merge_sha':sha}])))
 def test_an_entry_that_does_not_prove_the_named_test_is_refused(self):
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): one answers, `service_test::one_answers`')
  self.logs(red=RED,green=GREEN,cli_tests='test other::case ... ok\n',flipped=GREEN,typed='test domain::service_test::one_answers ... FAILED\n')
  for text in ('a service_test::one_answers',                                          # no logs
               'a service_test::one_answers (red red.log)',                            # one log only
               'a service_test::one_answers (red red.log, green red.log)',             # the same log twice
               'a service_test::one_answers (red cli_tests.log, green green.log)',     # an unrelated red log
               'a service_test::one_answers (red flipped.log, green green.log)',       # a red log where it passed
               'a service_test::one_answers (red red.log, green gone.log)',            # a log not in the directory
               'a service_test::one_answers (red typed.log, green green.log)',         # a red log without the runner's frame
               'a other::case (red red.log, green green.log)'):                       # another test than the message names
   with self.subTest(text=text): self.assertIn('no red_green entry over it names a test',self.fixes([self.entry(sha,text)])[0])
 def test_a_module_path_in_the_message_is_no_proved_test(self):
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): guard_shell::builder_check refuses more')
  self.logs(cli_tests='test other::case ... ok\n')
  self.assertEqual(len(self.fixes([self.entry(sha,'guard_shell::builder_check (red cli_tests.log, green cli_tests.log)')])),1)
 def test_a_behaviour_change_of_any_type_but_perf_is_a_fix(self):
  for n,kind in enumerate(('chore','docs','test','style','build','refactor')):
   with self.subTest(kind=kind):
    commit(self.repo,{'a/src/lib.rs':'pub fn one() {'+' '*(n+1)+'}\n'},kind+'(a): one')
    self.assertEqual(len(self.fixes([])),n+1)
 def test_harness_python_is_production(self):
  for n,path in enumerate(('ci/contract_review/service.py','crates/ryo/docs/contracts/evidence/checkpoint/run.py','.claude/hooks/guard_shell.py')):
   with self.subTest(path=path):
    commit(self.repo,{path:'x = '+str(n)+'\n'},'chore: harness')
    self.assertEqual(len(self.fixes([])),n+1)
 def test_test_files_and_docs_are_not_fixes(self):
  commit(self.repo,{'a/src/lib_test.rs':'x','a/README.md':'x','ci/contract_review/service_test.py':'x','.claude/hooks/test_hooks.py':'x'},'fix(a): tests and docs')
  self.assertEqual(self.fixes([]),[])
 def test_a_perf_commit_needs_an_after_json_with_identical_output(self):
  commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'perf(a): one is faster')
  self.assertIn('identical_output true',self.fixes([])[0])
  commit(self.repo,{'crates/ryo/docs/contracts/evidence/perf/one/after.json':'{"identical_output": false}'},'docs: perf evidence')
  self.assertEqual(len(self.fixes([])),1)
  for source in ('"'+self.base+'"','"'+'0'*40+'"','null'):  # a commit before the perf commit, an unknown one, none
   with self.subTest(source=source):
    commit(self.repo,{'crates/ryo/docs/contracts/evidence/perf/one/after.json':'{"identical_output": true, "source": '+source+'}'},'docs: perf evidence')
    self.assertEqual(len(self.fixes([])),1)
  perf=subprocess.run(['git','-C',self.repo,'rev-list','-1','--grep=^perf','HEAD'],capture_output=True,text=True).stdout.strip()
  commit(self.repo,{'crates/ryo/docs/contracts/evidence/perf/one/after.json':'{"identical_output": true, "source": "'+perf+'"}'},'docs: perf evidence')
  self.assertEqual(self.fixes([]),[])
  later=commit(self.repo,{'a/README.md':'later\n'},'docs: later')
  commit(self.repo,{'crates/ryo/docs/contracts/evidence/perf/one/after.json':'{"identical_output": true, "source": "'+later+'"}'},'docs: perf evidence')
  self.assertEqual(self.fixes([]),[])
 def test_a_merges_own_production_change_is_a_fix_and_a_clean_merge_is_not(self):
  subprocess.run(['git','-C',self.repo,'checkout','-q','-b','side'],check=True)
  commit(self.repo,{'a/README.md':'side\n'},'docs: side')
  subprocess.run(['git','-C',self.repo,'checkout','-q','claude/checkpoint-9'],check=True)
  subprocess.run(['git','-C',self.repo,'merge','-q','--no-ff','side','-m','Merge side'],check=True)
  self.assertEqual(self.fixes([]),[])
  subprocess.run(['git','-C',self.repo,'checkout','-q','side'],check=True)
  commit(self.repo,{'a/README.md':'side 2\n'},'docs: side 2')
  subprocess.run(['git','-C',self.repo,'checkout','-q','claude/checkpoint-9'],check=True)
  subprocess.run(['git','-C',self.repo,'merge','-q','--no-ff','--no-commit','side'],check=True)
  (Path(self.repo)/'a/src/lib.rs').write_text('pub fn one() { /* resolved in the merge */ }\n')
  subprocess.run(['git','-C',self.repo,'commit','-q','-am','Merge side again'],check=True)
  self.assertEqual(len(self.fixes([])),1)
 def test_a_production_commit_the_baseline_reaches_is_ignored_and_one_after_it_is_checked(self):
  before=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): before the rule')
  after=commit(self.repo,{'a/src/lib.rs':'pub fn one() {  }\n'},'fix(a): after the rule')
  found=run.fix_problems({'red_green':[],'raw_identity':str(self.raw)},repo=self.repo,base=self.base,baseline=before)
  self.assertEqual(len(found),1); self.assertIn(after[:10],found[0]); self.assertNotIn(before[:10],' '.join(found))
  self.assertEqual(len(run.fix_problems({'red_green':[]},repo=self.repo,base=self.base,baseline=None)),2)
 def test_a_pytest_log_proves_its_test_red_and_green(self):
  head='============================= test session starts ==============================\n'
  self.logs(red=head+'ci/a_test.py::test_one_answers FAILED [100%]\n=========================== short test summary info ============================\nFAILED ci/a_test.py::test_one_answers - assert 1 == 2\n============================== 1 failed in 0.10s ===============================\n',
            green=head+'ci/a_test.py::test_one_answers PASSED [100%]\n============================== 1 passed in 0.10s ===============================\n',
            bare='ci/a_test.py::test_one_answers PASSED [100%]\n')
  self.assertEqual([run.log_status(self.raw/(n+'.log'),'test_one_answers') for n in ('red','green','bare')],['failed','passed',None])
 def test_a_clippy_lint_is_red_at_the_changed_spans_and_green_when_they_lose_it(self):
  lint=lambda f,code: json.dumps({'reason':'compiler-message','message':{'code':{'code':code},'spans':[{'file_name':f,'is_primary':True}]}})
  done=json.dumps({'reason':'build-finished','success':False})
  changed=['crates/ryo/crates/values/src/a.rs']
  self.logs(red='\n'.join([lint('crates/values/src/a.rs','clippy::as_conversions'),done]),
            green='\n'.join([lint('crates/derived/src/b.rs','clippy::as_conversions'),lint('crates/values/src/a.rs','clippy::expect_used'),done]),
            cut=lint('crates/values/src/a.rs','clippy::expect_used'))
  status=lambda name,test: run.log_status(self.raw/(name+'.log'),test,changed)
  self.assertEqual([status('red','clippy::as_conversions'),status('green','clippy::as_conversions')],['failed','passed'])
  self.assertEqual([status('green','clippy::expect_used'),status('cut','clippy::as_conversions')],['failed',None])
 def test_without_a_base_both_checks_refuse(self):
  self.assertEqual(run.fix_problems({'red_green':[]},repo=self.repo),[run.NO_BASE])
  self.assertEqual(run.doc_problems(repo=self.repo),[run.NO_BASE])
 def test_a_new_pub_item_of_a_published_crate_needs_examples_and_errors(self):
  commit(self.repo,{'a/src/lib.rs':'pub fn one() {}\n/// Two.\npub fn two() -> Result<(), E> {}\n/// Three.\n///\n/// # Examples\n/// ```\n/// ```\n/// # Errors\npub fn three() -> std::result::Result<(), E> {}\npub(crate) fn four() {}\nmod seal { pub trait Sealed {} }\n',
                        'a/src/fixtures.rs':'pub fn fixture() {}\n','a/tests/t.rs':'pub fn helper() {}\n','b/src/lib.rs':'pub fn unpublished() {}\n'},'feat(a): two and three')
  self.assertEqual(run.doc_problems(repo=self.repo,base=self.base),['a/src/lib.rs:3 pub fn two: no `# Examples` section with a code block','a/src/lib.rs:3 pub fn two: returns a Result with no `# Errors` section'])
 def test_an_item_kept_in_its_module_is_not_new_and_one_moved_to_another_module_is(self):
  commit(self.repo,{'a/src/old.rs':None,'a/src/old/mod.rs':'pub fn kept() {}\n','a/src/x.rs':None,'a/src/y.rs':'pub fn parse() -> Result<(), E> {}\n'},'refactor(a): move')
  found=run.doc_problems(repo=self.repo,base=self.base)
  self.assertFalse(any('pub fn kept:' in x for x in found)); self.assertTrue(any('a/src/y.rs:1 pub fn parse:' in x for x in found),found)
 def test_an_item_kept_in_its_module_is_not_new_when_the_checks_run_from_a_subdirectory(self):
  # run.py passes its own directory as the repository, deep inside the tree: base paths must be read from the root.
  commit(self.repo,{'a/src/old.rs':'// Kept.\npub fn kept() {}\n','d/e/note.md':'deep\n'},'docs: a deep directory')
  self.assertEqual(run.doc_problems(repo=Path(self.repo)/'d'/'e',base=self.base),[])
 def test_an_examples_heading_without_a_code_block_is_no_example(self):
  commit(self.repo,{'a/src/z.rs':'/// Zed.\n///\n/// # Examples\npub fn zed() {}\n','a/src/w.rs':'/// Wye.\n///\n/// # Examples\n///\n/// # Panics\n/// ```\n/// ```\npub fn wye() {}\n'},'feat(a): zed')
  self.assertEqual(run.doc_problems(repo=self.repo,base=self.base),['a/src/w.rs:8 pub fn wye: no `# Examples` section with a code block','a/src/z.rs:4 pub fn zed: no `# Examples` section with a code block'])
 def test_a_test_named_directory_holds_tests(self):
  for path in ('a/src/service_test/cases.rs','crates/x/src/a/unit_test/mod.rs'):
   with self.subTest(path=path): self.assertTrue(run.is_test_file(path))
  self.assertFalse(run.is_test_file('a/src/testing/mod.rs'))
 def test_new_items_that_share_a_name_or_hide_in_a_rename_or_a_test_named_file_are_new(self):
  commit(self.repo,{'a/src/kind.rs':'pub struct Kind;\nimpl Kind {\n    pub fn one() -> Result<(), E> {}\n}\n',      # one is pub at the base, on no type
                    'a/src/shadow.rs':'pub fn shadow() {}\n',                                                   # pub at the base only in a test file
                    'a/src/old.rs':None,'a/src/renamed.rs':'pub fn kept() {}\npub fn added() {}\n',             # a rename that adds an item
                    'a/src/test_support.rs':'pub fn support() {}\n'},'feat(a): kinds')
  found=run.doc_problems(repo=self.repo,base=self.base)
  for item in ('pub struct Kind','pub fn one','pub fn shadow','pub fn added','pub fn support'):
   with self.subTest(item=item): self.assertTrue(any(item+':' in x for x in found),found)
  self.assertTrue(any('a/src/renamed.rs:1 pub fn kept:' in x for x in found),found)  # a renamed module is a new path
class Census(unittest.TestCase):
 FILES={'crates/execution_runtime/src/lib.rs':'pub struct FileSession;\npub trait DigestPort {}\n','crates/core/src/a.rs':'use ryo_execution::FileSession;\n',
        'crates/core/src/b.rs':'use x::DigestPort;\n','crates/core/src/c.rs':'ryo_execution_runtime::run();\n','crates/core/tests/t.rs':'FileSession\n',
        'crates/execution_store/tests/s.rs':'RelationOwner\n','crates/values/src/m.rs':' '.join('Operation::V'+str(n) for n in range(8)),'crates/values/src/n.rs':'Operation::V1',
        'crates/values/src/op.rs':'pub enum Operation {\n'+''.join('    V'+str(n)+',\n' for n in range(9))+'}\n',
        'languages/go/src/r.rs':'binding_read','crates/relations/src/r.rs':'BindingRead','crates/core/src/r_test.rs':'binding_read'}
 def census(self,**changes):
  with tempfile.TemporaryDirectory() as repo:
   subprocess.run(['git','init','-q',repo],check=True)
   for path,body in dict(self.FILES,**{k.replace('__','/').replace('_DOT_','.'):x for k,x in changes.items()}).items():
    f=Path(repo)/path; f.parent.mkdir(parents=True,exist_ok=True); f.write_text(body)
   subprocess.run(['git','-C',repo,'add','-A'],check=True)
   return run.census(repo)
 def test_the_census_counts_each_coupling_in_its_owners_terms(self):
  c=self.census()
  self.assertEqual(c['replace_the_evaluator'],{'production':{'files':2,'crates':1},'test':{'files':1,'crates':1},'total_files':3,'vocabulary':2})
  self.assertEqual(c['operation_exhaustive_matches'],{'files':1,'crates':1})
  self.assertEqual(c['per_file_fact_touch_points'],{'files':2,'crates':2})
  self.assertEqual(c['cross_crate_tests_naming_runtime_internals'],{'files':1,'crates':1})
 def test_an_alias_or_a_glob_of_operation_is_still_an_exhaustive_match(self):
  for body in ('use x::Operation as O;\n'+' '.join('O::V'+str(n) for n in range(8)),'use x::Operation::*;\n'+' '.join('V'+str(n) for n in range(8))):
   with self.subTest(body=body[:24]): self.assertEqual(self.census(crates__values__src__m_DOT_rs=body)['operation_exhaustive_matches']['files'],1)
 def test_a_type_alias_of_operation_is_still_an_exhaustive_match(self):
  body='type O = x::Operation;\n'+' '.join('O::V'+str(n) for n in range(8))
  self.assertEqual(self.census(crates__values__src__m_DOT_rs=body)['operation_exhaustive_matches']['files'],1)
 def test_a_same_named_type_elsewhere_does_not_hide_the_evaluators(self):
  for path in ('crates__other__tests__x_DOT_rs','crates__other__src__decoy_DOT_rs'):
   with self.subTest(path=path): self.assertEqual(self.census(**{path:'pub struct FileSession;\n'})['replace_the_evaluator']['total_files'],3)
 def test_a_name_its_own_crate_declares_or_imports_from_elsewhere_or_a_variant_is_not_the_evaluators(self):
  for path,body in (('crates__other__src__own_DOT_rs','pub struct FileSession;\nFileSession\n'),('crates__other__src__imp_DOT_rs','use ryo_sdk::FileSession;\nFileSession\n'),
                    ('crates__other__src__var_DOT_rs','Error::FileSession\n')):
   with self.subTest(path=path): self.assertEqual(self.census(**{path:body})['replace_the_evaluator']['production']['files'],2)
 def test_a_cfg_test_field_ends_at_its_comma_or_the_closing_brace(self):
  value=(Path(__file__).resolve().parents[4]/'crates/execution_domain/src/domain/kind_sets/types/vocabulary/value.rs').read_text()
  elements=(Path(__file__).resolve().parents[4]/'crates/language/src/domain/elements/types.rs').read_text()
  for name,text,kept in (('value.rs',value,'pub(crate) struct PreparedKindTest'),('elements/types.rs',elements,'pub decoded_text: Bytes')):
   with self.subTest(name=name):
    production,tests=run.split_tests(text)
    self.assertIn(kept,production); self.assertLess(len(tests),200,tests[:300])
  production,tests=run.split_tests('struct S {\n    #[cfg(test)]\n    a: HashMap<A, B>,\n    b: C,\n}\nstruct T {\n    #[cfg(test)]\n    z: u8\n}\nfn after() {}\n')
  self.assertEqual(tests,'    #[cfg(test)]\n    a: HashMap<A, B>,    #[cfg(test)]\n    z: u8\n'); self.assertIn('b: C',production); self.assertIn('fn after',production)
 def test_a_cfg_test_fn_body_ends_at_its_brace_whatever_it_compares_or_quotes(self):
  for body in ('self.seen < 4','n << 2','let c = \'{\';','let s = "}{"; let r = r#"{"#;','// a { in a comment\n','x as u8 > 1 && y >= 2'):
   with self.subTest(body=body):
    production,tests=run.split_tests('#[cfg(test)]\nfn early() -> bool {\n    '+body+'\n}\npub fn after() {}\nfn generic<T: Into<u8>>(t: T) {}\n')
    self.assertIn('pub fn after() {}',production); self.assertNotIn('pub fn after',tests); self.assertIn(body,tests)
  production,tests=run.split_tests("#[cfg(test)]\nfn life<'a>(x: &'a str) -> &'a str { x }\npub fn after() {}\n")
  self.assertIn('pub fn after',production)
 def test_a_cfg_test_initializer_is_no_header(self):
  for item in ('const LIMIT: usize = 1 << 10;','const B: bool = 3 < 4;','static S: [u8; 2] = [1 << 2, 0];'):
   with self.subTest(item=item):
    production,tests=run.split_tests('#[cfg(test)]\n'+item+'\npub fn after() {}\n')
    self.assertIn('pub fn after',production); self.assertIn(item,tests)
  production,tests=run.split_tests('enum E {\n    #[cfg(test)]\n    A = 1 << 3,\n    B,\n}\npub fn after() {}\nconst Q: Option<Vec<u8>> = None;\n')
  self.assertIn('B,',production); self.assertIn('pub fn after',production); self.assertNotIn('B,',tests)
 def test_a_cfg_test_item_counts_alone(self):
  body='#[cfg(test)]\nmod early_tests { }\n'+' '.join('Operation::V'+str(n) for n in range(8))
  self.assertEqual(self.census(crates__values__src__m_DOT_rs=body)['operation_exhaustive_matches']['files'],1)
 def test_a_test_moved_into_src_tests_rs_or_a_cfg_test_module_is_still_a_test(self):
  for path,body in (('crates__core__src__tests_DOT_rs','RelationOwner\n'),('crates__core__src__d_DOT_rs','fn f() {}\n#[cfg(test)]\nmod tests { RelationOwner }\n')):
   with self.subTest(path=path):
    c=self.census(**{path:body})
    self.assertEqual(c['cross_crate_tests_naming_runtime_internals']['files'],2)
class Modes(unittest.TestCase):
 def test_both_read_only_modes_run_to_an_answer(self):
  for mode in ('--check','--census'):
   with self.subTest(mode=mode):
    done=subprocess.run(['python3','-I',str(Path(run.__file__)),mode],capture_output=True,text=True)
    self.assertIn(done.returncode,(0,1),done.stderr); self.assertNotIn('Traceback',done.stderr)
if __name__=='__main__': unittest.main()
