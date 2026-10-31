//! Emit the potential implicit-name catalog once per physical module.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    qualification::{constants::BUILTIN_NAMES, service::builtin_named},
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Canonical Python builtin inventory; no module globals or fabricated syntax IDs.
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
  row {name:value(name),kind:value(kind),scope:node(at),since_major:value(3),since_minor:value(0),file:file,}
 }
}

#[derive(Clone, Copy)]
struct CatalogName {
    name: languages::NameText<'static>,
    major: languages::Integer,
    minor: languages::Integer,
}

/// Attributes supplied by the actual builtins module environment.
const BUILTIN_ATTRIBUTES: &[CatalogName] = &[
    catalog("__doc__", 3, 0),
    catalog("__loader__", 3, 0),
    catalog("__name__", 3, 0),
    catalog("__package__", 3, 0),
    catalog("__spec__", 3, 4),
];

/// Names installed by Python's module execution environment, not declarations.
const MODULE_GLOBALS: &[CatalogName] = &[
    catalog("__annotations__", 3, 6),
    catalog("__builtins__", 3, 0),
    catalog("__cached__", 3, 2),
    catalog("__file__", 3, 0),
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
