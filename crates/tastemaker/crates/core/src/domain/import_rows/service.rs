//! The `import_rows` service: free functions named by verb that write the query of each built-in rule over an import
//! relation, keep the rows it answers and read from them the imports the files make, with no IO and no ports.

use std::collections::BTreeMap;

use super::constants::{
    BINDING, BLOCK_KIND, FILE_COLUMN, KIND, MODULE_KIND, NAME_KEY, NAMES_RELATION, OWNER_KEY,
    OWNERS_RELATION, PACKAGE, PACKAGE_KEY, PACKAGES_RELATION, PATH_COLUMN, SOURCE_COLUMN, TARGET,
    WRITTEN,
};
use crate::domain::applications::types::CellText;
use crate::domain::comments::types::ByteSpan;
use crate::domain::contracts::types::Holding;
use crate::domain::definitions::types::QueryText;
use crate::domain::imports::constants::JOINERS;
use crate::domain::imports::types::{
    Import, ImportColumn, ImportPlace, ImportRows, ModuleName, Name, Owner, Package,
    PackageBinding, Relation, Scope, ScopeKind, Target, WrittenPath,
};
use crate::domain::module_paths::service::segments;
use crate::domain::paths::types::RepoPath;

/// The query of the built-in rule that reads `relation`, every column of each row.
#[must_use]
pub fn query(relation: Relation) -> QueryText {
    let (name, subject, key) = match relation {
        Relation::Edges => return QueryText(super::constants::SEMANTIC_EDGES.into()),
        Relation::Scopes => return QueryText(super::constants::SCOPES.into()),
        Relation::Names => (NAMES_RELATION, FILE_COLUMN, NAME_KEY),
        Relation::Packages => (PACKAGES_RELATION, SOURCE_COLUMN, PACKAGE_KEY),
        Relation::Owners => (OWNERS_RELATION, PATH_COLUMN, OWNER_KEY),
    };
    QueryText(
        format!(
            "let rows = select * from \"{name}\";\nemit rows subject \"{subject}\" key({key});\n"
        )
        .into(),
    )
}

/// One row of a built-in rule over `relation`, its file `path` and its columns read by `column`, kept with `rows`. A
/// column that names a file names it relative to the lint call's root, `base`, and is kept as the repository's path.
pub fn keep(
    rows: &mut ImportRows,
    relation: Relation,
    path: RepoPath,
    base: &RepoPath,
    column: &dyn Fn(ImportColumn<'_>) -> Option<CellText>,
    place: ImportPlace,
) {
    let file = |name| {
        column(ImportColumn(name)).map(|cell| {
            if cell.0.is_empty() {
                RepoPath(cell.0)
            } else {
                crate::domain::linting::service::repository_path(base, RepoPath(cell.0))
            }
        })
    };
    match relation {
        Relation::Edges => {
            if let Some(target) = file(TARGET) {
                rows.edges.push(Import {
                    source: path,
                    span: place,
                    written: column(ImportColumn(WRITTEN))
                        .map(|cell| WrittenPath(cell.0))
                        .unwrap_or_default(),
                    target: Target::File(target),
                });
            }
        }
        Relation::Names => {
            if let Some(written) = column(ImportColumn(TARGET)) {
                rows.names.push(Name {
                    source: path,
                    written: WrittenPath(written.0),
                    binding: column(ImportColumn(BINDING)).map(|binding| ModuleName(binding.0)),
                    span: place,
                });
            }
        }
        Relation::Packages => {
            // A package the repository does not hold has an empty target: no file of it is reached, so an import of it is a module's.
            if let (Some(target), Some(binding)) = (
                file(TARGET).filter(|target| !target.0.is_empty()),
                column(ImportColumn(BINDING)),
            ) {
                rows.packages.push(Package {
                    source: path,
                    target,
                    binding: PackageBinding(binding.0),
                });
            }
        }
        Relation::Owners => {
            if let Some(package) = file(PACKAGE) {
                rows.owners.push(Owner { path, package });
            }
        }
        Relation::Scopes => {
            let kind = match column(ImportColumn(KIND)).as_ref().map(|cell| &*cell.0) {
                Some(BLOCK_KIND) => ScopeKind::Block,
                Some(MODULE_KIND) => ScopeKind::Module,
                _ => return,
            };
            let scope = Scope { span: place, kind };
            match rows.scopes.get_mut(&path) {
                Some(scopes) => scopes.push(scope),
                None => {
                    let _: Option<Vec<Scope>> = rows.scopes.insert(path, vec![scope]);
                }
            }
        }
    }
}

/// The names a package binding or a written path is written with, each joiner read as the first.
fn written_names(text: &WrittenPath) -> Vec<ModuleName> {
    segments(text)
        .into_iter()
        .map(|segment| {
            ModuleName(
                segment
                    .0
                    .replace(JOINERS[1], &JOINERS[0].to_string())
                    .into(),
            )
        })
        .collect()
}

/// Whether `inner` lies within `outer`.
fn holds(outer: ImportPlace, inner: ImportPlace) -> Holding {
    match (outer, inner) {
        (ImportPlace::At(outer), ImportPlace::At(inner))
            if outer.start <= inner.start && inner.end <= outer.end =>
        {
            Holding::Holds
        }
        (ImportPlace::At(_) | ImportPlace::Unplaced, _) => Holding::Apart,
    }
}

/// The innermost of `sorted` (spans sorted by start, each two nested or apart) that holds `span`: of those that start at
/// or before it, found by bisection, the latest to start that also ends at or after it.
fn innermost(sorted: &[ByteSpan], span: ImportPlace) -> Option<ByteSpan> {
    let ImportPlace::At(span) = span else {
        return None;
    };
    let before = sorted.partition_point(|scope| scope.start <= span.start);
    sorted
        .get(..before)
        .unwrap_or_default()
        .iter()
        .rev()
        .find(|scope| span.end <= scope.end)
        .copied()
}

/// Whether an import whose innermost scope is `around` (the whole file when `None`) binds where `edge` is written.
///
/// The rule: a `use` binds within the innermost block or inline module that holds it, and everything nested in that
/// scope sees it (items nested in a block are held by the block's span), except what lies in an inline module nested
/// in it, for a module sees no `use` written outside it. So `around` must hold the edge, and the innermost inline module
/// that holds the edge, `module`, must be `around` itself or lie outside it. A glob import binds no single name, so a
/// path through one is never matched to it and stays an import of its own.
fn in_scope(around: Option<ByteSpan>, module: Option<ByteSpan>, edge: ImportPlace) -> Holding {
    let within = |inner: ImportPlace| {
        around.map_or(Holding::Holds, |outer| holds(ImportPlace::At(outer), inner))
    };
    match module {
        Some(module)
            if Some(module) != around && within(ImportPlace::At(module)) == Holding::Holds =>
        {
            Holding::Apart
        }
        Some(_) | None => within(edge),
    }
}

/// Every import the files make, read from the rows of the import relations: each path that names another file of the
/// repository, written as the `use` that holds it writes it, or as the edge's `written` column says the file writes it
/// when no import of the file binds the name it starts at (a path through an import is that import's), and each other name an import brings in, which reaches the manifest of a package of the repository the file's own package
/// depends on by a binding its first names are written with, or a module outside it. A file's package is the one
/// `file-package` says, whatever language's manifest it is.
#[must_use]
pub fn imports(rows: &ImportRows) -> Vec<Import> {
    let mut names: BTreeMap<&RepoPath, Vec<&Name>> = BTreeMap::new();
    for name in &rows.names {
        names.entry(&name.source).or_default().push(name);
    }
    let mut edges: BTreeMap<&RepoPath, Vec<&Import>> = BTreeMap::new();
    for edge in rows
        .edges
        .iter()
        .filter(|edge| !matches!(&edge.target, Target::File(target) if *target == edge.source))
    {
        edges.entry(&edge.source).or_default().push(edge);
    }
    let owners = rows
        .owners
        .iter()
        .map(|owner| (&owner.path, &owner.package))
        .collect::<BTreeMap<_, _>>();
    let mut packages: BTreeMap<&RepoPath, Vec<(Vec<ModuleName>, &RepoPath)>> = BTreeMap::new();
    for package in &rows.packages {
        packages.entry(&package.source).or_default().push((
            written_names(&WrittenPath(package.binding.0.clone())),
            &package.target,
        ));
    }
    let mut found = Vec::new();
    for (source, paths) in &edges {
        let brought = names.get(source).map(Vec::as_slice).unwrap_or_default();
        let spans = |only: Option<ScopeKind>| {
            let mut spans = rows
                .scopes
                .get(*source)
                .into_iter()
                .flatten()
                .filter(|scope| only.is_none_or(|kind| scope.kind == kind))
                .filter_map(|scope| match scope.span {
                    ImportPlace::At(span) => Some(span),
                    ImportPlace::Unplaced => None,
                })
                .collect::<Vec<_>>();
            spans.sort();
            spans
        };
        let scopes = spans(None);
        let modules = spans(Some(ScopeKind::Module));
        // Each import's innermost scope, read once: the bindings a use-site path may start at, where each binds.
        let bindings = brought
            .iter()
            .filter_map(|name| Some((name.binding.as_ref()?, innermost(&scopes, name.span))))
            .collect::<Vec<_>>();
        for edge in paths {
            let held = brought
                .iter()
                .find(|name| holds(name.span, edge.span) == Holding::Holds);
            // A path written at a use site that starts at a name an import binds where the path is written reaches
            // through that import, which is judged where it is written: the path is no import of its own. An ignore of
            // that import therefore covers the path too: the path is the import, used, and an import judged once is
            // exempted once.
            if held.is_none()
                && segments(&edge.written).first().is_some_and(|first| {
                    let module = innermost(&modules, edge.span);
                    bindings.iter().any(|(binding, around)| {
                        *binding == first && in_scope(*around, module, edge.span) == Holding::Holds
                    })
                })
            {
                continue;
            }
            let text = held
                .map(|name| name.written.clone())
                .or_else(|| match edge.span {
                    ImportPlace::At(_) => Some(edge.written.clone()),
                    ImportPlace::Unplaced => None,
                })
                .unwrap_or_default();
            found.push(Import {
                source: (*source).clone(),
                span: edge.span,
                written: text,
                target: edge.target.clone(),
            });
        }
    }
    for (source, named) in &names {
        let resolved = edges.get(source).map(Vec::as_slice).unwrap_or_default();
        let depends = owners
            .get(source)
            .and_then(|manifest| packages.get(*manifest))
            .map(Vec::as_slice)
            .unwrap_or_default();
        for name in named {
            if resolved
                .iter()
                .any(|edge| holds(name.span, edge.span) == Holding::Holds)
            {
                continue;
            }
            let path = written_names(&name.written);
            let target = depends
                .iter()
                .filter(|(binding, _)| !binding.is_empty() && path.starts_with(binding))
                .max_by_key(|(binding, _)| binding.len())
                .map_or(Target::Module, |(_, target)| {
                    Target::File((*target).clone())
                });
            found.push(Import {
                source: (*source).clone(),
                span: name.span,
                written: name.written.clone(),
                target,
            });
        }
    }
    found.sort();
    found.dedup();
    found
}
