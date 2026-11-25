//! The corpus runner's TSV is a list of invocations, not an execution schedule.
//! Only the final comparison has access to a reference digest.
#[path = "../../../ryo/crates/engine/benches/vec/check_session/requests.rs"]
mod readers;
#[path = "../../../ryo/crates/engine/benches/vec/check_session/stored.rs"]
mod stored;

use super::{Hosted, Parameters, bind_tables, execute_ids, finish, read_parameters};
use crate::check_session::{query::Query, report_slots::ReportSlots};
use crate::check_session::project::{self, Authority, Population};
use crate::{
    check_session::{self as session, Answer, Clock, Program},
    thread_cpu,
};
use ryo::languages as l;
use ryo::{
    languages::{Language, Recognition},
    linter::{Languages, RelativePath},
};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use serde::Deserialize;
use crate::{json_input, json_output};
use sha2::{Digest, Sha256};
#[path = "../../../ryo/crates/engine/benches/vec/check_session/closed_workers.rs"]
mod closed_workers;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

fn read_paths<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Rc<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(input).map(Rc::new)
}

fn read_inventory<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<Rc<Vec<String>>>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(input)?.map(canonical_inventory))
}

fn canonical_inventory(mut paths: Vec<String>) -> Rc<Vec<String>> {
    paths.sort_unstable();
    paths.dedup();
    Rc::new(paths)
}

include!("../../../ryo/crates/engine/benches/vec/check_session/batch.rs");

#[derive(Default, Deserialize)]
struct Authorities {
    #[serde(default)]
    roots: HashMap<String, Authority>,
    default: Option<Authority>,
}

impl Authorities {
    fn from_input() -> Result<Self, String> {
        match std::env::var_os("TM_CORPUS_PROJECTS") {
            Some(path) => json_input::decode(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Default)]
struct ReplayInputs {
    file_sets: HashSet<Rc<Vec<String>>>,
    shared_files: [usize; 2],
    queries: HashMap<String, Rc<str>>,
    inventories: HashMap<PathBuf, Rc<Vec<String>>>,
    roots: HashMap<String, PathBuf>,
    authorities: Authorities,
}

impl ReplayInputs {
    fn register(&mut self, batch: &mut Batch, pack: &str, mut req: Input, closed: Option<&Path>) -> Result<Work, String> {
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
        let mut work = batch.register_query(query.into(), req, |root| self.inventory(root, closed), None)?;
        if let Work::Closed { files, .. } = &mut work {
            if let Some(existing) = self.file_sets.get(files.as_ref()) {
                self.shared_files[0] += 1;
                self.shared_files[1] += files.iter().map(String::len).sum::<usize>();
                files.clone_from(existing);
            } else {
                self.file_sets.insert(files.clone());
            }
        }
        Ok(work)
    }

    fn inventory(&mut self, root: &str, closed: Option<&Path>) -> Result<Option<Rc<Vec<String>>>, String> {
        let closed = closed.or_else(|| self.roots.get(root).map(PathBuf::as_path));
        Ok(if let Some(path) = closed {
            if !self.inventories.contains_key(path) {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                self.inventories.insert(
                    path.to_owned(),
                    Rc::new({
                        let mut paths: Vec<String> =
                            json_input::decode(&bytes).map_err(|e| e.to_string())?;
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

pub(super) fn digest(rows: &[&finish::answer::Row]) -> String {
    let mut hash = Sha256::new();
    if rows.is_empty() { hash.update(b"\n"); }
    finish::json_output::ordered_rows(rows.iter().copied(), |line| {
        hash.update(line);
        hash.update(b"\n");
    });
    format!("{:x}", hash.finalize())
}

/// The committed benchmark passes the numbered inventory on stdin and source
/// roots in its adjacent manifest. Both streams describe inputs to one session.
pub(crate) fn packets(
    languages: Vec<Language>,
    manifest: &str,
    output: &str,
) -> Result<(), String> {
    #[derive(Deserialize)]
    struct Entry {
        id: String,
        pack: String,
        request: PathBuf,
    }
    #[derive(Deserialize)]
    struct Inventory {
        id: String,
        inventory: Vec<String>,
        repository: Option<Authority>,
    }
    let readers: Vec<_> = languages.into_iter().map(session::source::Linked::from).collect();
    let mut session = Batch { file_catalog: session::named::Catalog::from_readers(&readers)?, ..Default::default() };
    let mut inputs = ReplayInputs { authorities: Authorities::from_input()?, ..Default::default() };
    let mut ids = Vec::new();
    let source = std::fs::File::open(manifest).map_err(|e| e.to_string())?;
    let stdin = std::io::stdin();
    let mut projects = stdin.lock().lines();
    for line in std::io::BufReader::new(source).lines() {
        let item: Entry =
            json_input::decode_line(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mapped = projects
            .next()
            .ok_or("missing closed inventory")?
            .map_err(|e| e.to_string())?;
        let project: Inventory = json_input::decode_line(&mapped).map_err(|e| e.to_string())?;
        if project.id != item.id {
            return Err("inventory and admitted request differ".into());
        }
        let bytes = std::fs::read(item.request).map_err(|e| e.to_string())?;
        let mut input: Input = json_input::decode(&bytes).map_err(|e| e.to_string())?;
        input.inventory = Some(canonical_inventory(project.inventory));
        input.repository = project.repository;
        let work = inputs.register(&mut session, &item.pack, input, None)
            .unwrap_or_else(Work::Refused);
        session.work.push(work);
        ids.push(item.id);
    }
    if projects.next().is_some() {
        return Err("extra closed inventory".into());
    }
    drop(inputs);
    let answers = session.run_sources(&readers);
    let mut out =
        std::io::BufWriter::new(std::fs::File::create(output).map_err(|e| e.to_string())?);
    for (id, work) in ids.into_iter().zip(&session.work) {
        let result = match Batch::answer(work, &answers) {
            Ok(r) => {
                json_output::packet_receipt(&id, Ok((r.rows.len(), digest(&r.rows), r.unknowns)))
            }
            Err(reason) => json_output::packet_receipt(&id, Err(reason)),
        };
        writeln!(out, "{result}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

pub(crate) fn batch(languages: Vec<Language>, manifest: &str, output: &str) {
    let readers: Vec<_> = languages
        .into_iter()
        .map(session::source::Linked::from)
        .collect();
    batch_with_sources(&readers, manifest, output);
}

fn batch_with_sources<L: session::source::Source>(readers: &[L], manifest: &str, output: &str) {
    batch_with_workers(readers, manifest, output, None);
}
fn batch_with_workers<L: session::source::Source>(
    readers: &[L],
    manifest: &str,
    output: &str,
    workers: Option<std::num::NonZeroUsize>,
) {
    batch_mode_with_workers(readers, manifest, output, None, workers, || {})
}
fn write_receipt<U: serde::Serialize>(out: &mut impl Write, id: &str, expected: &str, digest: &str, unknowns: &[U]) {
    let status = if expected == digest && unknowns.is_empty() { "exact" } else { "different" };
    write!(out, "{id}\t{status}\t{digest}").unwrap();
    if !unknowns.is_empty() {
        write!(out, "\t").unwrap();
        serde_json::to_writer(&mut *out, unknowns).expect("write unknown findings");
    }
    writeln!(out).unwrap();
}
fn batch_mode<L:session::source::Source>(readers:&[L],manifest:&str,output:&str,root:Option<&Path>){
    batch_mode_with_workers(readers, manifest, output, root, None, || {})
}
fn batch_mode_with_workers<L:session::source::Source>(readers:&[L], manifest:&str, output:&str, root:Option<&Path>, workers:Option<std::num::NonZeroUsize>, completed: impl FnOnce()){
    let start = thread_cpu();
    let mut session = Batch { workers, file_catalog: session::named::Catalog::from_readers(readers).expect("linked file catalog"), ..Default::default() };
    let mut replay = ReplayInputs { authorities: Authorities::from_input().expect("repository declarations"), ..Default::default() };
    if let Some(index) = std::env::var_os("TM_CORPUS_INVENTORIES") {
        let bytes = std::fs::read(index).expect("closed input inventories");
        replay.roots = json_input::decode(&bytes).expect("source roots and their inventories");
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
                let result=std::fs::read(req).map_err(|e|e.to_string())
                    .and_then(|bytes|json_input::decode::<Input>(&bytes).map_err(|e|e.to_string()))
                    .and_then(|input|{
                        let inherited=if closed.is_none() && store.is_some(){replay.roots.get(&input.root).cloned()}else{None};
                        let work=replay.register(&mut session,pack,input,closed)?;
                        if let (Some(p),Some(s))=(inherited,&mut store) && matches!(work,Work::Closed{..}|Work::Project(_)){
                            let expected=s.contract_file(&p);
                            inherited_inventories.push((session.work.len(),p,expected));
                        }
                        Ok(work)
                    }).unwrap_or_else(Work::Refused);
                let idx = session.work.len();
                session.work.push(result);
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
    let [lists, path_bytes] = replay.shared_files;
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
                    return Err(reason.clone())
                }
                let r=Batch::answer(&session.work[i],&answers).map_err(str::to_owned)?;
                if let (Some(s),Some(k))=(&mut store,id) && let Some(k)=s.save(k,i,&r){k}
                else{return Ok(Output::Computed(digest(&r.rows),r.unknowns))}
            }
        };
        Ok(Output::Packet(packet))
    }).collect();
    if let Some(s)=&mut store{
        let pub_start=thread_cpu();
        let written=s.publish().expect("publish session snapshot");
        eprintln!("publication_cpu={:.6}",thread_cpu()-pub_start);s.costs();
        eprintln!("retained request_hits={} pair_hits={} closed_hits={} evaluated_pairs={} evaluated_closed={} written={written}",s.request_hits,s.file_hits,s.closed_hits,s.evaluated_pairs,s.evaluated_closed);
    }
    let mut refusals = BTreeMap::new();
    for (i, id, expected) in invocations {
        let refusal = match &results[i] {
            Ok(Output::Packet(k)) => {
                let packet = store.as_ref().unwrap().packet(k);
                if let Some(reason) = &packet.refusal { reason } else {
                    write_receipt(&mut out, &id, &expected, &packet.digest, &packet.unknowns);
                    continue;
                }
            }
            Ok(Output::Computed(d, u)) => {
                write_receipt(&mut out, &id, &expected, d, u);
                continue;
            }
            Err(reason) => reason,
        };
        *refusals.entry(refusal.as_str()).or_insert(0usize) += 1;
        writeln!(out, "{id}\trefused\t").unwrap();
    }
    out.flush().unwrap();
    for (reason, n) in refusals {
        println!("REFUSAL\t{n}\t{reason}");
    }
    println!(
        "PROJECT_POPULATIONS inventories={} builds={} relations={} repeated_relations={}",
        session.projects.len(), session.populations_built, session.relations_built, session.relations_rebuilt,
    );
    let [empty, eval, operators, posting, capture] = session.closed_reuse;
    eprintln!("CLOSED_REUSE empty_emits={empty} evaluations={eval} operators={operators} posting_builds={posting} capture_splits={capture}");
    println!("ADMISSION_REPLAY aliased_file_sets={lists} duplicate_path_bytes_removed={path_bytes}");
    let c = &mut session.clocks;
    c.assemble += thread_cpu() - t;
    for (name, r) in &c.readers {
        println!("CHECK_READER {name} parsed={} total={:.6} read={:.6} build={:.6} tree={:.6} facts={:.6} other={:.6}",
            r.parsed, r.total, r.read, r.build, r.tree, r.facts, r.total - r.read - r.build - r.tree - r.facts);
    }
    println!(
        "CHECK_SESSION programs={} bindings={} sources={} reads={} parses={} pairs={} distinct_pairs={} skipped={} evaluated={} productive={} empty={} refused={} prep={:.6} read_parse={:.6} first={:.6} second={:.6} project={:.6} assembly={:.6} admission={:.6} read_versions={:.6} reader_build={:.6} reader_projection_facts={:.6} inventory={:.6} join={:.6} bound_proofs={:.6} literal_positions={} certified_pairs={} requirement_programs={} requirement_bindings={}",
        session.schedule.programs.len(),
        session.bindings.len(),
        session.schedule.sources.len(),
        c.reads,
        c.files,
        c.pairs,
        c.unique_pairs,
        c.skipped,
        c.evaluated, c.productive, c.empty, c.refused,
        c.prepare,
        c.read_parse,
        c.screen,
        c.evaluate,
        c.project,
        c.assemble,
        registration,
        c.read_versions,
        c.build_file,
        c.read_parse - c.read_versions - c.build_file,
        c.inventory,
        c.join,
        c.bound_proofs,
        c.literal_positions,
        c.certified,
        c.requirement_programs,
        c.requirement_bindings,
    );
    completed();
}

/// Standalone scenario entry, using the same scheduler and full typed output.
pub(crate) fn one(
    languages: Vec<Language>,
    pack: &str,
    input: Input,
    output: &str,
) -> Result<(), String> {
    let readers: Vec<_> = languages
        .into_iter()
        .map(session::source::Linked::from)
        .collect();
    one_source(&readers, pack, input, output)
}

fn one_source<L: session::source::Source>(
    readers: &[L],
    pack: &str,
    req: Input,
    output: &str,
) -> Result<(), String> {
    let mut session = Batch::default();
    session.file_catalog = session::named::Catalog::from_readers(readers)?;
    let work = ReplayInputs::default().register(&mut session, pack, req, None)?;
    session.work.push(work);
    let answers = session.run_sources(readers);
    let rows = Batch::answer(&session.work[0], &answers)?;
    if !rows.unknowns.is_empty() {
        return Err("partial answer requires typed unknown output".into());
    }
    let mut stream = Vec::new();
    if rows.rows.is_empty() { stream.push(b'\n'); }
    finish::json_output::ordered_rows(rows.rows, |line| {
        stream.extend_from_slice(line);
        stream.push(b'\n');
    });
    std::fs::write(output, stream).map_err(|e| e.to_string())
}

#[cfg(feature = "product-session")]
#[path = "live/calls.rs"]
pub(super) mod calls;
