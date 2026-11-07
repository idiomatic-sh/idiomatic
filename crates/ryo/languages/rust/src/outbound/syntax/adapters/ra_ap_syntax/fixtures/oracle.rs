//! The library's own semantic model of a file, rebuilt from its items and compared with the rows the language wrote: what the oracle rail
//! of the conformance holds `function`, `static_import` and `syntax_error` to.

use ra_ap_syntax::ast::{self, HasName as _};
use ra_ap_syntax::{AstNode as _, Edition, SourceFile};
use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows, Point, point_of};

/// The leaves of a `use` tree as the library reads them: the name each binds and the path each imports.
fn leaves(tree: &ast::UseTree, prefix: &[String], out: &mut Vec<(String, String)>) {
    let mut segments: Vec<String> = prefix.to_vec();
    if let Some(path) = tree.path() {
        let mut own: Vec<String> = std::iter::successors(Some(path), ast::Path::qualifier)
            .filter_map(|each| each.segment())
            .map(|segment| segment.syntax().text().to_string())
            .collect();
        own.reverse();
        // A `::` inside a group names nothing the group does not: only the path that starts the `use` is rooted.
        if !prefix.is_empty()
            && let Some(first) = own.first_mut()
            && let Some(rest) = first.strip_prefix("::")
        {
            *first = rest.to_owned();
        }
        segments.extend(own);
    }
    if let Some(list) = tree.use_tree_list() {
        for inner in list.use_trees() {
            leaves(&inner, &segments, out);
        }
        return;
    }
    let glob = tree.star_token().is_some();
    if segments.is_empty() && prefix.is_empty() && tree.coloncolon_token().is_some() {
        segments.push(String::new());
    }
    if segments.last().is_some_and(|last| last == "self") && segments.len() > 1 {
        segments.truncate(segments.len().saturating_sub(1));
    }
    if segments.is_empty() && !glob {
        return;
    }
    let binding = match tree.rename() {
        Some(rename) => rename
            .name()
            .map_or_else(|| "_".to_owned(), |name| name.text().to_owned()),
        None if glob => "*".to_owned(),
        None => segments
            .last()
            .map(|last| last.trim_start_matches("::").to_owned())
            .unwrap_or_default(),
    };
    if glob {
        segments.push("*".to_owned());
    }
    out.push((binding, segments.join("::")));
}

/// What differs between the library's `expected` and the rows' `found`, as sentences about `what`.
fn differences<T>(what: &str, mut expected: Vec<T>, mut found: Vec<T>) -> Vec<String>
where
    T: Ord + std::fmt::Debug,
{
    expected.sort();
    found.sort();
    if expected == found {
        return Vec::new();
    }
    let missing = expected.iter().find(|each| !found.contains(each));
    let extra = found.iter().find(|each| !expected.contains(each));
    vec![format!(
        "{what}: the library has {} and the rows have {}; first only in the library: {missing:?}; first only in the rows: {extra:?}",
        expected.len(),
        found.len()
    )]
}

/// The text of a name cell.
fn name_of(cell: Option<&Cell>) -> String {
    match cell {
        Some(Cell::Name(text)) => text.clone(),
        _ => String::new(),
    }
}

/// The place of a span cell.
fn span_of(cell: Option<&Cell>) -> (Point, Point) {
    match cell {
        Some(Cell::Span(from, to)) => (*from, *to),
        _ => (Point { line: 0, column: 0 }, Point { line: 0, column: 0 }),
    }
}

/// A function as both sides give it: its name, where it starts and where it ends, each as a line and a column.
type Spanned = (String, (u32, u32), (u32, u32));

/// What differs between the library's items of `text` and `rows`.
#[must_use]
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let parse = SourceFile::parse(text, Edition::CURRENT);
    let file = parse.tree();
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let functions: Vec<Spanned> = file
        .syntax()
        .descendants()
        .filter_map(ast::Fn::cast)
        .filter_map(|each| {
            let name = each.name()?.text().to_owned();
            let range = each.syntax().text_range();
            let (from, to) = (
                point_of(bytes, u32::from(range.start())),
                point_of(bytes, u32::from(range.end())),
            );
            Some((name, (from.line, from.column), (to.line, to.column)))
        })
        .collect();
    let rowed: Vec<Spanned> = rows
        .of(Relation::Function)
        .iter()
        .map(|row| {
            let (from, to) = span_of(row.get(3));
            (
                name_of(row.get(1)),
                (from.line, from.column),
                (to.line, to.column),
            )
        })
        .collect();
    found.extend(differences("function", functions, rowed));
    let mut imports = Vec::new();
    for item in file.syntax().descendants().filter_map(ast::Use::cast) {
        if let Some(tree) = item.use_tree() {
            leaves(&tree, &[], &mut imports);
        }
    }
    let import_rows: Vec<(String, String)> = rows
        .of(Relation::StaticImport)
        .iter()
        .map(|row| (name_of(row.get(2)), name_of(row.get(3))))
        .collect();
    found.extend(differences("static_import", imports, import_rows));
    // The library also validates what the parser accepts (and says so as errors): the rows hold the parser's errors, never more.
    let (reported, wrote) = (parse.errors().len(), rows.of(Relation::SyntaxError).len());
    if wrote > reported {
        found.push(format!(
            "syntax_error: the rows have {wrote} errors and the library {reported}"
        ));
    }
    found
}
