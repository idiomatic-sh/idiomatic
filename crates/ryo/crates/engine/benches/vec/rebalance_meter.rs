//! Measurement build only. Split each cached computation among its actual downstream readers.
use super::super::{pair_summary::{Program,Need},thread_cpu};
use rustc_hash::{FxHashMap as Map,FxHashSet as Set};
use std::{cell::RefCell,fmt::Write as _};
#[derive(Default)]
struct Node { edges: Vec<usize>, cpu: f64, calls: usize }
#[derive(Default,Clone)]
struct Charge { eval:f64, fill:f64, seen:u64, rejects:u64, saved:f64, removed:u64 }
#[derive(Default)]
struct Ledger {
    work:Map<(u64,u64),(u64,f64)>, family:Vec<String>, fills:Vec<(u64,f64)>, coarse:f64, empty_steps:u64, empty_time:f64, empty_profile:Map<u64,(u64,f64)>,
    stack:Vec<(usize,f64,f64)>, graph:Vec<Node>, costs:Map<(usize,usize),Charge>,
    operations:Vec<String>, units:Map<(usize,usize,usize),(f64,f64)>, source:Map<(usize,usize,usize),f64>, readers:Vec<Vec<(usize,usize)>>,
    seen:Vec<u32>, stamp:u32, rule_names:Vec<String>, fact_names:Vec<String>,
    pending:Vec<((usize,usize,usize),f64,bool)>, second:Set<usize>, solo:Map<(usize,usize,usize),(f64,f64,f64)>,
    groups:Map<usize,String>, eligible:Vec<u64>, allowed:Vec<u64>, empty:Vec<(u64,f64)>,
    last:Map<String,(usize,Option<usize>)>, files:u64, fill_base:usize, base:f64,
}
thread_local! {static LEDGER:RefCell<Ledger>=Default::default();}
fn settle(l:&mut Ledger){
 for (key,cpu,only) in l.pending.drain(..){
  let slot=l.solo.entry(key).or_default();if only{slot.0+=cpu;}if !l.second.contains(&key.2){slot.1+=cpu;if only{slot.2+=cpu}}
 }
 l.second.clear();
}
pub(crate) fn bypass()->bool{static B:std::sync::OnceLock<bool>=std::sync::OnceLock::new();*B.get_or_init(||std::env::var_os("P8_BYPASS").is_some())}
pub(crate) fn rejected(name:&str)->bool {LEDGER.with_borrow(|l|l.last.get(name).is_some_and(|x|x.1.is_some()))}
pub(crate) fn second_read(name:&str){
 if light() || bypass(){return} LEDGER.with_borrow_mut(|l|if let Some(slot)=l.fact_names.iter().position(|s|s==name){l.second.insert(slot);});
}
pub(crate) fn light()->bool{static L:std::sync::OnceLock<bool>=std::sync::OnceLock::new();*L.get_or_init(||std::env::var_os("P8_LIGHT").is_some())}
pub(crate) fn coarse(cpu:f64){LEDGER.with_borrow_mut(|l|l.coarse+=cpu);}
pub(crate) fn fill(slot:usize,cpu:f64){LEDGER.with_borrow_mut(|l|{l.fills[slot].0+=1;l.fills[slot].1+=cpu;});}
pub(crate) fn start(p:&Program) {
 LEDGER.with_borrow_mut(|l|{
  settle(l);if l.rule_names.is_empty(){
   l.rule_names=p.roots.iter().map(|r|r.0.clone()).collect();l.family=p.facts.iter().map(|f|f.family.name().into()).collect();l.fills.resize(p.facts.len(),(0,0.));l.fact_names=p.facts.iter().map(|f|f.name.to_owned()).collect();
   l.eligible.resize(p.roots.len(),0);l.allowed.resize(p.roots.len(),0);l.empty.resize(p.roots.len(),(0,0.));
   l.operations.extend(p.needs.iter().map(|n|format!("file {n:?}")));
   l.operations.extend(p.domains.iter().map(|n|format!("node {n:?}")));
   l.operations.extend(p.facts.iter().map(|f|format!("read {}",f.name)));
   l.operations.extend(p.roots.iter().map(|_|"certified emit".to_owned()));
   let t=thread_cpu(); for _ in 0..1024 {std::hint::black_box(thread_cpu());} l.base=(thread_cpu()-t)/1024.;
  }
  l.files+=1;l.fill_base=p.needs.len()+p.domains.len();l.last.clear();l.stack.clear();if light(){return}
  let n=l.fill_base+p.facts.len()+p.roots.len();l.graph.resize_with(n,Default::default);
  for x in &mut l.graph{x.cpu=0.;x.calls=0;x.edges.clear();}
  l.readers.resize_with(n,Vec::new);for x in &mut l.readers{x.clear()}l.seen.resize(n,0);
 });
}
pub(crate) fn enter(i:usize) {
 if light(){return}
 let outer=thread_cpu();
 LEDGER.with_borrow_mut(|l|{
   if let Some(&(parent,_,_))=l.stack.last(){let e=&mut l.graph[parent].edges;if e.last()!=Some(&i){e.push(i);}}
   l.graph[i].calls+=1;l.stack.push((i,outer,0.));let t=thread_cpu();l.stack.last_mut().unwrap().2=t-outer;
 });
}
pub(crate) fn leave(){
 if light(){return}
 let stop=thread_cpu();
 LEDGER.with_borrow_mut(|l|{
  let (i,start,child)=l.stack.pop().unwrap();l.graph[i].cpu+=(stop-start-child-l.base).max(0.);
  if let Some(parent)=l.stack.last_mut(){parent.2+=thread_cpu()-start+l.base;}
 });
}
pub(crate) fn link(i:usize){if light(){return}LEDGER.with_borrow_mut(|l|if let Some(&(p,_,_))=l.stack.last(){l.graph[p].edges.push(i);});}
fn groups(p:&Program,id:usize,values:&[Option<bool>],out:&mut Vec<usize>){
 match &p.needs[id] {
  Need::Both(a,b)=>{groups(p,*a,values,out);if values[*a]==Some(true){groups(p,*b,values,out);}}
  Need::Terms(true,terms)=>for &i in terms{groups(p,i,values,out);if values[i]!=Some(true){break;}},
  _=>out.push(id)
 }
}
pub(crate) fn finish(p:&Program,before:&[bool],alive:&mut [bool],values:&[Option<bool>],bypass:bool){
 if light(){if bypass{LEDGER.with_borrow_mut(|l|for (i,(name,_)) in p.roots.iter().enumerate(){if before[i]{l.last.insert(name.clone(),(i,(!alive[i]).then_some(0)));alive[i]=true;}});}return}
 LEDGER.with_borrow_mut(|l|{
  let split=l.fill_base;let exact=split+p.facts.len();let mut gs=Vec::new();let mut todo=Vec::new();
  for (rule,(name,root)) in p.roots.iter().enumerate(){
   if !before[rule]{continue} l.eligible[rule]+=1;l.allowed[rule]+=u64::from(alive[rule]);
   gs.clear();groups(p,*root,values,&mut gs);let failed=gs.iter().copied().find(|&i|values[i]==Some(false));
   assert_eq!(failed.is_some(),!alive[rule]);l.last.insert(name.clone(),(rule,failed));
   if l.graph[exact+rule].calls>0{gs.push(exact+rule);}
   for &g in &gs {
    l.groups.entry(g).or_insert_with(||if g>=exact{"exact completion".to_owned()}else{match &p.needs[g]{Need::Some(d)=>format!("Some {:?}",p.domains[*d]),n=>format!("{n:?}")}});
    let z=l.costs.entry((rule,g)).or_default();z.seen+=1;z.rejects+=u64::from(Some(g)==failed);
    l.stamp+=1;todo.push(g);
    while let Some(i)=todo.pop(){if l.seen[i]==l.stamp{continue}l.seen[i]=l.stamp;l.readers[i].push((rule,g));todo.extend(l.graph[i].edges.iter().copied());}
   }
   // The connective's exclusive dispatch belongs to the rule as well.
   if !gs.contains(root){
    l.stamp+=1;todo.push(*root);
    while let Some(i)=todo.pop(){if l.seen[i]==l.stamp || gs.contains(&i){continue}l.seen[i]=l.stamp;
      let dispatch=usize::MAX;l.readers[i].push((rule,dispatch));l.groups.entry(dispatch).or_insert_with(||"conjunction dispatch".into());todo.extend(l.graph[i].edges.iter().copied());
    }
   }
   if bypass {alive[rule]=true;}
  }
  for (i,rs) in l.readers.iter_mut().enumerate(){
   if rs.is_empty(){continue}rs.sort_unstable();rs.dedup();
   let mut count=0;let mut prev=None;
   for &(rule,_) in rs.iter(){if prev!=Some(rule){count+=1;prev=Some(rule);}}
   let per=l.graph[i].cpu/count as f64;
   let mut j=0;while j<rs.len(){let r=rs[j].0;let k=j+rs[j..].partition_point(|e|e.0==r);let v=per/(k-j) as f64;
    for &key in &rs[j..k]{let unit=l.units.entry((key.0,key.1,i)).or_default();unit.0+=v;unit.1+=l.graph[i].calls as f64/count as f64/(k-j) as f64;let charge=l.costs.entry(key).or_default();
     if (split..exact).contains(&i){charge.fill+=v;l.pending.push(((key.0,key.1,i-split),v,count==1));*l.source.entry((key.0,key.1,i-split)).or_default()+=v;}else{charge.eval+=v;}
    }j=k;
   }
  }
 });
}
pub(crate) fn ask(name:&str,empty:bool,cpu:f64,steps:u64,rows:u64){
 static CAL:std::sync::OnceLock<bool>=std::sync::OnceLock::new();let calibrate=*CAL.get_or_init(||std::env::var_os("P8_WORK_SAMPLES").is_some());
 LEDGER.with_borrow_mut(|l|{if empty{if calibrate{let v=l.work.entry((steps,rows)).or_default();v.0+=1;v.1+=cpu;}l.empty_steps+=steps;l.empty_time+=cpu;let profile=l.empty_profile.entry(steps).or_default();profile.0+=1;profile.1+=cpu;}if let Some(&(r,g))=l.last.get(name){
  if let Some(g)=g{assert!(empty,"productive rejected candidate {name}");if light(){return}let z=l.costs.entry((r,g)).or_default();z.saved+=cpu;z.removed+=1;}
  else if empty{l.empty[r].0+=1;l.empty[r].1+=cpu;}
 }});
}
pub(crate) fn report(){
 LEDGER.with_borrow_mut(|l|{settle(l);let mut o=String::new();
  writeln!(o,"C\t{}\t{:.9}\t{}\t{:.9}",l.files,l.coarse,l.empty_steps,l.empty_time).unwrap();
  for (&(steps,rows),&(n,cpu)) in &l.work{writeln!(o,"Q\t{steps}\t{rows}\t{n}\t{cpu:.9}").unwrap();}
  for (&steps,&(n,cpu)) in &l.empty_profile{writeln!(o,"E\t{steps}\t{n}\t{cpu:.9}").unwrap();}
  for (i,(n,cpu)) in l.fills.iter().enumerate(){writeln!(o,"T\t{}\t{}\t{n}\t{cpu:.9}",l.fact_names[i],l.family[i]).unwrap();}
  for (i,name) in l.rule_names.iter().enumerate(){writeln!(o,"R\t{name}\t{}\t{}\t{}\t{}\t{:.9}",l.files,l.eligible[i],l.allowed[i],l.empty[i].0,l.empty[i].1).unwrap();}
  for (&(r,g),v) in &l.costs{writeln!(o,"G\t{}\t{g}\t{}\t{}\t{}\t{:.9}\t{:.9}\t{:.9}\t{}",l.rule_names[r],v.seen,v.rejects,v.removed,v.eval,v.fill,v.saved,l.groups[&g]).unwrap();}
  for (&(r,g,f),&cpu) in &l.source{let (sole,first,exclusive)=l.solo[&(r,g,f)];writeln!(o,"F\t{}\t{g}\t{}\t{cpu:.9}\t{sole:.9}\t{first:.9}\t{exclusive:.9}",l.rule_names[r],l.fact_names[f]).unwrap();}
  for (&(r,g,i),&(cpu,calls)) in &l.units{writeln!(o,"U\t{}\t{g}\t{i}\t{calls:.4}\t{cpu:.9}\t{}",l.rule_names[r],l.operations[i]).unwrap();}
  if !l.rule_names.is_empty(){std::fs::write(std::env::var("P8_CAPTURE").unwrap(),o).unwrap();}
 });
}
