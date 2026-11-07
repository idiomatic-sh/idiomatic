//! Compile the production relation boundary, with a valid client before each negative.
//! Unrelated scalar storage is scaffolded; interface, types, value and registry are real source.
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
const METHODS: &[(&str, &str)] = &[
    (
        "prepare",
        r#"fn prepare<'f>(&self,input:RepresentationInput<'f>)->Result<RelationContract<'f>,OperatingError>{
        let(kind,kinds)=match input {
            RepresentationInput::Stored(view)=>(RepresentationKind::Stored,view.kinds),
            RepresentationInput::Selected{kinds,..}=>(RepresentationKind::Selected,kinds),
            RepresentationInput::Tree(_)=>(RepresentationKind::Tree,&[][..]),
            RepresentationInput::Computed(table)=>(RepresentationKind::Computed,table.kinds.as_slice()),
            RepresentationInput::Compact(_)=>(RepresentationKind::Compact,&[][..]),
            RepresentationInput::Positioned(_)=>(RepresentationKind::Positioned,&[][..]),
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

const KINDS: &[&str] = &[
    "Stored",
    "Selected",
    "Tree",
    "Computed",
    "Compact",
    "Positioned",
];

fn production(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/domain/table_model/collections/representations")
            .join(name),
    )
    .unwrap()
}
/// The real interface, types, value and registration are inlined at their real module positions; unrelated scalars are stubs.
fn scaffold(omitted: Option<&str>, value: &str) -> String {
    let template = r#"
#![allow(dead_code,unused_imports,private_interfaces)]
extern crate self as ryo_language;
extern crate self as ryo_relations;
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
    pub mod rows {pub mod types {
        pub struct RowIndex(pub usize);
        pub struct Table {pub kinds:Vec<crate::domain::relations::types::CellKind>,pub cells:Vec<crate::domain::relations::types::Value>}
    }}
    pub mod table_words {pub mod types {pub struct CompactTable;}}
    pub mod table_cells {pub mod types {#[derive(Clone,Copy)]pub struct TreeRows<'a>(pub &'a());}}
    pub mod table_model {
        pub mod types {#[derive(Clone,Copy)]pub struct PositionedRows<'a>(pub &'a());}
        pub mod collections {pub mod representations {
            use crate::domain::{operating::errors::OperatingError,relations::types::Value,rows::types::RowIndex,plan_parts::types::ColumnIndex};
            use self::types::*;
            pub mod interface {__INTERFACE__}
            pub mod types {__TYPES__}
            pub mod stored {pub mod item {pub(crate) use super::super::Legitimate as StoredItem;}}
            pub mod selected {pub mod item {pub(crate) use super::super::Legitimate as SelectedItem;}}
            pub mod tree {pub mod item {pub(crate) use super::super::Legitimate as TreeItem;}}
            pub mod computed {pub mod item {pub(crate) use super::super::Legitimate as ComputedItem;}}
            pub mod compact {pub mod item {pub(crate) use super::super::Legitimate as CompactItem;}}
            pub mod positioned {pub mod item {pub(crate) use super::super::Legitimate as PositionedItem;}}
            pub mod registration {__REGISTRATION__}
            pub(crate) struct Legitimate;
            impl interface::RepresentationsItem for Legitimate {__METHODS__}
        }}
    }
}
use domain::operating::errors::OperatingError;
use domain::relations::types::Value;
use domain::rows::types::RowIndex;
use domain::plan_parts::types::ColumnIndex;
use domain::table_model::collections::representations::types::*;
"#;
    let types = production("types.rs").replace(
        "pub(crate) mod registered_representation;",
        &format!("pub(crate) mod registered_representation {{ pub mod value {{ {value} }} }}"),
    );
    template
        .replace("__INTERFACE__", &production("interface.rs"))
        .replace("__TYPES__", &types)
        .replace("__REGISTRATION__", &production("registration.rs"))
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
fn value() -> String {
    production("types/registered_representation/value.rs")
}

#[test]
fn every_relation_method_is_required_by_the_real_interface() {
    let directory = tempfile::tempdir().unwrap();
    let registration = value();
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
    let registration = value();
    positive(
        directory.path(),
        "complete_registry",
        &scaffold(None, &registration),
    );
    for kind in KINDS {
        let marker = format!("RepresentationKind::{kind} => ");
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
    let source = scaffold(None, &value());
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

fn declaration(source: &str, marker: &str) -> String {
    let start = source.find(marker).expect("production declaration");
    let opening = start + source[start..].find('{').expect("declaration body");
    let mut depth = 0usize;
    for (offset, byte) in source[opening..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return source[start..=opening + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("unclosed production declaration: {marker}");
}

/// The real record and the real published-relation trait, at their real module positions, with a valid owner and a peer client.
fn private_publication_scaffold() -> String {
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    let record =
        fs::read_to_string(core.join("src/application/file_dag/relations/publication/types.rs"))
            .unwrap();
    let published = production("publication.rs");
    r#"
#![allow(dead_code,unused_imports)]
pub mod domain {
  pub mod operating {pub mod errors {#[derive(Clone,Debug)]pub enum OperatingError {WrongOperation}}}
  pub mod table_model {
    pub mod types {#[derive(Debug,Default)]pub struct Table;}
    pub mod collections {pub mod representations {
      pub mod types {#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum PhysicalShape {Complete}}
      pub mod publication {__TRAIT__}
    }}
  }
}
pub mod application {pub mod file_dag {
  pub mod relations {
    pub mod publication {pub mod types {
      use crate::domain::operating::errors::OperatingError;
      use crate::domain::table_model::{types::Table,collections::representations::{publication::PublishedRelation,types::PhysicalShape}};
      use std::sync::{Arc,OnceLock};
      #[derive(Clone,Copy,Debug)]pub(in crate::application::file_dag) enum ProducerScope {File}
      #[derive(Debug)]pub(in crate::application::file_dag::relations) enum RelationBody {Computed(Arc<Table>)}
      #[derive(Debug)]
      __RESULT__
      impl PublishedRelation for RelationResult {fn as_any(&self)->&dyn std::any::Any{self}}
    }}
    pub fn control()->publication::types::RelationResult {
      use std::sync::{Arc,OnceLock};
      publication::types::RelationResult{population:0,producer:OnceLock::new(),producer_scope:OnceLock::new(),generation:Arc::new(()),
        body:publication::types::RelationBody::Computed(Arc::new(Default::default())),
        inspected:OnceLock::from(Ok(crate::domain::table_model::collections::representations::types::PhysicalShape::Complete)),normalized:OnceLock::new()}
    }
  }
  pub mod peer {
    use crate::application::file_dag::relations::publication::types::RelationResult;
    use crate::domain::table_model::{types::Table,collections::representations::{publication::PublishedRelation,types::PhysicalShape}};
    use std::sync::{Arc,OnceLock};
    fn legitimate(){let record=super::relations::control();let _=format!("{record:?}");}
    __CLIENT__
  }
}}
"#
    .replace("__TRAIT__", &declaration(&published, "pub(crate) trait PublishedRelation"))
    .replace("__RESULT__", &declaration(&record, "pub(crate) struct RelationResult"))
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
