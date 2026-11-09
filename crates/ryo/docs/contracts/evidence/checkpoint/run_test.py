"""Fixture cases of `run.py --check`: a ticked row (a `[x]` box or a `☑` cell) stays ticked only when it cites a test, every test it cites passed in the recorded logs, and it does not say its review is pending."""
import importlib.util,json,subprocess,tempfile,unittest
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
 def test_every_red_result_carries_its_owner_and_a_green_one_carries_none(self):
  readme=''.join('- [ ] '+t+'\n' for t in set(run.OWNERS.values()))
  v={'results':{'core_allocation':{'exit':101},'ruff_rule_phases':{'exit':0,'note':'repaired; owned by "Close full TM, contracts, independent review and normal CI."'},'unowned_target':{'failed':1},'libraries_total':{'passed':1,'failed':1}}}
  changed,bad=run.own(v,readme)
  self.assertTrue(changed); self.assertEqual(bad,['red unowned_target has no owner in OWNERS'])
  self.assertEqual(v['results']['core_allocation']['note'],'owned by "'+run.OWNERS['core_allocation']+'"')
  self.assertEqual(v['results']['ruff_rule_phases']['note'],'repaired')
  self.assertNotIn('note',v['results']['libraries_total'])
def commit(repo,files,message):
 """Writes the files (path -> text, None deletes) into the fixture repository and commits them; returns the sha."""
 for path,body in files.items():
  f=Path(repo)/path
  if body is None: f.unlink(); continue
  f.parent.mkdir(parents=True,exist_ok=True); f.write_text(body)
 subprocess.run(['git','-C',repo,'add','-A'],check=True); subprocess.run(['git','-C',repo,'commit','-q','-m',message],check=True)
 return subprocess.run(['git','-C',repo,'rev-parse','HEAD'],capture_output=True,text=True,check=True).stdout.strip()
RED='test domain::service_test::one_answers ... FAILED\n'
GREEN='test domain::service_test::one_answers ... ok\n'
class Diff(unittest.TestCase):
 """The checkpoint-diff checks: a fix carries its red-then-green, and a published crate's new pub item its examples."""
 def setUp(self):
  self.dir=tempfile.TemporaryDirectory(); self.repo=self.dir.name; self.raw=Path(self.repo)/'raw'; self.raw.mkdir()
  subprocess.run(['git','init','-q','-b','claude/checkpoint-9',self.repo],check=True)
  for k,x in (('user.email','t@example.com'),('user.name','t')): subprocess.run(['git','-C',self.repo,'config',k,x],check=True)
  (Path(self.repo)/'.gitignore').write_text('raw/\n')
  self.base=commit(self.repo,{'a/Cargo.toml':'[package]\nname="a"\n','a/src/lib.rs':'/// One.\n///\n/// # Examples\n/// ```\n/// ```\npub fn one() {}\n','a/src/old.rs':'pub fn kept() {}\n','a/tests/t.rs':'pub fn shadow() {}\n','b/Cargo.toml':'[package]\nname="b"\npublish = false\n','b/src/lib.rs':''},'chore: base')
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
  self.logs(red=RED,green=GREEN,cli_tests='test other::case ... ok\n',flipped=GREEN)
  for text in ('a service_test::one_answers',                                          # no logs
               'a service_test::one_answers (red red.log)',                            # one log only
               'a service_test::one_answers (red red.log, green red.log)',             # the same log twice
               'a service_test::one_answers (red cli_tests.log, green green.log)',     # an unrelated red log
               'a service_test::one_answers (red flipped.log, green green.log)',       # a red log where it passed
               'a service_test::one_answers (red red.log, green gone.log)',            # a log not in the directory
               'a other::case (red red.log, green green.log)'):                       # another test than the message names
   with self.subTest(text=text): self.assertIn('no red_green entry over it names a test',self.fixes([self.entry(sha,text)])[0])
 def test_a_module_path_in_the_message_is_no_proved_test(self):
  sha=commit(self.repo,{'a/src/lib.rs':'pub fn one() { }\n'},'fix(a): guard_shell::builder_check refuses more')
  self.logs(cli_tests='test other::case ... ok\n')
  self.assertEqual(len(self.fixes([self.entry(sha,'guard_shell::builder_check (red cli_tests.log, green cli_tests.log)')])),1)
 def test_a_behaviour_change_of_any_type_but_perf_and_refactor_is_a_fix(self):
  for n,kind in enumerate(('chore','docs','test','style','build')):
   with self.subTest(kind=kind):
    commit(self.repo,{'a/src/lib.rs':'pub fn one() {'+' '*(n+1)+'}\n'},kind+'(a): one')
    self.assertEqual(len(self.fixes([])),n+1)
 def test_docs_tests_refactors_and_perf_are_not_fixes(self):
  for files,message in (({'a/src/lib.rs':'pub fn one() { }\n'},'refactor(a): one'),({'a/src/lib.rs':'pub fn one() {  }\n'},'perf(a): one'),
                        ({'a/src/lib_test.rs':'x'},'fix(a): a test'),({'a/README.md':'x'},'fix(a): docs')):
   commit(self.repo,files,message)
  self.assertEqual(self.fixes([]),[])
 def test_without_a_base_both_checks_refuse(self):
  self.assertEqual(run.fix_problems({'red_green':[]},repo=self.repo),[run.NO_BASE])
  self.assertEqual(run.doc_problems(repo=self.repo),[run.NO_BASE])
 def test_a_new_pub_item_of_a_published_crate_needs_examples_and_errors(self):
  commit(self.repo,{'a/src/lib.rs':'pub fn one() {}\n/// Two.\npub fn two() -> Result<(), E> {}\n/// Three.\n///\n/// # Examples\n/// # Errors\npub fn three() -> std::result::Result<(), E> {}\npub(crate) fn four() {}\nmod seal { pub trait Sealed {} }\n',
                        'a/src/fixtures.rs':'pub fn fixture() {}\n','a/tests/t.rs':'pub fn helper() {}\n','b/src/lib.rs':'pub fn unpublished() {}\n'},'feat(a): two and three')
  self.assertEqual(run.doc_problems(repo=self.repo,base=self.base),['a/src/lib.rs:3 pub fn two: no `# Examples` section','a/src/lib.rs:3 pub fn two: returns a Result with no `# Errors` section'])
 def test_an_item_moved_within_its_crate_is_not_new(self):
  commit(self.repo,{'a/src/lib.rs':'','a/src/moved.rs':'pub fn one() {}\n'},'refactor(a): move one')
  self.assertEqual(run.doc_problems(repo=self.repo,base=self.base),[])
 def test_new_items_that_share_a_name_or_hide_in_a_rename_or_a_test_named_file_are_new(self):
  commit(self.repo,{'a/src/kind.rs':'pub struct Kind;\nimpl Kind {\n    pub fn one() -> Result<(), E> {}\n}\n',      # one is pub at the base, on no type
                    'a/src/shadow.rs':'pub fn shadow() {}\n',                                                   # pub at the base only in a test file
                    'a/src/old.rs':None,'a/src/renamed.rs':'pub fn kept() {}\npub fn added() {}\n',             # a rename that adds an item
                    'a/src/test_support.rs':'pub fn support() {}\n'},'feat(a): kinds')
  found=run.doc_problems(repo=self.repo,base=self.base)
  for item in ('pub struct Kind','pub fn one','pub fn shadow','pub fn added','pub fn support'):
   with self.subTest(item=item): self.assertTrue(any(item+':' in x for x in found),found)
  self.assertFalse(any('pub fn kept:' in x for x in found))
class Census(unittest.TestCase):
 FILES={'crates/execution_runtime/src/lib.rs':'pub struct FileSession;\npub trait DigestPort {}\n','crates/core/src/a.rs':'use x::FileSession;\n',
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
 def test_a_same_named_type_in_another_crates_test_does_not_hide_the_evaluators(self):
  self.assertEqual(self.census(crates__other__tests__x_DOT_rs='pub struct FileSession;\n')['replace_the_evaluator']['total_files'],4)
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
