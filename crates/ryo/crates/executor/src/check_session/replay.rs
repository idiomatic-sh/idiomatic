//! The corpus runner's TSV is a list of invocations, not an execution schedule.
//! Only the final comparison has access to a reference digest.
pub(crate) mod readers;
pub mod stored;

use super::hosted::{HostCell, Hosted, Parameters, bind_tables, execute_ids, read_parameters};
use crate::check_session::{query::Query, report_slots::ReportSlots};
use crate::check_session::project::{self, Authority, Population};
use crate::{
    check_session::{self as session, Answer, Clock, Program},
    finish, request_input, thread_cpu,
};
use crate::languages as l;
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use serde::Deserialize;
use sha2::{Digest, Sha256};
pub(crate) mod batch;
pub use batch::*;
pub(crate) mod closed_workers;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

pub(crate) fn read_paths<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Rc<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(input).map(Rc::new)
}

pub(crate) fn read_inventory<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<Rc<Vec<String>>>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(input)?.map(canonical_inventory))
}

pub(crate) fn canonical_inventory(mut paths: Vec<String>) -> Rc<Vec<String>> {
    paths.sort_unstable();
    paths.dedup();
    Rc::new(paths)
}


#[derive(Default, Deserialize)]
pub(crate) struct Authorities {
    #[serde(default)]
    pub(crate) roots: HashMap<String, Authority>,
    pub(crate) default: Option<Authority>,
}

impl Authorities {
    pub(crate) fn from_input() -> Result<Self, String> {
        match std::env::var_os("TM_CORPUS_PROJECTS") {
            Some(path) => request_input::decode(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Default)]
pub(crate) struct ReplayInputs {
    pub(crate) file_sets: HashSet<Rc<Vec<String>>>,
    pub(crate) queries: HashMap<String, Rc<str>>,
    pub(crate) inventories: HashMap<PathBuf, Rc<Vec<String>>>,
    pub(crate) roots: HashMap<String, PathBuf>,
    pub(crate) authorities: Authorities,
}

impl ReplayInputs {
    pub(crate) fn register<L: session::source::Source>(&mut self, batch: &mut Batch, pack: &str, mut req: Input, closed: Option<&Path>, layouts: Option<&[L]>) -> Result<Work, crate::check_session::refusal::Refusal> {
        let t = thread_cpu();
        let query = match self.queries.get(pack) {
            Some(q) => q.clone(),
            None => {
                let q: Rc<str> =
                    std::fs::read_to_string(Path::new(pack).join("rules/query/query.ryoql"))
                        .map_err(|e| e.to_string())?
                        .into();
                self.queries.insert(pack.to_owned(), q.clone());
                q
            }
        };
        batch.clocks.prepare += thread_cpu() - t;
        if req.repository.is_none() {
            req.repository = self.authorities.roots.get(&req.root).or(self.authorities.default.as_ref()).cloned();
        }
        let mut work = batch.register_query(query.into(), req, |root| self.inventory(root, closed), None, layouts.map(|readers| (readers, None)))?;
        if let Work::Closed { files, .. } = &mut work {
            if let Some(existing) = self.file_sets.get(files.as_ref()) {
                files.clone_from(existing);
            } else {
                self.file_sets.insert(files.clone());
            }
        }
        Ok(work)
    }

    pub fn inventory(&mut self, root: &str, closed: Option<&Path>) -> Result<Option<Rc<Vec<String>>>, String> {
        let closed = closed.or_else(|| self.roots.get(root).map(PathBuf::as_path));
        Ok(if let Some(path) = closed {
            if !self.inventories.contains_key(path) {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                self.inventories.insert(
                    path.to_owned(),
                    Rc::new({
                        let mut paths: Vec<String> =
                            request_input::decode(&bytes).map_err(|e| e.to_string())?;
                        paths.sort_unstable();
                        paths.dedup();
                        paths
                    }),
                );
            }
            self.inventories.get(path).cloned()
        } else {
            None
        })
    }
}

pub fn digest(rows: &[&finish::answer::Row]) -> String {
    let mut hash = Sha256::new();
    if rows.is_empty() { hash.update(b"\n"); }
    finish::json_output::ordered_rows(rows.iter().copied(), |line| {
        hash.update(line);
        hash.update(b"\n");
    });
    format!("{:x}", hash.finalize())
}

pub(crate) fn write_receipt<U: serde::Serialize>(out: &mut impl Write, id: &str, expected: &str, digest: &str, unknowns: &[U]) {
    let status = if expected == digest && unknowns.is_empty() { "exact" } else { "different" };
    write!(out, "{id}\t{status}\t{digest}").unwrap();
    if !unknowns.is_empty() {
        write!(out, "\t").unwrap();
        serde_json::to_writer(&mut *out, unknowns).expect("write unknown findings");
    }
    writeln!(out).unwrap();
}
pub fn batch_mode<L:session::source::Source>(readers:&[L],manifest:&str,output:&str,root:Option<&Path>){
    batch_mode_with_workers(readers, manifest, output, root, None, || {})
}
pub fn batch_mode_with_workers<L:session::source::Source>(readers:&[L], manifest:&str, output:&str, root:Option<&Path>, workers:Option<std::num::NonZeroUsize>, completed: impl FnOnce()){
    let start = thread_cpu();
    let mut session = Batch { workers, file_catalog: session::named::Catalog::from_readers(readers).expect("linked file catalog"), ..Default::default() };
    let mut replay = ReplayInputs { authorities: Authorities::from_input().expect("repository declarations"), ..Default::default() };
    if let Some(index) = std::env::var_os("TM_CORPUS_INVENTORIES") {
        let bytes = std::fs::read(index).expect("closed input inventories");
        replay.roots = request_input::decode(&bytes).expect("source roots and their inventories");
    }
    let mut store=root.map(|p|stored::Store::open(p, readers).expect("open request store"));
    let authority=store.as_mut().map(|s| {
        ["TM_CORPUS_INVENTORIES","TM_CORPUS_PROJECTS"].map(|name|std::env::var_os(name).map(|p|
            s.contract_file(Path::new(&p)).expect("read declared session inputs")))
    });
    enum Entry { Stored(stored::Key), Evaluate(usize,Option<stored::Key>) }
    let mut entries=Vec::new();
    let mut inherited_inventories=Vec::new();
    let file = std::fs::File::open(manifest).expect("input manifest");
    let mut invocations = Vec::new();
    let mut expected_text = std::collections::HashSet::<Rc<str>>::new();
    let mut inputs = HashMap::new();
    for l in std::io::BufReader::new(file).lines() {
        let line = l.expect("manifest line");
        let mut fields = line.split('\t');
        let (id, pack, req, oracle) = (
            fields.next().unwrap(),
            fields.next().unwrap(),
            fields.next().unwrap(),
            fields.next().unwrap(),
        );
        let closed = fields.next().map(Path::new);
        let key = (
            PathBuf::from(pack),
            PathBuf::from(req),
            closed.map(Path::to_owned),
        );
        let idx = *inputs.entry(key).or_insert_with(|| {
            let contract=store.as_mut().and_then(|s| {
                let input=s.contract_file(Path::new(req))?;
                let query=s.contract_file(&Path::new(pack).join("rules/query/query.ryoql"))?;
                let inventory=match closed{Some(p)=>Some(s.contract_file(p)?),None=>None};
                Some(stored::key((input,query,inventory,authority)))
            });
            if let (Some(s),Some(k))=(&mut store,contract) && let Some((answer,_))=s.lookup(k) {
                entries.push(Entry::Stored(answer));
            }else {
                let result=std::fs::read(req).map_err(|e|crate::check_session::refusal::Refusal::from(e.to_string()))
                    .and_then(|bytes|request_input::decode::<Input>(&bytes).map_err(|e|crate::check_session::refusal::Refusal::from(e.to_string())))
                    .and_then(|input|{
                        let inherited=if closed.is_none() && store.is_some(){replay.roots.get(&input.root).cloned()}else{None};
                        let work=replay.register(&mut session,pack,input,closed,store.is_none().then_some(readers))?;
                        if let (Some(p),Some(s))=(inherited,&mut store) && matches!(work,Work::Closed{..}|Work::Project(_)){
                            let expected=s.contract_file(&p);
                            inherited_inventories.push((session.work.len(),p,expected));
                        }
                        Ok(work)
                    }).unwrap_or_else(Work::Refused);
                let idx = session.add_work(result, store.as_mut().zip(contract));
                if store.is_none() {
                    return idx;
                }
                entries.push(Entry::Evaluate(idx,contract));
            }
            entries.len()-1
        });
        // Expected receipt text is opaque until the completed results are compared.
        let expected = if let Some(existing) = expected_text.get(oracle) {
            existing.clone()
        } else {
            let text = Rc::<str>::from(oracle);
            expected_text.insert(text.clone());
            text
        };
        invocations.push((idx, Box::<str>::from(id), expected));
    }
    drop(inputs);
    drop(replay);
    drop(expected_text);
    let invocations = invocations.into_boxed_slice();
    let registration = thread_cpu() - start - session.clocks.prepare - session.clocks.project;
    let answers=if let Some(s)=&mut store {
        let mut state=Active::default();
        if !session.work.is_empty(){session.epoch_with_store(readers,&mut state,Some(s));}
        for (i,path,expected) in inherited_inventories {s.inventory(i,path,expected);}
        state.answers
    }else{session.run_sources(readers)};
    let t=thread_cpu();
    let mut out=std::io::BufWriter::new(std::fs::File::create(output).unwrap());
    enum Output<'a> { Packet(stored::Key), Computed(String, Vec<&'a (String,String)>) }
    let results:Vec<Result<Output,String>>=entries.into_iter().chain(
        (0..if store.is_none() { session.work.len() } else { 0 }).map(|i| Entry::Evaluate(i,None))
    ).map(|entry|{
        let packet=match entry {
            Entry::Stored(k)=>k,
            Entry::Evaluate(i,id)=>{
                if let Work::Invalid(reason)=&session.work[i] {
                    if let (Some(s),Some(k))=(&mut store,id){s.decline(k,reason);}
                    return Err(reason.to_string())
                }
                let r=Batch::answer(&session.work[i],&answers).map_err(std::borrow::Cow::into_owned)?;
                if let (Some(s),Some(k))=(&mut store,id) && let Some(k)=s.save(k,i,&r){k}
                else{return Ok(Output::Computed(digest(&r.rows),r.unknowns))}
            }
        };
        Ok(Output::Packet(packet))
    }).collect();
    if let Some(s)=&mut store{
        s.publish().expect("publish session snapshot");
    }
    let mut refusals = BTreeMap::new();
    for (i, id, expected) in invocations {
        let refusal = match &results[i] {
            Ok(Output::Packet(k)) => {
                let packet = store.as_ref().unwrap().packet(k);
                if let Some(reason) = &packet.refusal { reason.to_string() } else {
                    write_receipt(&mut out, &id, &expected, &packet.digest, &packet.unknowns);
                    continue;
                }
            }
            Ok(Output::Computed(d, u)) => {
                write_receipt(&mut out, &id, &expected, d, u);
                continue;
            }
            Err(reason) => reason.clone(),
        };
        *refusals.entry(refusal.as_str().to_owned()).or_insert(0usize) += 1;
        writeln!(out, "{id}\trefused\t").unwrap();
    }
    out.flush().unwrap();
    for (reason, n) in refusals {
        println!("REFUSAL\t{n}\t{reason}");
    }
    let c = &mut session.clocks;
    c.assemble += thread_cpu() - t;
    println!(
        "CHECK_SESSION programs={} bindings={} sources={} reads={} parses={} skipped={} evaluated={} productive={} empty={} refused={} prep={:.6} read_parse={:.6} first={:.6} second={:.6} project={:.6} assembly={:.6} admission={:.6} bound_proofs={:.6} literal_positions={} certified_pairs={}",
        session.schedule.programs.len(),
        session.bindings.len(),
        session.schedule.sources.len(),
        c.reads,
        c.files,
        c.skipped,
        c.evaluated, c.productive, c.empty, c.refused,
        c.prepare,
        c.read_parse,
        c.screen,
        c.evaluate,
        c.project,
        c.assemble,
        registration,
        c.bound_proofs,
        c.literal_positions,
        c.certified,
    );
    completed();
}

pub fn one_source<L: session::source::Source>(
    readers: &[L],
    pack: &str,
    req: Input,
    output: &str,
) -> Result<(), crate::check_session::refusal::Refusal> {
    let mut session = Batch::default();
    session.file_catalog = session::named::Catalog::from_readers(readers)?;
    let work = ReplayInputs::default().register(&mut session, pack, req, None, Some(readers))?;
    session.work.push(work);
    let answers = session.run_sources(readers);
    let rows = Batch::answer(&session.work[0], &answers).map_err(std::borrow::Cow::into_owned)?;
    if !rows.unknowns.is_empty() {
        return Err("partial answer requires typed unknown output".into());
    }
    let mut stream = Vec::new();
    if rows.rows.is_empty() { stream.push(b'\n'); }
    finish::json_output::ordered_rows(rows.rows, |line| {
        stream.extend_from_slice(line);
        stream.push(b'\n');
    });
    std::fs::write(output, stream).map_err(|e| e.to_string().into())
}
