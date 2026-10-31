//! Emit the enumerated potential implicit-name catalog once per physical module.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    qualification::{constants::BUILTIN_NAMES, service::builtin_named},
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Enumerated Python implicit-name providers; entries are potential origins, not live values.
pub(crate) struct ImplicitSymbolItem;

ryo::reads! {
 ImplicitSymbolItem => languages::ImplicitSymbol, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  each (BUILTIN_NAMES.iter()) as builtin {
   given since = builtin_named(*builtin)?;
   given name = languages::intern_derived(symbols, tree.source, *builtin);
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText("builtin"));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(since.major),since_minor:value(since.minor),file:file,}
  }
  each (BUILTIN_ATTRIBUTES.iter()) as attribute {
   given name = languages::intern_derived(symbols, tree.source, attribute.name);
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText("builtin"));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(attribute.major),since_minor:value(attribute.minor),file:file,}
  }
  each (MODULE_GLOBALS.iter()) as global {
   given name = languages::intern_derived(symbols, tree.source, global.name);
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText("module-global"));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(global.major),since_minor:value(global.minor),file:file,}
  }
  given name = languages::intern_derived(symbols, tree.source, languages::NameText("WindowsError"));
  given kind = languages::intern_derived(symbols, tree.source, languages::NameText("platform-builtin"));
  row {name:value(name),kind:value(kind),scope:node(at),since_major:value(languages::Integer(3)),since_minor:value(languages::Integer(0)),file:file,}
 }
}

#[derive(Clone, Copy)]
struct CatalogName {
    name: languages::NameText<'static>,
    major: languages::Integer,
    minor: languages::Integer,
}

/// Names supplied by the builtins environment, including attributes of the builtins module.
/// These module attributes overlap with module execution globals without sharing origin.
const BUILTIN_ATTRIBUTES: &[CatalogName] = &[
    catalog("__doc__", 3, 0),
    catalog("__loader__", 3, 0),
    catalog("__name__", 3, 0),
    catalog("__package__", 3, 0),
    catalog("__spec__", 3, 4),
];

/// Selected known names installed by Python's module environment, not declarations.
/// This is an enumerated provider catalog, not an exhaustive claim about every import loader.
/// Version anchors: `__cached__` is in What's New 3.2; `__spec__` was added in 3.4; module
/// `__annotations__` begins with PEP 526 in 3.6; `__annotate__` was added in the 3.14 data model.
/// The baseline names predate Python 3.0: `__loader__` is from PEP 302 and `__package__` from
/// PEP 366. Sources: docs.python.org/3.2/whatsnew/3.2.html, docs.python.org/3.12/reference/datamodel.html,
/// docs.python.org/3.6/whatsnew/3.6.html, docs.python.org/3/reference/datamodel.html,
/// peps.python.org/pep-0302/, and peps.python.org/pep-0366/.
const MODULE_GLOBALS: &[CatalogName] = &[
    catalog("__annotations__", 3, 6),
    catalog("__builtins__", 3, 0),
    catalog("__cached__", 3, 2),
    catalog("__doc__", 3, 0),
    catalog("__file__", 3, 0),
    catalog("__loader__", 3, 0),
    catalog("__name__", 3, 0),
    catalog("__package__", 3, 0),
    catalog("__spec__", 3, 4),
    catalog("__warningregistry__", 3, 0),
    catalog("__annotate__", 3, 14),
];

const fn catalog(name: &'static str, major: i64, minor: i64) -> CatalogName {
    CatalogName {
        name: languages::NameText(name),
        major: languages::Integer(major),
        minor: languages::Integer(minor),
    }
}
