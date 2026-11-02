//! Compile the production relation boundary, with a valid client before each negative.
//! Unrelated scalar storage is scaffolded; interface, data lease and registry are real source.
use std::{fs, path::Path, process::Command};

fn compile(directory: &Path, name: &str, source: &str) -> std::process::Output {
    let input = directory.join(format!("{name}.rs"));
    fs::write(&input, source).unwrap();
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(input)
        .arg("-o")
        .arg(directory.join(format!("{name}.rmeta")))
        .output()
        .unwrap()
}
fn positive(directory: &Path, name: &str, source: &str) {
    let result = compile(directory, name, source);
    assert!(
        result.status.success(),
        "invalid positive control {name}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
fn negative(directory: &Path, name: &str, source: &str, diagnostic: &str) {
    let result = compile(directory, name, source);
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success(), "negative compiled: {name}");
    assert!(
        error.contains(diagnostic),
        "wrong refusal for {name}: {error}"
    );
}
fn rust_path(path: &Path) -> String {
    // A Rust string literal, including quotes: spaces, backslashes and quotes are escaped.
    format!("{:?}", path.to_str().expect("UTF-8 production path"))
}
const METHODS: &[(&str, &str)] = &[
    (
        "prepare",
        r#"fn prepare<'f>(&self,input:RepresentationInput<'f>)->Result<RelationContract<'f>,OperatingError>{
        let(kind,kinds)=match input {
            RepresentationInput::Stored(view)=>(RepresentationKind::Stored,view.kinds),
            RepresentationInput::Selected{kinds,..}=>(RepresentationKind::Selected,kinds),
            RepresentationInput::Tree(_)=>(RepresentationKind::Tree,&[][..]),
            RepresentationInput::Computed(table)=>(RepresentationKind::Computed,table.kinds.as_slice()),
        };
        Ok(RelationContract{input,kind,kinds,rows:0,metadata:Ok(())})
    }"#,
    ),
    (
        "inspect",
        "fn inspect(&self,_contract:&RelationContract<'_>)->Result<PhysicalShape,OperatingError>{Ok(PhysicalShape::Complete)}",
    ),
    (
        "bulk",
        "fn bulk<'f>(&self,input:RepresentationInput<'f>)->Result<BulkAccess<'f>,OperatingError>{Ok(input)}",
    ),
    (
        "cell",
        "fn cell(&self,_input:RepresentationInput<'_>,_row:RowIndex,_column:ColumnIndex)->Result<CellRead,OperatingError>{Ok(CellRead::Value(Value::Number(0)))}",
    ),
    (
        "row",
        "fn row<'r>(&self,_input:RepresentationInput<'r>,_row:RowIndex,decode:&'r mut[Value])->Result<RowRead<'r>,OperatingError>{Ok(RowRead::Decoded(decode))}",
    ),
];

fn scaffold(omitted: Option<&str>, registry: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/domain/table_model/collections/representations");
    let template = r#"
#![allow(dead_code,unused_imports,private_interfaces)]
extern crate self as ryo_language;
extern crate self as ryo_values;
pub mod domain {
    pub mod operating {pub mod errors {#[derive(Clone,Debug)]pub enum OperatingError {WrongOperation}}}
    pub mod relations {pub mod types {
        #[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum CellKind {Number}
        #[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Value {Number(u32)}
        pub struct FactColumn(pub Vec<u64>);
        #[derive(Clone,Copy)]pub struct ColumnsView<'a> {pub kinds:&'a[CellKind],pub columns:&'a[FactColumn]}
    }}
    pub mod plan_parts {pub mod types {pub struct ColumnIndex(pub u32);}}
    pub mod table_cells {pub mod types {
        pub struct RowIndex(pub usize);
        #[derive(Clone,Copy)]pub struct TreeRows<'a>(pub &'a());
    }}
    pub mod table_model {
        pub mod types {
            pub struct Table {pub kinds:Vec<crate::domain::relations::types::CellKind>,pub cells:Vec<crate::domain::relations::types::Value>}
        }
        pub mod collections {pub mod representations {
            #[path=__INTERFACE__]pub mod interface;
            #[path=__TYPES__]pub mod types;
            pub mod stored {pub mod item {pub(crate) use crate::Legitimate as StoredItem;}}
            pub mod selected {pub mod item {pub(crate) use crate::Legitimate as SelectedItem;}}
            pub mod tree {pub mod item {pub(crate) use crate::Legitimate as TreeItem;}}
            pub mod computed {pub mod item {pub(crate) use crate::Legitimate as ComputedItem;}}
            pub mod registration {__REGISTRATION__}
        }}
    }
}
use domain::operating::errors::OperatingError;
use domain::relations::types::Value;
use domain::table_cells::types::RowIndex;
use domain::plan_parts::types::ColumnIndex;
use domain::table_model::collections::representations::{interface::RepresentationsItem,types::*};
pub(crate) struct Legitimate;
impl RepresentationsItem for Legitimate {__METHODS__}
"#;
    template
        .replace("__INTERFACE__", &rust_path(&root.join("interface.rs")))
        .replace("__TYPES__", &rust_path(&root.join("types.rs")))
        .replace("__REGISTRATION__", registry)
        .replace(
            "__METHODS__",
            &METHODS
                .iter()
                .filter(|(name, _)| Some(*name) != omitted)
                .map(|(_, body)| *body)
                .collect::<Vec<_>>()
                .join("\n"),
        )
}
fn registry() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/domain/table_model/collections/representations/registration.rs"),
    )
    .unwrap()
}

#[test]
fn every_relation_method_is_required_by_the_real_interface() {
    let directory = tempfile::tempdir().unwrap();
    let registration = registry();
    positive(
        directory.path(),
        "all_methods",
        &scaffold(None, &registration),
    );
    for (method, _) in METHODS {
        negative(
            directory.path(),
            &format!("missing_{method}"),
            &scaffold(Some(method), &registration),
            "E0046",
        );
    }
}

#[test]
fn every_real_representation_arm_is_required() {
    let directory = tempfile::tempdir().unwrap();
    let registration = registry();
    positive(
        directory.path(),
        "complete_registry",
        &scaffold(None, &registration),
    );
    for kind in ["Stored", "Selected", "Tree", "Computed"] {
        let marker = format!("RepresentationKind::{kind}");
        let begin = registration
            .find(&marker)
            .expect("actual production registry arm");
        let end = begin + registration[begin..].find(',').unwrap() + 1;
        let mut missing = registration.clone();
        missing.replace_range(begin..end, "");
        negative(
            directory.path(),
            &format!("unregistered_{kind}"),
            &scaffold(None, &missing),
            "E0004",
        );
    }
}

#[test]
fn physical_readers_cannot_gain_scheduler_cache_mutation_or_static_source_lifetimes() {
    let directory = tempfile::tempdir().unwrap();
    let source = scaffold(None, &registry());
    positive(directory.path(), "borrowed_relation_control", &source);
    for (name, body, diagnostic) in [
        (
            "scheduler",
            "fn forbidden(contract:RelationContract<'_>){let _=contract.scheduler;}",
            "E0609",
        ),
        (
            "cache",
            "fn forbidden(contract:RelationContract<'_>){let _=contract.file_indexes;}",
            "E0609",
        ),
        (
            "context",
            "fn forbidden(input:RepresentationInput<'_>){let _=input.operator_context();}",
            "E0599",
        ),
        (
            "stored_mutation",
            "fn forbidden(input:RepresentationInput<'_>){if let RepresentationInput::Stored(view)=input{view.columns[0].0.push(1);}}",
            "E0596",
        ),
        (
            "computed_mutation",
            "fn forbidden(input:RepresentationInput<'_>){if let RepresentationInput::Computed(table)=input{table.cells.clear();}}",
            "E0596",
        ),
        (
            "selected_mutation",
            "fn forbidden(input:RepresentationInput<'_>){if let RepresentationInput::Selected{positions,..}=input{positions[0]=1;}}",
            "E0594",
        ),
        (
            "source_escape",
            "fn forbidden<'a>(input:RepresentationInput<'a>)->BulkAccess<'static>{input}",
            "lifetime may not live long enough",
        ),
        (
            "decoded_row_escape",
            "fn forbidden<'a>(decode:&'a mut[Value])->RowRead<'static>{RowRead::Decoded(decode)}",
            "lifetime may not live long enough",
        ),
        (
            "schema_escape",
            "fn forbidden(contract:RelationContract<'_>)->&'static[domain::relations::types::CellKind]{contract.kinds}",
            "lifetime may not live long enough",
        ),
    ] {
        negative(
            directory.path(),
            name,
            &format!("{source}\n{body}"),
            diagnostic,
        );
    }
}

fn private_publication_scaffold() -> String {
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    let directory = core.join("src/application/file_dag/relations");
    let token = fs::read_to_string(directory.join("produced.rs")).unwrap();
    let start = token.find("pub(crate) struct ProducedSelection").unwrap();
    let end = start + token[start..].find("\n}").unwrap() + 2;
    // These are the actual authority-bearing fields and visibility, not a narrower copy.
    let token = &token[start..end];
    let mut source = scaffold(None, &registry());
    source = source.replace("    pub mod table_model {", "    pub mod stored_selection {pub mod types {#[derive(Debug)]pub struct Selector;}}\n    pub mod table_model {");
    source = source.replace("pub struct Table {", "#[derive(Debug,Default)]pub struct StoredSelection;\n            #[derive(Debug,Default)]pub struct Table {");
    source = source.replacen(
        "pub mod types;",
        &format!(
            "pub mod types; #[path={}]pub mod publication;",
            rust_path(
                &core.join("src/domain/table_model/collections/representations/publication.rs")
            )
        ),
        1,
    );
    let private = r#"
pub mod application {pub(crate) mod file_dag {
 mod relations {
  use crate::domain::operating::errors::OperatingError;
  use crate::domain::table_model::{types::{Table,StoredSelection},collections::representations::{types::{PhysicalShape,RepresentationKind},publication::PublishedRelation}};
  use std::sync::{Arc,OnceLock};
  #[derive(Debug)]enum RelationBody {Computed(Arc<Table>)}
  #[path=__RECORD__]mod record;
  pub(crate) use record::RelationResult;
  mod produced {use super::{Arc,RelationResult,StoredSelection};__TOKEN__}
  pub(super) use produced::ProducedSelection;
  pub(super) fn control()->(Arc<RelationResult>,ProducedSelection) {
   let result=Arc::new(RelationResult{population:0,producer:OnceLock::new(),generation:Arc::new(()),kind:RepresentationKind::Computed,body:RelationBody::Computed(Arc::new(Table::default())),inspected:OnceLock::from(Ok(PhysicalShape::Complete)),normalized:OnceLock::new()});
   let token=produced::ProducedSelection{source:Arc::clone(&result),selection:Arc::new(StoredSelection)};
   (result,token)
  }
 }
 mod peer {
  use super::relations::{RelationResult,ProducedSelection};
  use crate::domain::table_model::{types::Table,collections::representations::{publication::PublishedRelation,types::{PhysicalShape,RepresentationKind}}};
  use std::sync::{Arc,OnceLock};
  fn legitimate(){let(record,token)=super::relations::control();let _=record.kind();drop(token);}
  __CLIENT__
 }
}}
"#;
    source.push_str(
        &private
            .replace("__RECORD__", &rust_path(&directory.join("record.rs")))
            .replace("__TOKEN__", token),
    );
    source
}

#[test]
fn only_the_actual_application_owner_can_construct_or_modify_admission_state() {
    let directory = tempfile::tempdir().unwrap();
    let source = private_publication_scaffold();
    positive(
        directory.path(),
        "private_publication_control",
        &source.replace("__CLIENT__", ""),
    );
    for (name, client, error) in [
        (
            "forge_record",
            "fn forbidden(seed:RelationResult)->RelationResult{RelationResult{inspected:OnceLock::from(Ok(PhysicalShape::Complete)),..seed}}",
            "E0451",
        ),
        (
            "write_inspection",
            "fn forbidden(record:&RelationResult){let _=record.inspected.set(Ok(PhysicalShape::Complete));}",
            "E0616",
        ),
        (
            "write_normalized",
            "fn forbidden(record:&RelationResult){let _=record.normalized.set(Arc::new(Table::default()));}",
            "E0616",
        ),
        (
            "write_epoch",
            "fn forbidden(record:&mut RelationResult){record.generation=Arc::new(());}",
            "E0616",
        ),
        (
            "forge_producer_token",
            "fn forbidden(seed:ProducedSelection)->ProducedSelection{ProducedSelection{source:super::relations::control().0,..seed}}",
            "E0451",
        ),
        (
            "rewrite_producer_projection",
            "fn forbidden(token:&mut ProducedSelection){token.selection=Arc::new(Default::default());}",
            "E0616",
        ),
        (
            "trait_proof_backdoor",
            "fn forbidden(record:&dyn PublishedRelation){let _=record.inspected().set(Ok(PhysicalShape::Complete));}",
            "E0599",
        ),
        (
            "trait_cache_backdoor",
            "fn forbidden(record:&dyn PublishedRelation){let _=record.normalized().set(Arc::new(Table::default()));}",
            "E0599",
        ),
    ] {
        negative(
            directory.path(),
            name,
            &source.replace("__CLIENT__", client),
            error,
        );
    }
}
