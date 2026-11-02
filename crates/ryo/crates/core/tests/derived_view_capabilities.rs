//! Compile the actual derived trait, types, metadata service and registration.
//! Scalar resources/build bodies are scaffolded only for these compile tests;
//! actual family preparation is exercised by the separate in-crate registry tests.
use std::{fs, path::Path, process::Command};

fn compile(root: &Path, name: &str, source: &str) -> std::process::Output {
    let path = root.join(format!("{name}.rs"));
    fs::write(&path, source).unwrap();
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(path)
        .arg("-o")
        .arg(root.join(format!("{name}.rmeta")))
        .output()
        .unwrap()
}
fn positive(root: &Path, name: &str, source: &str) {
    let output = compile(root, name, source);
    assert!(
        output.status.success(),
        "invalid positive control {name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn negative(root: &Path, name: &str, source: &str, code: &str) {
    let output = compile(root, name, source);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "negative compiled: {name}");
    assert!(error.contains(code), "wrong refusal for {name}: {error}");
}
fn rust_path(path: &Path) -> String {
    format!("{:?}", path.to_str().expect("UTF-8 production source path"))
}
const METHODS: &[(&str, &str)] = &[
    (
        "prepare",
        "fn prepare(&self, request:&ViewRequest, inputs:&[InputSchema<'_>])->Result<DerivedContract,OperatingError>{service::prepare(service::kind(request),request,inputs)}",
    ),
    (
        "build",
        "fn build(&self,_contract:&DerivedContract,_inputs:DerivedInputs<'_,'_>,_storage:&mut DerivedStorage,_scratch:&mut DerivedScratch)->Result<(),OperatingError>{Ok(())}",
    ),
    (
        "view",
        "fn view<'a>(&self,contract:&DerivedContract,storage:&'a DerivedStorage)->Result<DerivedView<'a>,OperatingError>{service::view(service::kind(&contract.request),contract,storage)}",
    ),
];
const FAMILIES: &[&str] = &[
    "Equality",
    "PredicateMatches",
    "CandidatePostings",
    "DistinctRepresentatives",
    "Adjacency",
    "Membership",
    "PartitionOrder",
    "ClosureAdjacency",
    "CycleAdjacency",
    "OrderedKeys",
    "OrderedDfs",
    "NativeLookup",
];

fn source(omit: Option<&str>, registry: &str) -> String {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/derived_views");
    let template = r#"
#![allow(dead_code,unused_imports,private_interfaces)]
extern crate self as ryo_language;
extern crate self as ryo_values;
extern crate self as hashbrown;
pub use std::collections::HashSet;
pub mod domain {
 pub mod files {pub mod types {pub struct FilePath;}}
 pub mod relations {pub mod types {
  #[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum CellKind {Number}
  #[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Relation {Fixture}
  #[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]pub enum Value {Number(i64)}
 }}
 pub mod symbols {pub mod types {pub struct Symbols;pub struct QuerySymbols<'a>(pub std::marker::PhantomData<&'a mut ()>);}}
 pub mod plan_parts {pub mod types {
  #[derive(Clone,Copy,Debug)]pub struct ColumnIndex(pub u32);
  #[derive(Clone,Debug)]pub struct LiteralText(pub String);
 }}
 pub mod stored_facts {pub mod types {pub struct ArenaBytes;}}
 pub mod operating {pub mod errors {
  #[derive(Clone,Debug)]pub enum OperatingError {WrongOperation,NotRun,MissingInput{position:usize},NoSuchColumn}
 }}
 pub mod operation_schema {pub mod types {
  pub struct InputSchema<'a>{pub kinds:&'a[crate::domain::relations::types::CellKind]}
 }}
 pub mod operator {pub mod contract {
  pub mod graph {
   #[derive(Debug)]pub struct ClosurePartition;#[derive(Debug)]pub struct CycleGraph;
   #[derive(Debug)]pub struct OrderedMatching;#[derive(Debug)]pub struct PreparedDfs;
  }
  pub mod resources {
   use crate::domain::plan_parts::types::ColumnIndex;
   #[derive(Clone,Debug)]pub struct DfsResources {
    pub root:ColumnIndex,pub source:ColumnIndex,pub destination:ColumnIndex,pub order:ColumnIndex,pub edge:ColumnIndex,
    pub root_keys:Vec<ColumnIndex>,pub edge_keys:Vec<ColumnIndex>
   }
  }
  pub mod types {pub trait PredicateProgram {}}
  pub mod views {#[derive(Clone,Copy)]pub struct BorrowedRows<'a>(pub std::marker::PhantomData<&'a()>);}
 }}
 pub mod stored_graph {pub mod types {#[derive(Debug)]pub struct CompactGraph;pub struct GraphSource;}}
 pub mod stored_selection {pub mod types {#[derive(Debug)]pub struct LiteralPostings;#[derive(Clone,Debug)]pub struct Selector;}}
 pub mod table_cells {pub mod types {#[derive(Clone,Debug,Default)]pub struct ColumnPlaces(pub Vec<usize>);#[derive(Debug)]pub struct RowIndex(pub usize);pub struct TreeRows<'a>(pub &'a());}}
 pub mod table_model {
  pub mod types {#[derive(Debug)]pub struct KeyIndex;#[derive(Default)]pub struct Table;}
  pub mod service {
   use crate::domain::{relations::types::CellKind,plan_parts::types::ColumnIndex,operating::errors::OperatingError};
   pub fn check_columns(kinds:&[CellKind],columns:&[ColumnIndex])->Result<(),OperatingError>{
    if columns.iter().any(|c|c.0 as usize>=kinds.len()){Err(OperatingError::NoSuchColumn)}else{Ok(())}
   }
  }
 }
 pub mod table_texts {pub mod types {pub struct SourceView<'a>(pub &'a());pub struct Texts<'a>(pub &'a());}}
 pub mod tree_join {pub mod collections {pub mod strategies {pub mod types {#[derive(Clone,Copy,Debug)]pub struct Finding;}}}}
 pub mod derived_views {
  #[path=__TYPES__]pub mod types;
  #[path=__SERVICE__]pub mod service;
  pub mod collections {pub mod views {
   #[path=__INTERFACE__]pub mod interface;
   pub mod items {__ITEMS__}
   pub mod registration {__REGISTRY__}
  }}
 }
}
use domain::derived_views::{types::*,service,collections::views::{interface::DerivedViewsItem,registration::registered}};
use domain::{operating::errors::OperatingError,operation_schema::types::InputSchema};
pub(crate) struct Legitimate;
impl DerivedViewsItem for Legitimate {__METHODS__}
fn legitimate_client(){
 let kinds=[domain::relations::types::CellKind::Number];let inputs=[InputSchema{kinds:&kinds}];
 let item=registered(DerivedKind::Equality);
 let contract=item.prepare(&ViewRequest::Equality{columns:vec![domain::plan_parts::types::ColumnIndex(0)]},&inputs).unwrap();
 let storage=DerivedStorage::Equality(domain::table_model::types::KeyIndex);
 let _borrowed=item.view(&contract,&storage).unwrap();
}
"#;
    let methods = METHODS
        .iter()
        .filter(|(name, _)| Some(*name) != omit)
        .map(|(_, body)| *body)
        .collect::<Vec<_>>()
        .join("\n");
    let items = FAMILIES
        .iter()
        .map(|name| format!("pub(crate) use crate::Legitimate as {name}Item;"))
        .collect::<Vec<_>>()
        .join("\n");
    template
        .replace("__TYPES__", &rust_path(&core.join("types.rs")))
        .replace("__SERVICE__", &rust_path(&core.join("service.rs")))
        .replace(
            "__INTERFACE__",
            &rust_path(&core.join("collections/views/interface.rs")),
        )
        .replace("__ITEMS__", &items)
        .replace("__REGISTRY__", registry)
        .replace("__METHODS__", &methods)
}
fn registry() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/domain/derived_views/collections/views/registration.rs"),
    )
    .unwrap()
}

#[test]
fn mandatory_methods_and_every_real_registry_arm_are_enforced() {
    let directory = tempfile::tempdir().unwrap();
    let registration = registry();
    positive(
        directory.path(),
        "actual_boundary",
        &source(None, &registration),
    );
    for (method, _) in METHODS {
        negative(
            directory.path(),
            &format!("missing_{method}"),
            &source(Some(method), &registration),
            "E0046",
        );
    }
    for (index, family) in FAMILIES.iter().enumerate() {
        let arm = format!("        DerivedKind::{family} => ITEMS[{index}],\n");
        assert!(
            registration.contains(&arm),
            "real registration arm changed: {family}"
        );
        negative(
            directory.path(),
            &format!("missing_{family}"),
            &source(None, &registration.replacen(&arm, "", 1)),
            "E0004",
        );
    }
}

#[test]
fn view_has_no_owner_scheduler_source_scratch_or_mutation_capability() {
    let directory = tempfile::tempdir().unwrap();
    let original = source(None, &registry());
    positive(directory.path(), "readonly_view", &original);
    let body = "service::view(service::kind(&contract.request),contract,storage)";
    for (name, expression, code) in [
        ("cache", "let _ = storage.cache;", "E0609"),
        ("scheduler", "let _ = contract.scheduler;", "E0609"),
        ("source", "let _ = storage.source;", "E0609"),
        ("scratch", "let _ = scratch;", "E0425"),
        ("acquire", "let _ = storage.acquire();", "E0599"),
        ("mutation", "*storage = DerivedStorage::Unbuilt;", "E0594"),
    ] {
        let bad = original.replacen(body, &format!("{expression}{body}"), 1);
        negative(directory.path(), name, &bad, code);
    }
    let extra_input = original.replacen(
        "storage:&'a DerivedStorage)->Result<DerivedView<'a>,OperatingError>",
        "storage:&'a DerivedStorage,inputs:DerivedInputs<'_,'_>)->Result<DerivedView<'a>,OperatingError>",
        1,
    );
    assert_ne!(extra_input, original);
    negative(
        directory.path(),
        "view_source_parameter",
        &extra_input,
        "E0050",
    );
    negative(
        directory.path(),
        "escaping_view",
        &(original
            + r#"
fn escape<'a>(item:&dyn DerivedViewsItem,contract:&DerivedContract)->DerivedView<'a>{
 let storage=DerivedStorage::RowIds{kind:DerivedKind::PredicateMatches,rows:vec![0]};
 item.view(contract,&storage).unwrap()
}
"#),
        "E0515",
    );
}
