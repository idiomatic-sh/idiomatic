//! Compile the actual private boundary inside a small same-crate scaffold.
//! The scaffold supplies only unrelated data types; the lease, trait and registry
//! are read from production source. Every negative first has a compiling control.
use std::{fs, path::Path, process::Command};
fn compile(directory: &Path, name: &str, source: &str) -> std::process::Output {
    let input = directory.join(format!("{name}.rs"));
    fs::write(&input, source).unwrap();
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(&input)
        .arg("-o")
        .arg(directory.join(format!("{name}.rmeta")))
        .output()
        .unwrap()
}
fn check_failure(directory: &Path, name: &str, source: &str, code: &str) {
    let result = compile(directory, name, source);
    let errors = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "negative control unexpectedly compiled: {name}"
    );
    assert!(errors.contains(code), "wrong refusal for {name}: {errors}");
}
fn rust_path_literal(path: &Path) -> String {
    format!(
        "{:?}",
        path.to_str().expect("Rust source path must be UTF-8")
    )
}
fn production_paths() -> (String, String, String, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contract = root.join("src/domain/operator/contract");
    let interface = root.join("src/domain/operator/collections/operators/interface.rs");
    let variants = ryo_values::domain::plans::types::OperationKind::ALL
        .iter()
        .map(|kind| format!("    {kind:?},\n"))
        .collect::<String>();
    let kinds = format!("pub enum OperationKind {{\n{variants}}}");
    (
        rust_path_literal(&contract.join("types.rs")),
        rust_path_literal(&interface),
        contract.join("registration.rs").display().to_string(),
        kinds,
    )
}
fn lease_scaffold(types: &str, interface: &str, kinds: &str) -> String {
    let representations = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/domain/table_model/collections/representations");
    let representation_types = rust_path_literal(&representations.join("types.rs"));
    let publication = rust_path_literal(&representations.join("publication.rs"));
    let base = r#"
#![allow(dead_code,unused_imports,private_interfaces)]
extern crate self as ryo_language;
extern crate self as ryo_values;
extern crate self as regex_automata;
extern crate self as hashbrown;
pub struct HashTable<T>(std::marker::PhantomData<T>);
pub mod meta {pub struct Regex;}
pub mod domain {
 pub mod operating {pub mod errors{#[derive(Clone,Debug)]pub struct OperatingError;}pub mod types{#[derive(Clone,Copy)]pub struct InputPlace(pub usize);pub trait SemanticLookup:std::fmt::Debug{}}}
 pub mod relations {pub mod types {#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct CellKind;#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct Value;#[derive(Clone,Copy)]pub struct ColumnsView<'a>(pub &'a ());pub struct Position;}}
 pub mod symbols {pub mod types {pub struct Symbols;pub struct QuerySymbols<'a>(pub &'a mut Symbols);impl QuerySymbols<'_>{pub fn as_ref(&self)->&Symbols {self.0}}}}
 pub mod files {pub mod types {pub struct FilePath;}}
 pub mod plan_parts {pub mod types {#[derive(Clone,Debug)]pub struct Inputs;#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct ColumnIndex;#[derive(Clone,Debug)]pub struct LiteralText;#[derive(Clone,Debug)]pub struct Comparison;}}
 pub mod plans {pub mod types {#[derive(Clone,Debug)]pub struct Operation;#[derive(Clone,Copy,Debug,PartialEq,Eq)] __KINDS__}}
 pub mod stored_facts {pub mod types {pub struct ArenaBytes;}}
 pub mod operation_schema {pub mod types {pub struct InputSchema<'a>{pub kinds:&'a[crate::domain::relations::types::CellKind]}}}
 pub mod stored_selection {pub mod types {#[derive(Debug)]pub struct Selector;#[derive(Debug)]pub struct PreparedPlan;#[derive(Debug)]pub struct PreparedSelectionViews;}}
 pub mod derived_views {pub mod types {#[derive(Clone,Debug,Default)]pub struct DerivedSlots;}}
 pub mod table_model {pub mod types {#[derive(Clone,Debug)]pub struct ParameterBinding;pub struct Table;pub struct Verdict;}pub mod collections {pub mod representations {#[path=__REPRESENTATION_TYPES__]pub mod types;#[path=__PUBLICATION__]pub mod publication;}}}
 pub mod table_texts {pub mod types {#[derive(Clone,Copy)]pub struct SourceView<'a>(pub &'a[u8]);}}
 pub mod table_scratch {pub mod types {pub struct ScratchWord;pub struct Stretches;}}
 pub mod table_cells {pub mod types {pub struct RowIndex;#[derive(Clone,Copy)]pub struct TreeRows<'a>(pub &'a());}}
 pub mod table_groups {pub mod types {pub struct Group;}}
 pub mod table_reach {pub mod types {pub struct Walk;}}
 pub mod operator {
  pub mod collections {pub mod operators {
   pub mod source_matches {pub mod source {pub struct ValidatedSource<'a>(pub &'a str);}}
   #[path=__INTERFACE__]pub mod interface;
  }}
  pub mod contract {
   pub mod views {#[derive(Clone,Copy)]pub struct BorrowedRows<'a>(pub &'a());pub struct IndexedRows<'a>(pub &'a());}
   pub mod graph {pub struct PreparedDfs;pub struct CycleGraph;pub struct ClosurePartition;pub struct OrderedMatching;pub struct DfsWorkspace;pub struct CycleMarks;}
   pub mod axis {pub struct PreparedAxis<'a>(pub &'a());pub struct AxisWorkspace;}
   pub mod join {pub struct NativeJoin<'a>(pub &'a());}
   pub mod graph_view {pub struct ReachResult<'a>(pub &'a());pub struct AxisResult<'a>(pub &'a());}
   pub mod resources {#[derive(Clone,Debug)]pub struct ResourceProgram;}
   #[path=__TYPES__]pub mod types;
  }
 }
}
"#;
    base.replace("__REPRESENTATION_TYPES__", &representation_types)
        .replace("__PUBLICATION__", &publication)
        .replace("__TYPES__", types)
        .replace("__INTERFACE__", interface)
        .replace("__KINDS__", kinds)
}
#[test]
fn generated_path_literal_escapes_windows_separators_and_quotes() {
    let literal = rust_path_literal(Path::new("C:\\source\\quoted\"module.rs"));
    assert_eq!(literal, "\"C:\\\\source\\\\quoted\\\"module.rs\"");
    let temp = tempfile::tempdir().unwrap();
    let control = compile(
        temp.path(),
        "path_literal_control",
        &format!("const PATH: &str = {literal};"),
    );
    assert!(
        control.status.success(),
        "invalid generated Rust literal: {}",
        String::from_utf8_lossy(&control.stderr)
    );
}
#[test]
fn primitive_cannot_acquire_scheduler_cache_or_persistent_lifetime_capabilities() {
    let temp = tempfile::tempdir().unwrap();
    let (types, interface, _, kinds) = production_paths();
    let source = lease_scaffold(&types, &interface, &kinds);
    let source = format!(
        "{source}{}",
        r#"
struct Legitimate;
impl domain::operator::collections::operators::interface::OperatorsItem for Legitimate {
 fn prepare(&self,_operation:&domain::plans::types::Operation,_inputs:&[domain::operation_schema::types::InputSchema<'_>],_bindings:&[domain::table_model::types::ParameterBinding])->Result<domain::operator::contract::types::OperatorContract,domain::operating::errors::OperatingError>{unimplemented!()}
 fn evaluate(&self,_operation:&domain::plans::types::Operation,inputs:domain::operator::contract::types::PreparedInputs<'_>,lease:&mut domain::operator::contract::types::PrimitiveLease<'_>,sink:&mut dyn domain::operator::contract::types::RowSink)->Result<(),domain::operating::errors::OperatingError>{
  if let domain::operator::contract::types::PreparedInputs::Row{row,..}=inputs {sink.row(lease,row)?;}Ok(())
 }
}
"#
    );

    let control = compile(temp.path(), "lease_control", &source);
    assert!(
        control.status.success(),
        "invalid scaffold: {}",
        String::from_utf8_lossy(&control.stderr)
    );
    for (name, body, code) in [
        ("file_cache", "lease.views.stored_indexes;", "E0609"),
        ("backend", "lease.views.backend;", "E0609"),
        ("memo_policy", "lease.memo;", "E0609"),
        ("input_schedule", "lease.hold();", "E0599"),
        ("predicate_rebinding", "lease.predicate;", "E0616"),
    ] {
        let negative = format!(
            "{source}\nfn forbidden(lease:&mut domain::operator::contract::types::PrimitiveLease<'_>){{{body}}}"
        );
        check_failure(temp.path(), name, &negative, code);
    }
    check_failure(
        temp.path(),
        "retained_source",
        &format!(
            "{source}\nfn escape(lease:&domain::operator::contract::types::PrimitiveLease<'_>)-> &'static [domain::files::types::FilePath]{{lease.views.paths}}"
        ),
        "lifetime may not live long enough",
    );
    check_failure(
        temp.path(),
        "missing_evaluator",
        &format!(
            "{source}\nstruct Missing;impl domain::operator::collections::operators::interface::OperatorsItem for Missing {{}}"
        ),
        "E0046",
    );
}
#[test]
fn removing_any_registration_is_a_compiler_error() {
    let temp = tempfile::tempdir().unwrap();
    let (_, _, registration, kinds) = production_paths();
    let production = fs::read_to_string(registration).unwrap();
    let mut scaffold = format!(
        "#![allow(dead_code,unused_imports)]\nextern crate self as ryo_values;\npub mod domain{{pub mod plans{{pub mod types{{#[derive(Clone,Copy)]{kinds}}}}}pub mod operator{{pub mod collections{{pub mod operators{{pub mod interface{{pub trait OperatorsItem:Sync{{}}}}\n"
    );
    let names = kinds[kinds.find('{').unwrap() + 1..kinds.rfind('}').unwrap()]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                None
            } else {
                Some(line.trim_end_matches(','))
            }
        })
        .collect::<Vec<_>>();
    for name in &names {
        let mut module = String::new();
        for (at, ch) in name.chars().enumerate() {
            if ch.is_uppercase() && at > 0 {
                module.push('_');
            }
            module.push(ch.to_ascii_lowercase());
        }
        scaffold.push_str(&format!("pub mod {module}{{pub mod item{{pub struct {name}Item;impl super::super::interface::OperatorsItem for {name}Item{{}}}}}}\n"));
    }
    scaffold.push_str("}}pub mod registry {");
    scaffold.push_str(&production);
    scaffold.push_str("}}}");
    let control = compile(temp.path(), "registry_control", &scaffold);
    assert!(
        control.status.success(),
        "invalid registry scaffold: {}",
        String::from_utf8_lossy(&control.stderr)
    );
    for name in names {
        let arm = format!("OperationKind::{name} => &{name}Item,");
        assert!(scaffold.contains(&arm), "fixture must mutate a real arm");
        check_failure(
            temp.path(),
            &format!("missing_{name}"),
            &scaffold.replacen(&arm, "", 1),
            "E0004",
        );
    }
}
