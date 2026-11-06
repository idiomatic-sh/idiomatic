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
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::BUILTIN));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(since.major),since_minor:value(since.minor),file:file,}
  }
  each (BUILTIN_ATTRIBUTES.iter()) as attribute {
   given name = languages::intern_derived(symbols, tree.source, attribute.name);
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::BUILTIN));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(attribute.major),since_minor:value(attribute.minor),file:file,}
  }
  each (MODULE_GLOBALS.iter()) as global {
   given name = languages::intern_derived(symbols, tree.source, global.name);
   given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::MODULE_GLOBAL));
   row {name:value(name),kind:value(kind),scope:node(at),since_major:value(global.major),since_minor:value(global.minor),file:file,}
  }
  given name = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::WINDOWSERROR));
  given kind = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::PLATFORM_BUILTIN));
  row {name:value(name),kind:value(kind),scope:node(at),since_major:value(languages::Integer(3)),since_minor:value(languages::Integer(0)),file:file,}
 }
}

#[derive(Clone, Copy)]
/// A potential Python environment-provided name and its target-version origin.
struct CatalogName {
    /// The spelling made available by Python.
    name: languages::NameText<'static>,
    /// First Python major version providing the name.
    major: languages::Integer,
    /// First Python minor version providing the name.
    minor: languages::Integer,
}

/// Names supplied by the builtins environment, including attributes of the builtins module.
/// These module attributes overlap with module execution globals without sharing origin.
const BUILTIN_ATTRIBUTES: &[CatalogName] = &[
    catalog(super::constants::DOC, 3, 0),
    catalog(super::constants::LOADER, 3, 0),
    catalog(super::constants::NAME, 3, 0),
    catalog(super::constants::PACKAGE, 3, 0),
    catalog(super::constants::SPEC, 3, 4),
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
    catalog(super::constants::ANNOTATIONS, 3, 6),
    catalog(super::constants::BUILTINS, 3, 0),
    catalog(super::constants::CACHED, 3, 2),
    catalog(super::constants::DOC, 3, 0),
    catalog(super::constants::FILE, 3, 0),
    catalog(super::constants::LOADER, 3, 0),
    catalog(super::constants::NAME, 3, 0),
    catalog(super::constants::PACKAGE, 3, 0),
    catalog(super::constants::SPEC, 3, 4),
    catalog(super::constants::WARNINGREGISTRY, 3, 0),
    catalog(super::constants::ANNOTATE, 3, 14),
];

/// Build a static catalog row from a name and its first supported version.
const fn catalog(name: &'static str, major: i64, minor: i64) -> CatalogName {
    CatalogName {
        name: languages::NameText(name),
        major: languages::Integer(major),
        minor: languages::Integer(minor),
    }
}
