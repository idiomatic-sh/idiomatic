//! The library's own nodes of a file compared with the rows: the name of every function definition with the `function` rows, the
//! binding and the target of every name an import brings in with the `static_import` rows, and every positional base of every class
//! with the `inherits` rows. The library's tree is read here, node by node, apart from the element table the readers read: a reader
//! that loses, adds or misnames one of them is a difference.

use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
use ruff_python_ast::{AnyNodeRef, Expr, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows};

/// What the library's nodes say: each fact as text, in the order met.
#[derive(Default)]
struct Facts {
    /// The names of the functions defined, in the order met.
    functions: Vec<String>,
    /// The modules imported, in the order met.
    imports: Vec<String>,
    /// The bases of the classes defined, in the order met.
    bases: Vec<String>,
}

/// A name the library writes, or the reason it cannot be resolved, as the rows show it.
fn shown(cell: Option<&Cell>) -> String {
    match cell {
        Some(Cell::Name(text)) => format!("name({text})"),
        Some(Cell::Unknown(reason)) => format!("unknown({reason})"),
        Some(other) => format!("{other}"),
        None => "nothing".to_owned(),
    }
}

/// The dotted name an expression writes: a name, or attributes of one.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name and an attribute write a dotted name; every other expression writes none"
)]
fn dotted(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attribute) => {
            dotted(&attribute.value).map(|value| format!("{value}.{}", attribute.attr.id))
        }
        _ => None,
    }
}

impl<'a> SourceOrderVisitor<'a> for Facts {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "a base that is no starred or subscripted expression is read as a dotted name or as dynamic, whatever its kind"
    )]
    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        if let AnyNodeRef::StmtFunctionDef(function) = node {
            self.functions.push(format!("name({})", function.name.id));
        }
        if let AnyNodeRef::StmtImport(import) = node {
            for alias in &import.names {
                let target = alias.name.id.to_string();
                let binding = alias.asname.as_ref().map_or_else(
                    || target.split('.').next().unwrap_or_default().to_owned(),
                    |asname| asname.id.to_string(),
                );
                self.imports.push(format!("name({binding}) name({target})"));
            }
        }
        if let AnyNodeRef::StmtImportFrom(import) = node {
            let dots = ".".repeat(usize::try_from(import.level).unwrap_or(0));
            let module = import
                .module
                .as_ref()
                .map(|module| module.id.to_string())
                .unwrap_or_default();
            for alias in &import.names {
                if alias.name.id.as_str() == "*" {
                    self.imports
                        .push(format!("unknown(wildcard-import) name({dots}{module})"));
                    continue;
                }
                let binding = alias
                    .asname
                    .as_ref()
                    .map_or_else(|| alias.name.id.to_string(), |asname| asname.id.to_string());
                let target = if module.is_empty() {
                    format!("{dots}{}", alias.name.id)
                } else {
                    format!("{dots}{module}.{}", alias.name.id)
                };
                self.imports.push(format!("name({binding}) name({target})"));
            }
        }
        if let AnyNodeRef::StmtClassDef(class) = node
            && let Some(arguments) = &class.arguments
        {
            for base in &arguments.args {
                let target = match base {
                    Expr::Starred(_) => "unknown(star-base)".to_owned(),
                    Expr::Subscript(subscript) if subscript.slice.is_number_literal_expr() => {
                        "unknown(dynamic-base)".to_owned()
                    }
                    Expr::Subscript(subscript) => dotted(&subscript.value).map_or_else(
                        || "unknown(dynamic-base)".to_owned(),
                        |name| format!("name({name})"),
                    ),
                    other => dotted(other).map_or_else(
                        || "unknown(dynamic-base)".to_owned(),
                        |name| format!("name({name})"),
                    ),
                };
                self.bases.push(target);
            }
        }
        TraversalSignal::Traverse
    }
}

/// What differs between the facts the library gives and the ones the rows give, for one kind of fact.
fn compare(what: &str, mut library: Vec<String>, mut rows: Vec<String>, found: &mut Vec<String>) {
    library.sort();
    rows.sort();
    if library != rows {
        found.push(format!(
            "the {what} the library gives are {library:?}, the rows give {rows:?}"
        ));
    }
}

/// Compares the library's function definitions, imports and class bases of `text` with the rows; what differs, one sentence for each
/// kind of fact; nothing when they agree.
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let options = ParseOptions::from(Mode::Module).with_target_version(PythonVersion::latest());
    let parsed = parse_unchecked(text, options);
    let mut facts = Facts::default();
    facts.visit_mod(parsed.syntax());
    let mut found = Vec::new();
    compare(
        "functions",
        facts.functions,
        rows.of(Relation::Function)
            .iter()
            .map(|row| shown(row.get(1)))
            .collect(),
        &mut found,
    );
    compare(
        "imports",
        facts.imports,
        rows.of(Relation::StaticImport)
            .iter()
            .map(|row| format!("{} {}", shown(row.get(2)), shown(row.get(3))))
            .collect(),
        &mut found,
    );
    compare(
        "class bases",
        facts.bases,
        rows.of(Relation::Inherits)
            .iter()
            .map(|row| shown(row.get(2)))
            .collect(),
        &mut found,
    );
    found
}
