//! Compile the real cache and context declarations at the application boundary.
//! Scalar dependencies are scaffolded; the authority-bearing declarations are not.
use std::{fs, path::Path, process::Command};

fn rust_path(path: &Path) -> String {
    format!("{:?}", path.to_str().expect("UTF-8 Rust source path"))
}

fn compile(directory: &Path, name: &str, source: &str) -> std::process::Output {
    let input = directory.join(format!("{name}.rs"));
    fs::write(&input, source).unwrap();
    Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(&input)
        .arg("-o")
        .arg(directory.join(format!("{name}.rmeta")))
        .output()
        .unwrap()
}

fn positive(directory: &Path, name: &str, source: &str) {
    let result = compile(directory, name, source);
    assert!(
        result.status.success(),
        "invalid owner control {name}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn negative(directory: &Path, name: &str, source: &str, diagnostic: &str) {
    let result = compile(directory, name, source);
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success(), "forbidden {name} compiled");
    assert!(
        error.contains(diagnostic),
        "wrong refusal for {name}: {error}"
    );
}

fn backend_scaffold(client: &str) -> String {
    let backend =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/application/file_dag/derived/owner/cache/types.rs");
    let template = r#"
#![allow(dead_code,unused_imports,private_interfaces)]
extern crate self as ryo_language;
extern crate self as ryo_relations;
extern crate self as ryo_values;
pub mod domain {
 pub mod table_model {pub mod types {#[derive(Debug)]pub struct KeyIndex;}}
 pub mod relations {pub mod types {
  #[derive(Debug)]pub struct CellKind;
  #[derive(Debug)]pub struct Relation;
 }}
 pub mod plan_parts {pub mod types {#[derive(Debug)]pub struct ColumnIndex;#[derive(Debug)]pub struct LiteralText;}}
 pub mod derived_views {pub mod types {pub mod contract {pub mod value {#[derive(Debug)]pub struct PreparedSelectionViews;}}}}
 pub mod stored_selection {pub mod types {
  #[derive(Debug)]pub struct Selector;
  #[derive(Debug)]pub struct Matches;
  #[derive(Debug)]pub struct LiteralFanout;
  #[derive(Debug)]pub struct CandidateColumn;
  #[derive(Debug)]pub struct SelectionCounts;
  #[derive(Debug)]pub struct SelectorHistogram;
 }}
 pub mod stored_graph {pub mod types {
  #[derive(Debug)]pub struct GraphEntry;
  #[derive(Debug)]pub struct DomainEntry;
  #[derive(Debug)]pub struct NearestMemo;
  #[derive(Debug)]pub struct ExpandedVertices(pub u64);
  #[derive(Debug)]pub struct ExaminedVertices(pub u64);
  #[derive(Debug)]pub struct GraphCounts {pub expanded:ExpandedVertices,pub examined:ExaminedVertices}
 }}
}
pub mod application {pub mod file_dag {
 pub mod derived {
  pub mod ledger {pub mod types {#[derive(Debug,Clone,Copy,Default)]pub struct ByteCount(pub usize);}}
  pub mod owner {
   pub mod index {pub mod types {#[derive(Debug,Clone,Copy,Default)]pub struct ByteLimit(pub usize);}}
   pub mod types {
    #[derive(Debug)]pub struct BuildCount(pub u64);
    #[derive(Debug)]pub struct ReuseCount(pub u64);
    #[derive(Debug)]pub struct RefusalCount(pub u64);
   }
   pub mod graphs {pub mod types {pub mod retained {pub mod types {
    #[derive(Debug)]pub struct GraphEntry;
    #[derive(Debug)]pub struct DomainEntry;
    #[derive(Debug)]pub struct NearestMemo;
   }}}}
   pub mod cache {
    #[path=__BACKEND__]pub(in crate::application::file_dag) mod types;
   }
   use cache::types::CacheBackend;
   pub fn inspect(cache:&CacheBackend)->usize {cache.graphs.len()}
  }
 }
 pub mod consumer {
  use super::derived::owner::cache::types::CacheBackend;
  pub fn legitimate(cache:&CacheBackend)->usize {super::derived::owner::inspect(cache)}
  __CLIENT__
 }
}}
"#;
    template
        .replace("__BACKEND__", &rust_path(&backend))
        .replace("__CLIENT__", client)
}

#[test]
fn only_derived_owner_can_mutate_the_actual_backend() {
    let directory = tempfile::tempdir().unwrap();
    positive(
        directory.path(),
        "owner_can_read_actual_backend",
        &backend_scaffold(""),
    );
    for (name, client, diagnostic) in [
        (
            "consumer_read_entries",
            "pub fn forbidden(cache:&CacheBackend)->usize {cache.graphs.len()}",
            "E0616",
        ),
        (
            "consumer_write_bytes",
            "pub fn forbidden(cache:&mut CacheBackend){cache.bytes=0;}",
            "E0616",
        ),
        (
            "consumer_rebind_entries",
            "pub fn forbidden(seed:CacheBackend)->CacheBackend {CacheBackend{graphs:Vec::new(),..seed}}",
            "E0451",
        ),
    ] {
        negative(
            directory.path(),
            name,
            &backend_scaffold(client),
            diagnostic,
        );
    }
}

fn context_scaffold(client: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/operating/types.rs");
    let production = fs::read_to_string(path).unwrap();
    let start = production
        .find("pub(crate) struct OperatorContext")
        .unwrap();
    let end = start + production[start..].find("\n}").unwrap() + 2;
    let declaration = &production[start..end];
    let template = r#"
#![allow(dead_code,unused_imports)]
pub mod domain {
 pub mod operating {pub mod types {
  use crate::domain::{table_model::types::ParameterBinding,symbols::types::QuerySymbols,table_texts::types::SourceView,files::types::FilePath,stored_facts::types::ArenaBytes};
  #[derive(Debug)]pub struct AfterTheFiles;
  #[derive(Debug)]pub struct Memo;
  #[derive(Debug)]pub struct OperatorScratch;
  pub trait SemanticLookup: std::fmt::Debug {}
  __DECLARATION__
 }}
 pub mod matching {pub mod types {#[derive(Debug)]pub struct PatternMatcher<'a>(pub std::marker::PhantomData<&'a()>);pub mod prepared {pub mod value {#[derive(Debug)]pub struct PreparedMatchers;}}}}
 pub mod kind_sets {pub mod types {pub mod vocabulary {pub mod value {#[derive(Debug)]pub struct PreparedVocabulary;}}}}
 pub mod table_model {pub mod types {#[derive(Debug)]pub struct ParameterBinding;}}
 pub mod symbols {pub mod types {#[derive(Debug)]pub struct QuerySymbols<'a>(pub std::marker::PhantomData<&'a()>);}}
 pub mod table_texts {pub mod types {#[derive(Debug)]pub struct SourceView<'a>(pub std::marker::PhantomData<&'a()>);}}
 pub mod files {pub mod types {#[derive(Debug)]pub struct FilePath;}}
 pub mod stored_facts {pub mod types {#[derive(Debug)]pub struct ArenaBytes;}}
}
use domain::operating::types::OperatorContext;
fn legitimate(context:&OperatorContext<'_>)->usize {context.paths.len()}
__CLIENT__
"#;
    template
        .replace("__DECLARATION__", declaration)
        .replace("__CLIENT__", client)
}

#[test]
fn operator_context_exposes_no_mutable_file_cache() {
    let directory = tempfile::tempdir().unwrap();
    positive(
        directory.path(),
        "context_without_cache",
        &context_scaffold(""),
    );
    negative(
        directory.path(),
        "context_cache_access",
        &context_scaffold(
            "fn forbidden(context:&OperatorContext<'_>){let _=&context.stored_indexes;}",
        ),
        "E0609",
    );
}
