//! Syntax and packaging facts about Python statements and imports.
use super::{
    finish::{self, Run, Table},
    provider_packaging::Project,
};
use ryo::languages::Position;
use ryo_python::fixtures::columns::ColumnFile;

pub(super) const NEIGHBORS: &str = "statement-neighbors";
pub(super) const HEADS: &str = "import-head";
pub(super) const STATEMENTS: &str = "import-statement";
pub(super) const ALIASES: &str = "import-alias-parts";
pub(super) const CONTEXTUAL_ALIASES: &str = "import-alias-context";
pub(super) const LINE_COMMENTS: &str = "import-line-comment";
pub(super) const ENTRIES: &str = "import-module-context";

#[derive(Default)]
pub(super) struct LineComments(Option<Vec<(Position, Position, bool)>>);

impl LineComments {
    pub(super) fn known_count(&self) -> Option<usize> {
        self.0.as_ref().map(Vec::len)
    }
}

pub(super) fn declare(run: &mut Run) {
    for schema in ColumnFile::IMPORT_SCHEMAS {
        let columns: Vec<_> = schema
            .columns
            .iter()
            .map(|&(name, kind)| (name, finish::ty_of(kind)))
            .collect();
        run.declare(schema.name, &columns, finish::Supplier::Provider);
    }
}
/// The directory's top regular package, as in the module-identity relation.
fn package_root(run: &mut Run, project: &mut Project, number: u32) -> u64 {
    let path = std::path::Path::new(&run.paths[number as usize]);
    let package = super::provider_packaging::module_stem(path)
        .and_then(|_| path.parent())
        .and_then(|dir| project.package(dir));
    u64::from(run.sym(package.map_or("", |p| p.root_name.as_str()).as_bytes()))
}

#[derive(Clone, Copy)]
pub(super) struct ModuleContext {
    written_root: u64,
    catalog: u64,
    module_prefix: u64,
    package_prefix: u64,
    namespace: bool,
}

// A canonical absolute module has the same project and release context at every import head.
fn head_context(
    run: &mut Run,
    project: &mut Project,
    file: &ColumnFile<'_, '_>,
    a: &ryo_python::fixtures::columns::ImportAlias<'_>,
) -> (u64, u64, ModuleContext) {
    let module = run.sym(a.module.as_bytes());
    let canonical = a.dots.is_empty() && a.written_module == a.module;
    let key = (file.import_release(), module);
    if canonical && let Some(&(root, context)) = project.head_modules.get(&key) {
        return (u64::from(module), root, context);
    }
    let root = a.module.split('.').next().unwrap_or_default();
    let root_id = u64::from(if root == a.module {
        module
    } else {
        run.sym(root.as_bytes())
    });
    let context = module_context(run, project, file, a, a.module);
    if canonical {
        project.head_modules.insert(key, (root_id, context));
    }
    (u64::from(module), root_id, context)
}

/// A Python path may pass through a regular package or a namespace before it reaches the named member.
/// Record observations of path entries and catalog membership separately.
fn module_context(
    run: &mut Run,
    project: &mut Project,
    file: &ColumnFile<'_, '_>,
    a: &ryo_python::fixtures::columns::ImportAlias<'_>,
    target: &str,
) -> ModuleContext {
    let [
        written_root,
        catalog,
        module_prefix,
        package_prefix,
        namespace,
    ] = file.import_context(
        a,
        target,
        &mut (run, project),
        |context, word| context.0.sym(word),
        |context, target| {
            let observed = path_context(context.0, context.1, target);
            (observed.module, observed.package, observed.namespace)
        },
    );
    ModuleContext {
        written_root,
        catalog,
        module_prefix,
        package_prefix,
        namespace: namespace != 0,
    }
}

#[derive(Clone, Copy)]
pub(super) struct ImportPrefix {
    module: u64,
    package: u64,
    namespace: bool,
}

// Import entries are immutable for this request. Prefixes and interned names share the project's lifetime.
fn path_context(run: &mut Run, project: &mut Project, target: &str) -> ImportPrefix {
    let target_id = run.sym(target.as_bytes());
    if let Some(&answer) = project.import_prefixes.get(&target_id) {
        return answer;
    }
    let empty = u64::from(run.empty);
    let mut answer = ImportPrefix {
        module: empty,
        package: empty,
        namespace: false,
    };
    for end in target
        .match_indices('.')
        .map(|(i, _)| i)
        .chain(std::iter::once(target.len()))
    {
        let prefix = &target[..end];
        let id = if end == target.len() {
            target_id
        } else {
            run.sym(prefix.as_bytes())
        };
        if let Some(&seen) = project.import_prefixes.get(&id) {
            answer = seen;
            continue;
        }
        answer.namespace = false;
        for kind in project.observed_path_kinds(prefix) {
            match kind {
                "module" => answer.module = id.into(),
                "package" => answer.package = id.into(),
                "namespace" => answer.namespace = true,
                _ => {}
            }
        }
        project.import_prefixes.insert(id, answer);
    }
    answer
}

pub(super) use ryo_python::fixtures::comment_positions::starts as line_starts;

pub(super) fn fill(
    run: &mut Run,
    file: &ColumnFile<'_, '_>,
    src: &[u8],
    project: &mut Project,
    number: u32,
    rel: &str,
    comments: &mut LineComments,
) -> Result<Table, String> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("missing {rel}"))?;
    let schema = ColumnFile::IMPORT_SCHEMAS
        .iter()
        .find(|schema| schema.name == rel)
        .ok_or_else(|| format!("no import relation {rel}"))?;
    let (n, cols) = file.import_columns(
        schema,
        src,
        number,
        &mut comments.0,
        &mut (run, project),
        |context, word| context.0.sym(word),
        |context| Ok(package_root(context.0, context.1, number)),
        |context, file, a, head| {
            let (run, project) = (&mut *context.0, &mut *context.1);
            let (module, root, observed) = if head {
                head_context(run, project, file, a)
            } else {
                let root = a.module.split('.').next().unwrap_or_default();
                let root_id = u64::from(run.sym(root.as_bytes()));
                (0, root_id, module_context(run, project, file, a, a.target))
            };
            (
                module,
                root,
                [
                    observed.written_root,
                    observed.catalog,
                    observed.module_prefix,
                    observed.package_prefix,
                    observed.namespace.into(),
                ],
            )
        },
    )?;
    Ok(Table {
        names,
        tys,
        cols,
        n,
    })
}
