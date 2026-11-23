"""Construct the two comparison arms only on the immutable round-58 control source."""
from pathlib import Path
import sys,subprocess,shutil
tools=Path(__file__).resolve().parent
root=Path('/tmp/ryo-pass8-round58')
E=Path('crates/ryo/crates/engine/benches/vec')
audit='--check' in sys.argv
def source(name):
    return subprocess.check_output(['git','-C',root,'show',str('HEAD:'+str(E/name))],text=True)
def one(text,old,new):
    if text.count(old)!=1:raise ValueError((old,text.count(old)))
    return text.replace(old,new)
def emit(name,text):
    if not audit:(root/E/name).write_text(text)
if subprocess.check_output(['git','-C',root,'rev-parse','HEAD'],text=True).strip()!='dfbd06f1f05d09b940e5404ef47c6868a505a728':
    raise RuntimeError('wrong comparison source')
if not audit:
    assert not subprocess.check_output(['git','-C',root,'status','--porcelain'])
s=source('pair_summary.rs')
a='#[path = "pair_compact.rs"]'
s=one(s,a,'''#[path = "rebalance_choose.rs"] mod choose;
#[path = "rebalance_prices.rs"] mod prices;
#[path = "rebalance_frontier.rs"] mod frontier;
#[path = "rebalance_feedback.rs"] pub(crate) mod feedback;
#[path = "rebalance_learning.rs"] pub(crate) mod learning;
'''+a)
s=one(s,'pub(super) struct Program {','''pub(super) struct Program {
    pub(crate) learning: Option<learning::Shared>,
    pub(crate) file_fees: Vec<u64>,
    pub(crate) node_fees: Vec<u64>,''')
s=one(s,'        out: Program {','''        out: Program {
            learning: None, file_fees: vec![], node_fees: vec![],''')
s=one(s,'    cost::arrange(&mut b.out, omit);','    cost::arrange(&mut b.out, omit);\n    choose::prepare(&mut b.out, plan);')
emit('pair_summary.rs',s)
s=source('pair_checks.rs')
a='use super::pair_summary::cost::{self, Sample};'
s=one(s,a,'''#[path = "rebalance_readers.rs"] pub(crate) mod readers;
use super::pair_summary::feedback;
'''+a)
s=one(s,"struct File<'a, const PROFILE: bool> {","struct File<'a, const PROFILE: bool> {\n    collecting: bool,\n    paid: u64,")
a='        let answered = read(spec.source, words, &mut self.cells);'
s=one(s,a,'        let price_start=readers::start();\n'+a+'\n        readers::provider(spec,price_start);')
a='    fn pass_miss(&mut self, id: Id) -> bool {'
s=one(s,a,a+'\n        if self.collecting { self.paid+=self.p.file_fees[id]; }')
a='    fn nodes_miss(&mut self, id: Id) -> Nodes {'
s=one(s,a,a+'\n        if self.collecting { self.paid+=self.p.node_fees[id]; }')
a='        languages::facts::clear_words(words);'
s=one(s,a,'''        let mut training=self.learning.as_ref().map(|book| book.borrow_mut()).filter(|book| book.active);
        if training.is_some() { feedback::send(self.learning.as_ref().unwrap(),path); }
        let collect_supplier=std::cell::Cell::new(false);
        let supplier_work=std::cell::Cell::new(0u64);
'''+a)
a='        let mut observe = |source, words: &mut FileWords, cells: &mut FilledColumns| {'
s=one(s,a,a+'''
            if collect_supplier.get() {
                let fact=self.facts.iter().find(|f| f.source==source).expect("declared provider source");
                supplier_work.set(supplier_work.get()+feedback::supplier(fact.family));
            }''')
s=one(s,'        let mut f = File::<PROFILE> {','        let mut f = File::<PROFILE> {\n            collecting:false, paid:0,')
a='        for (rule, (a, (_, root))) in alive.iter_mut().zip(&self.roots).enumerate() {'
s=one(s,a,'        let checks_start=readers::start();\n'+a)
s=one(s,'                *a = f.pass(*root)','''                *a = if let Some(book)=&mut training {
                    book.rules[rule].admits(*root,|id,record| {
                        if !record { return (f.pass(id),0,0.0); }
                        let before=readers::training_clock();
                        f.collecting=true; f.paid=0;
                        collect_supplier.set(true);supplier_work.set(0);
                        let yes=f.pass(id);
                        collect_supplier.set(false); f.collecting=false;
                        (yes,feedback::work(f.paid,supplier_work.get()),readers::elapsed(before))
                    })
                } else { f.pass(*root) }''')
s=one(s,'        f.steps\n','        readers::file(checks_start);\n        f.steps\n')
emit('pair_checks.rs',s)
s=source('finisher.rs')
s=one(s,'    rule_stats: Vec<[u64; 3]>,','    price_feedback: Option<super::pair_summary::learning::Shared>,\n    rule_stats: Vec<[u64; 3]>,')
s=one(s,'            rule_stats: vec!', '            price_feedback:None,\n            rule_stats: vec!')
a='        self.files_run += 1;'
s=one(s,a,a+'''
        let lease=super::pair_summary::feedback::receive(&self.run.paths[file_no],self.run.paths.len());
        if let Some(current)=&lease { self.price_feedback=Some(current.clone()); }
        let mut learned=lease.as_ref().map(|r|r.borrow_mut());''')
a='            let got = self.exec.eval(&mut f, run, pv, e.1);'
s=one(s,a,'''            let clock=if learned.is_some() { super::pair_checks::readers::training_clock() } else { 0. };
'''+a+'''
            let empty=got.as_ref().is_ok_and(|t| t.n==0);''')
a='                self.failed_idx[*ei] = true;\n            }'
s=one(s,a,'''                self.failed_idx[*ei] = true;
            } else if empty && let Some(book)=&mut learned && let Some(r)=book.names.get(e.0.as_str()).copied() {
                book.rules[r].observed_empty(self.exec.stats.total_runs-before.0,self.exec.stats.total_rows-before.1,
                                             super::pair_checks::readers::elapsed(clock));
            }''')
a='        let ok = self.failed.is_empty();'
s=one(s,a,'''        if super::pair_checks::readers::enabled() {
            super::pair_checks::readers::report(self.price_feedback.as_ref().map(|book|book.borrow().diagnostics()));
        }
'''+a)
emit('finisher.rs',s)
for name in ('choose','feedback','learning','frontier','readers'):
    emit(f'rebalance_{name}.rs',(tools/f'p8-{name}.rs').read_text())
if not audit:
    shutil.copyfile(tools.parent/'runs/pass8/neutral-prices.rs',root/E/'rebalance_prices.rs')
print('verified trial hooks' if audit else 'installed current-price trial')

