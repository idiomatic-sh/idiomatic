//! Syntax and packaging facts about Python statements and imports.
use super::{
    finish::{self, Run, Table, Ty},
    provider_packaging::Project,
};
use ryo::languages::{NodeTable, Position, node_table};
use ryo_python::fixtures::columns::ColumnFile;

pub(super) const NEIGHBORS: &str = "statement-neighbors";
pub(super) const HEADS: &str = "import-head";
pub(super) const STATEMENTS: &str = "import-statement";
pub(super) const ALIASES: &str = "import-alias-parts";
pub(super) const CONTEXTUAL_ALIASES: &str = "import-alias-context";
pub(super) const LINE_COMMENTS: &str = "import-line-comment";
pub(super) const ENTRIES: &str = "import-module-context";

pub(super) fn declare(run: &mut Run) {
    use Ty::{File, Node, Num, Span, Sym};
    let neighbors = &[
        ("id", Node),
        ("parent", Node),
        ("field", Sym),
        ("kind", Sym),
        ("span", Span),
        ("parent-kind", Sym),
        ("previous", Node),
        ("previous-kind", Sym),
        ("previous-gap", Span),
        ("has-previous", Num),
        ("next", Node),
        ("next-kind", Sym),
        ("has-next", Num),
        ("gap", Span),
        ("first", Node),
        ("last", Node),
        ("suite-span", Span),
        ("file", File),
    ];
    let mut statements = neighbors.to_vec();
    statements.extend([
        ("gap-text", Sym),
        ("previous-gap-text", Sym),
        ("statement-text", Sym),
        ("run-first", Node),
        ("run-last", Node),
        ("run-span", Span),
    ]);
    let aliases = &[
        ("alias", Node),
        ("statement", Node),
        ("index", Num),
        ("count", Num),
        ("renames", Num),
        ("form", Sym),
        ("module", Sym),
        ("dots", Sym),
        ("name", Sym),
        ("rename", Sym),
        ("written-name", Sym),
        ("written-rename", Sym),
        ("last-segment", Sym),
        ("bound-name", Sym),
        ("target", Sym),
        ("written-module", Sym),
        ("root", Sym),
        ("alias-text", Sym),
        ("alias-span", Span),
        ("statement-span", Span),
        ("at", Num),
        ("start-line", Num),
        ("end-line", Num),
        ("file", File),
    ];
    let mut parts = aliases.to_vec();
    parts.extend([
        ("statement-start-line", Num),
        ("statement-end-line", Num),
        ("statement-text", Sym),
        ("previous-start-line", Num),
    ]);
    let mut heads = statements.clone();
    heads.extend([
        ("head-alias", Node),
        ("head-at", Num),
        ("count", Num),
        ("renames", Num),
        ("module", Sym),
        ("written-module", Sym),
        ("written-name", Sym),
        ("written-rename", Sym),
        ("dots", Sym),
        ("level", Num),
        ("root", Sym),
        ("written-root", Sym),
        ("catalog", Sym),
        ("source-module-prefix", Sym),
        ("source-package-prefix", Sym),
        ("module-namespace", Num),
        ("package-root", Sym),
        ("statement-at", Num),
        ("statement-end", Num),
        ("statement-start-line", Num),
        ("statement-end-line", Num),
    ]);
    let mut contextual = aliases.to_vec();
    contextual.extend([
        ("written-root", Sym),
        ("level", Num),
        ("catalog", Sym),
        ("module-prefix", Sym),
        ("package-prefix", Sym),
        ("module-namespace", Num),
        ("package-root", Sym),
        ("statement-at", Num),
        ("statement-end", Num),
        ("statement-start-line", Num),
        ("statement-end-line", Num),
        ("statement-text", Sym),
    ]);
    for (rel, columns) in [
        (NEIGHBORS, &neighbors[..]),
        (STATEMENTS, &statements[..]),
        (ALIASES, &parts[..]),
        (HEADS, &heads[..]),
        (
            LINE_COMMENTS,
            &[
                ("statement", Node),
                ("comment", Node),
                ("first-line", Num),
                ("following", Num),
                ("prefix", Span),
                ("gap", Span),
                ("text", Sym),
                ("file", File),
            ][..],
        ),
        (CONTEXTUAL_ALIASES, &contextual[..]),
        (
            ENTRIES,
            &[
                ("alias", Node),
                ("module", Sym),
                ("root", Sym),
                ("written-root", Sym),
                ("form", Sym),
                ("level", Num),
                ("catalog", Sym),
                ("module-prefix", Sym),
                ("package-prefix", Sym),
                ("module-namespace", Num),
                ("package-root", Sym),
                ("file", File),
            ][..],
        ),
    ] {
        run.declare(rel, columns, finish::Supplier::Provider);
    }
}

fn push<const N: usize>(t: &mut Table, row: [u64; N]) {
    for (c, v) in t.cols.iter_mut().zip(row) {
        c.push(v);
    }
    t.n += 1;
}

fn bounds(table: &NodeTable, id: Position) -> (u32, u32) {
    let span = node_table::span_of(table, id);
    (span.start.0, span.end.0)
}

fn span(start: u32, end: u32) -> u64 {
    (u64::from(start) << 32) | u64::from(end)
}

/// The directory's top regular package, as in the module-identity relation.
fn package_root(run: &mut Run, project: &mut Project, number: u32) -> u64 {
    let path = std::path::Path::new(&run.paths[number as usize]);
    let package = super::provider_packaging::module_stem(path)
        .and_then(|_| path.parent())
        .and_then(|dir| project.package(dir));
    u64::from(run.sym(package.map_or("", |p| p.root_name.as_str()).as_bytes()))
}

struct ModuleContext {
    written_root: u64,
    catalog: u64,
    module_prefix: u64,
    package_prefix: u64,
    namespace: bool,
}

/// A Python path may pass through a regular package or a namespace before it reaches the named member.
/// Record observations of path entries and catalog membership separately.
fn module_context(
    run: &mut Run,
    project: &mut Project,
    file: &ColumnFile<'_, '_>,
    a: &ryo_python::fixtures::columns::ImportAlias<'_>,
    target: &str,
    root: (&str, u64),
    catalog: &mut std::collections::HashMap<u64, u64>,
) -> ModuleContext {
    let empty = u64::from(run.empty);
    let (root, root_id) = root;
    let catalog_id = *catalog
        .entry(root_id)
        .or_insert_with(|| u64::from(run.sym(file.import_catalog(root).as_bytes())));
    let mut module = empty;
    let mut package = empty;
    let mut namespace = false;
    if a.dots.is_empty() && !target.is_empty() {
        for end in target
            .match_indices('.')
            .map(|(i, _)| i)
            .chain(std::iter::once(target.len()))
        {
            let path = &target[..end];
            for kind in project.observed_path_kinds(path) {
                match kind {
                    "module" => module = u64::from(run.sym(path.as_bytes())),
                    "package" => package = u64::from(run.sym(path.as_bytes())),
                    "namespace" if path == a.module => namespace = true,
                    _ => {}
                }
            }
        }
    }
    let written_root = run.sym(
        a.written_module
            .split('.')
            .next()
            .unwrap_or_default()
            .as_bytes(),
    );
    ModuleContext {
        written_root: written_root.into(),
        catalog: catalog_id,
        module_prefix: module,
        package_prefix: package,
        namespace,
    }
}

fn line_starts(src: &[u8]) -> Vec<u32> {
    let mut starts = vec![0];
    for (i, &byte) in src.iter().enumerate() {
        if byte == b'\n' || (byte == b'\r' && src.get(i + 1) != Some(&b'\n')) {
            starts.push(i as u32 + 1);
        }
    }
    starts
}

pub(super) fn fill(
    run: &mut Run,
    file: &ColumnFile<'_, '_>,
    src: &[u8],
    project: &mut Project,
    number: u32,
    rel: &str,
) -> Result<Table, String> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("missing {rel}"))?;
    let mut t = Table::with_rows(names, tys, 0);
    let f = u64::from(number);
    let table = file.table;
    if rel == HEADS {
        // A statement has one first alias; its source module is the plain path or the from-clause, independently of the imported members.
        let mut layout = fill(run, file, src, project, number, STATEMENTS)?;
        t.n = layout.n;
        let extension = layout.cols.len();
        t.cols = std::mem::take(&mut layout.cols);
        t.cols
            .resize_with(t.names.len(), || vec![finish::NULL; t.n]);
        if t.n == 0 {
            return Ok(t);
        }
        let by_statement: rustc_hash::FxHashMap<_, _> = t.cols[0]
            .iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        let starts = line_starts(src);
        let containing_package = package_root(run, project, number);
        let mut catalog = std::collections::HashMap::new();
        file.visit_import_heads(|a| {
            let Some(&row) = by_statement.get(&u64::from(a.statement)) else {
                return;
            };
            let (begin, end) = bounds(table, Position(a.statement));
            let root = a.module.split('.').next().unwrap_or_default();
            let root_id = u64::from(run.sym(root.as_bytes()));
            let observed = module_context(
                run,
                project,
                file,
                &a,
                a.module,
                (root, root_id),
                &mut catalog,
            );
            let mut sym = |s: &str| u64::from(run.sym(s.as_bytes()));
            let values = [
                a.id.into(),
                bounds(table, Position(a.id)).0.into(),
                a.count as u64,
                a.renames as u64,
                sym(a.module),
                sym(a.written_module),
                sym(a.written_name),
                sym(a.written_rename),
                sym(a.dots),
                a.dots.len() as u64,
                root_id,
                observed.written_root,
                observed.catalog,
                observed.module_prefix,
                observed.package_prefix,
                observed.namespace.into(),
                containing_package,
                begin.into(),
                end.into(),
                starts.partition_point(|s| *s <= begin) as u64,
                starts.partition_point(|s| *s <= end) as u64,
            ];
            for (column, v) in t.cols[extension..].iter_mut().zip(values) {
                column[row] = v;
            }
        });
    } else if matches!(rel, NEIGHBORS | STATEMENTS) {
        let plain = run.sym(b"import");
        let from = run.sym(b"import-from");
        let plain_code = run.kind_sym.iter().position(|k| *k == plain);
        let from_code = run.kind_sym.iter().position(|k| *k == from);
        let kind =
            |run: &Run, id: Position| run.kind_sym[usize::from(node_table::kind_of(table, id).0)];
        let import = |id| {
            let code = Some(usize::from(node_table::kind_of(table, id).0));
            code == plain_code || code == from_code
        };
        // Source order in each suite fixes both neighbors. A run contains consecutive import statements regardless of intervening trivia.
        file.visit_statement_suites(rel == STATEMENTS, |suite| {
            let first = suite[0];
            let last = suite[suite.len() - 1];
            let suite_span = span(bounds(table, first).0, bounds(table, last).1);
            let parent = node_table::parent_of(table, first);
            let field = run.field_sym[usize::from(node_table::field_of(table, first).0)];
            let mut start = 0;
            while start < suite.len() {
                let mut end = start + 1;
                if import(suite[start]) {
                    while end < suite.len() && import(suite[end]) {
                        end += 1;
                    }
                }
                let head = suite[start];
                let tail = suite[end - 1];
                let run_span = span(bounds(table, head).0, bounds(table, tail).1);
                for i in start..end {
                    let id = suite[i];
                    if rel == STATEMENTS && !import(id) {
                        continue;
                    }
                    let (begin, finish) = bounds(table, id);
                    let previous = i.checked_sub(1).map(|p| suite[p]);
                    let next = suite.get(i + 1).copied();
                    push(
                        &mut t,
                        [
                            id.0.into(),
                            parent.0.into(),
                            field.into(),
                            kind(run, id).into(),
                            span(begin, finish),
                            kind(run, parent).into(),
                            previous.map_or(finish::NULL, |p| p.0.into()),
                            previous.map_or(finish::NULL, |p| kind(run, p).into()),
                            previous.map_or(finish::NULL, |p| span(bounds(table, p).1, begin)),
                            u64::from(previous.is_some()),
                            next.map_or(finish::NULL, |p| p.0.into()),
                            next.map_or(finish::NULL, |p| kind(run, p).into()),
                            u64::from(next.is_some()),
                            next.map_or(finish::NULL, |p| span(finish, bounds(table, p).0)),
                            first.0.into(),
                            last.0.into(),
                            suite_span,
                            f,
                        ],
                    );
                    if rel == STATEMENTS {
                        let mut spelling = |lo: u32, hi: u32| {
                            u64::from(
                                run.sym(src.get(lo as usize..hi as usize).unwrap_or_default()),
                            )
                        };
                        let gap =
                            next.map_or(finish::NULL, |p| spelling(finish, bounds(table, p).0));
                        let before =
                            previous.map_or(finish::NULL, |p| spelling(bounds(table, p).1, begin));
                        let text = spelling(begin, finish);
                        for (c, v) in t.cols.iter_mut().rev().zip(
                            [gap, before, text, head.0.into(), tail.0.into(), run_span]
                                .into_iter()
                                .rev(),
                        ) {
                            c.push(v);
                        }
                    }
                }
                start = end;
            }
        });
    } else if rel == LINE_COMMENTS {
        // A comment beginning on a line touched by the import has a source prefix and, when it follows the statement, an intervening gap.
        file.visit_import_line_comments(|statement, comment, first_line| {
            let (start, end) = bounds(table, statement);
            let (at, until) = bounds(table, comment);
            let following = end <= at;
            let text = run.sym(&src[at as usize + 1..until as usize]);
            push(
                &mut t,
                [
                    statement.0.into(),
                    comment.0.into(),
                    first_line.into(),
                    following.into(),
                    span(start, at),
                    if following {
                        span(end, at)
                    } else {
                        finish::NULL
                    },
                    text.into(),
                    f,
                ],
            );
        });
    } else if rel == ENTRIES {
        let package_root = package_root(run, project, number);
        let mut catalog = std::collections::HashMap::new();
        file.visit_import_aliases(|a| {
            let root = a.module.split('.').next().unwrap_or_default();
            let root_id = u64::from(run.sym(root.as_bytes()));
            let context = module_context(
                run,
                project,
                file,
                &a,
                a.target,
                (root, root_id),
                &mut catalog,
            );
            push(
                &mut t,
                [
                    a.id.into(),
                    run.sym(a.module.as_bytes()).into(),
                    root_id,
                    context.written_root,
                    run.sym(a.form.as_bytes()).into(),
                    a.dots.len() as u64,
                    context.catalog,
                    context.module_prefix,
                    context.package_prefix,
                    context.namespace.into(),
                    package_root,
                    f,
                ],
            );
        });
    } else {
        let contextual = rel == CONTEXTUAL_ALIASES;
        let mut catalog = std::collections::HashMap::new();
        let mut containing_package = None;
        let mut statement_text = 0;
        let starts = line_starts(src);
        let mut previous_statement = None;
        let mut statement_lines = [0; 2];
        let line = |byte| starts.partition_point(|&s| s <= byte) as u64;
        // An alias has a parser name and an optional as-name, each with its own original source spelling.
        file.visit_import_aliases(|a| {
            let (at, end) = bounds(table, Position(a.id));
            let (st, en) = bounds(table, Position(a.statement));
            let mut sym = |s: &str| u64::from(run.sym(s.as_bytes()));
            let root = a.module.split('.').next().unwrap_or_default();
            let root_id = sym(root);
            let alias_text =
                std::str::from_utf8(src.get(at as usize..end as usize).unwrap_or_default())
                    .unwrap_or_default();
            push(
                &mut t,
                [
                    a.id.into(),
                    a.statement.into(),
                    a.index as u64,
                    a.count as u64,
                    a.renames as u64,
                    sym(a.form),
                    sym(a.module),
                    sym(a.dots),
                    sym(a.name),
                    sym(a.rename),
                    sym(a.written_name),
                    sym(a.written_rename),
                    sym(a.name.rsplit('.').next().unwrap_or_default()),
                    sym(if !a.rename.is_empty() {
                        a.rename
                    } else if a.form == "import" {
                        root
                    } else if a.name == "*" {
                        ""
                    } else {
                        a.name
                    }),
                    sym(a.target),
                    sym(a.written_module),
                    root_id,
                    sym(alias_text),
                    span(at, end),
                    span(st, en),
                    at.into(),
                    line(at),
                    line(end),
                    f,
                ],
            );
            // Every alias shares its enclosing statement's extent; its preceding name is fixed by the ordered alias list.
            if previous_statement != Some(a.statement) {
                previous_statement = Some(a.statement);
                statement_text = u64::from(run.sym(&src[st as usize..en as usize]));
                statement_lines = [line(st), line(en)];
            }
            if contextual {
                let context = module_context(
                    run,
                    project,
                    file,
                    &a,
                    a.target,
                    (root, root_id),
                    &mut catalog,
                );
                let package_root =
                    *containing_package.get_or_insert_with(|| package_root(run, project, number));
                let extra = [
                    context.written_root,
                    a.dots.len() as u64,
                    context.catalog,
                    context.module_prefix,
                    context.package_prefix,
                    context.namespace.into(),
                    package_root,
                    st.into(),
                    en.into(),
                    statement_lines[0],
                    statement_lines[1],
                    statement_text,
                ];
                for (col, v) in t.cols.iter_mut().rev().zip(extra.into_iter().rev()) {
                    col.push(v);
                }
            } else {
                let extra = [
                    statement_lines[0],
                    statement_lines[1],
                    statement_text,
                    a.previous_start.map_or(finish::NULL, line),
                ];
                for (col, v) in t.cols.iter_mut().rev().zip(extra.into_iter().rev()) {
                    col.push(v);
                }
            }
        });
    }
    Ok(t)
}
