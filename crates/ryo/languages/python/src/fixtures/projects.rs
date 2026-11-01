//! Reading the manifests of a small repository with the language's own project contract and its toml adapter, for the tests of the
//! items of a manifest relation: every row the item writes, each with every column shown, and the causes for which the item says the
//! relation is read only in part.

use std::sync::Arc;

use crate::domain::links::types::Linked;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::factory::{pep621, pip, pipenv, poetry, uv};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::AvailableManifests;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;
use ryo::languages::{self, Project as _, Symbols};

/// What an item wrote for a repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reading {
    /// Every row, every column shown, in the order the item wrote them.
    pub rows: &'static [&'static str],
    /// The causes for which the relation is read only in part, once each.
    pub causes: &'static [&'static str],
}

/// What a row's columns are shown against: the names, and the files of the run in the order of their numbers.
pub(crate) struct Shown<'a> {
    /// The names.
    pub symbols: &'a Symbols,
    /// The paths of the files of the run, in the order of their paths.
    pub files: &'a [&'a str],
}

impl Shown<'_> {
    /// A name, as its text.
    fn name(&self, symbol: languages::Symbol) -> String {
        format!(
            "{:?}",
            languages::text_of(self.symbols, languages::ByteSlice(&[]), symbol)
                .map_or("", |text| text.0)
        )
    }

    /// A file, by its path.
    fn file(&self, file: languages::FileId) -> &str {
        usize::try_from(file.0)
            .ok()
            .and_then(|place| self.files.get(place))
            .copied()
            .unwrap_or_default()
    }
}

/// A row shown with every column.
pub(crate) trait Show {
    /// The row as text.
    fn show(&self, shown: &Shown<'_>) -> String;
}

impl Show for languages::PackageDependencyRow {
    /// Every column; the version requirement in `<..>` when there is one, with `!` when it admits one version, and each column of
    /// text after it that is not empty as `column=value`.
    fn show(&self, shown: &Shown<'_>) -> String {
        let mut written = String::new();
        for (column, value) in [
            ("branch", self.branch),
            ("tag", self.tag),
            ("revision", self.revision),
            ("interpreter", self.interpreter),
            ("catalog", self.catalog),
            ("replacement", self.replacement),
        ] {
            let value = shown.name(value);
            if value != "\"\"" {
                written.push(' ');
                written.push_str(column);
                written.push('=');
                written.push_str(&value);
            }
        }
        let requirement = shown.name(self.requirement);
        let version = if requirement == "\"\"" {
            String::new()
        } else if self.exact == languages::Flag::Raised {
            format!(" <{requirement}>!")
        } else {
            format!(" <{requirement}>")
        };
        format!(
            "{} > {} {} {} {:?} {} {:?} {:?}{version}{written}",
            shown.file(self.source),
            shown.file(self.target),
            shown.name(self.binding),
            shown.name(self.package),
            self.kind,
            shown.name(self.condition),
            self.optional,
            self.origin
        )
    }
}

impl Show for languages::WorkspaceMemberRow {
    fn show(&self, shown: &Shown<'_>) -> String {
        format!(
            "{} holds {}",
            shown.file(self.workspace),
            shown.file(self.package)
        )
    }
}

impl Show for languages::SourceDependencyRow {
    fn show(&self, shown: &Shown<'_>) -> String {
        format!(
            "{} > {} at {}..{}",
            shown.file(self.source),
            shown.file(self.target),
            self.span.start.0,
            self.span.end.0
        )
    }
}

impl Show for languages::AnalyzedSourceRow {
    fn show(&self, shown: &Shown<'_>) -> String {
        shown.file(self.path).to_owned()
    }
}

impl Show for languages::FilePackageRow {
    fn show(&self, shown: &Shown<'_>) -> String {
        format!("{} in {}", shown.file(self.path), shown.file(self.package))
    }
}

/// The rows an item writes, and why it says it reads in part.
struct Sink<Row> {
    rows: Vec<Row>,
    causes: Vec<String>,
}

impl<Row> languages::Rows<Row> for Sink<Row> {
    fn write(&mut self, row: Row) {
        self.rows.push(row);
    }

    fn partial(&mut self, cause: languages::ReportText<'_>, _missing: languages::Missing) {
        if !self.causes.iter().any(|each| each == cause.0) {
            self.causes.push(cause.0.to_owned());
        }
    }

    fn partial_at(
        &mut self,
        _file: languages::FileId,
        cause: languages::ReportText<'_>,
        missing: languages::Missing,
    ) {
        self.partial(cause, missing);
    }
}

/// The manifest adapters of a run over `available`, every manifest of its repository: one for each package manager.
fn adapters(available: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![
        Arc::new(pep621()),
        Arc::new(uv()),
        Arc::new(poetry()),
        Arc::new(pip(AvailableManifests(available.to_vec()))),
        Arc::new(pipenv()),
    ]
}

/// The notes the language's own parse leaves of the file at `path` whose text is `text`; none for a file it does not read.
#[must_use]
pub(crate) fn notes_of(path: &str, text: &str) -> languages::Bytes {
    let parse = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let (mut elements, mut symbols) = (languages::Elements::default(), Symbols::default());
    let built = languages::contract::build(
        &parse,
        languages::ParseRequest {
            path: languages::SourcePath(path),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(text.as_bytes()),
            notes: languages::NoteRequest::Wanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    );
    match built {
        Ok(()) => languages::contract::notes_of(&elements).clone(),
        Err(_) => languages::Bytes::default(),
    }
}

/// What the item writes for the repository whose files are `files`, each with its path and its text, shown with every column: the
/// run asks for the links between the files, and the language's own parse notes each.
///
/// # Panics
///
/// When the project contract refuses the request, which it never does for a repository.
#[must_use]
pub(crate) fn reading<Item>(files: &'static [(&'static str, &'static str)]) -> Reading
where
    <Item::Relation as languages::CatalogRelation>::Row: Show,
    Item: languages::ManifestItem<Derived = Linked>,
{
    let mut sorted: Vec<(&str, &str)> = files.to_vec();
    sorted.sort_unstable();
    let paths: Vec<&str> = sorted.iter().map(|(path, _)| *path).collect();
    let request = languages::ProjectRequest {
        repository: languages::Repository::Disk,
        root: languages::RootPath("".into()),
        manifests: files
            .iter()
            .map(|(path, text)| languages::ManifestFile {
                path: languages::FilePath((*path).into()),
                text: languages::ManifestText((*text).into()),
            })
            .collect(),
        files: paths
            .iter()
            .map(|path| languages::FilePath((*path).into()))
            .collect(),
        notes: sorted
            .iter()
            .map(|(path, text)| notes_of(path, text))
            .collect(),
        wanted: vec![
            languages::Relation::SourceDependency,
            languages::Relation::AnalyzedSource,
        ],
    };
    let contract = ProjectContract::new(
        adapters,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    let resolved = contract
        .resolve(&request)
        .expect("the manifests are resolved");
    let mut symbols = Symbols::default();
    let mut sink = Sink {
        rows: Vec::new(),
        causes: Vec::new(),
    };
    (Item::READER.read)(
        &languages::ManifestInput {
            request: &request,
            derived: &resolved.derived,
        },
        &mut symbols,
        &mut sink,
    );
    let shown = Shown {
        symbols: &symbols,
        files: &paths,
    };
    let rows: Vec<&'static str> = sink
        .rows
        .iter()
        .map(|row| &*Box::leak(row.show(&shown).into_boxed_str()))
        .collect();
    let causes: Vec<&'static str> = sink
        .causes
        .into_iter()
        .map(|cause| &*Box::leak(cause.into_boxed_str()))
        .collect();
    Reading {
        rows: Box::leak(rows.into_boxed_slice()),
        causes: Box::leak(causes.into_boxed_slice()),
    }
}

/// A repository of the shape the cases read: a workspace at the root that holds the packages below `packages/` but one, which two of
/// them need and one of which needs an outside package too.
pub(crate) const WORKSPACE: &[(&str, &str)] = &[
    (
        "pyproject.toml",
        "[project]\nname = \"Root_Pkg\"\ndependencies = [\"Lib-One>=1.0\", \"requests[security]>=2 ; python_version >= '3.9'\"]\n[project.optional-dependencies]\ndocs = [\"lib.two\"]\n[dependency-groups]\ndev = [\"pytest\", \"lib-one\"]\n[build-system]\nrequires = [\"setuptools>=61\"]\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\nexclude = [\"packages/skip\"]\n",
    ),
    (
        "packages/lib_one/pyproject.toml",
        "[project]\nname = \"lib-one\"\ndependencies = [\"numpy==2.1.0\"]\n",
    ),
    ("packages/lib_one/src/lib_one/__init__.py", ""),
    (
        "packages/lib.two/pyproject.toml",
        "[project]\nname = \"lib.two\"\ndependencies = [\"lib-one ; sys_platform == 'win32'\"]\n",
    ),
    (
        "packages/skip/pyproject.toml",
        "[project]\nname = \"skip\"\n",
    ),
    ("app.py", ""),
    ("README.md", ""),
];

/// The same repository with a manifest the tool cannot read in it.
pub(crate) const WITH_A_BROKEN_MANIFEST: &[(&str, &str)] = &[
    ("pyproject.toml", "[project]\nname = \"a\"\n"),
    ("b/pyproject.toml", "[project\nname = "),
    ("b/c.py", ""),
];

/// A monorepo of the shape the alpha reads end to end: a `[project]` package at the root that is a `uv` workspace holding one member,
/// and a `requirements-dev.txt` beside the root manifest that needs the member again, for development.
pub(crate) const MONOREPO: &[(&str, &str)] = &[
    (
        "pyproject.toml",
        "[project]\nname = \"app\"\ndependencies = [\"lib-one>=1\"]\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
    ),
    (
        "packages/lib_one/pyproject.toml",
        "[project]\nname = \"lib-one\"\n",
    ),
    ("packages/lib_one/src/lib_one/__init__.py", ""),
    (
        "requirements-dev.txt",
        "pytest\nlib-one ; sys_platform == 'win32'\n",
    ),
    ("app.py", ""),
];

/// A repository whose Python files import one another every way the links read: a module at the root (the flat layout), a package in
/// the `src` layout of a workspace member, a regular package and a namespace package, a module of a package and a name its
/// `__init__.py` re-exports, relative imports, a module outside the repository and two imports that name a module of the repository no
/// file is.
pub(crate) const IMPORTS: &[(&str, &str)] = &[
    (
        "pyproject.toml",
        "[project]\nname = \"app\"\n[tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
    ),
    (
        "packages/lib_one/pyproject.toml",
        "[project]\nname = \"lib-one\"\n",
    ),
    (
        "packages/lib_one/src/lib_one/__init__.py",
        "from .workers import run\nfrom . import workers\n",
    ),
    (
        "packages/lib_one/src/lib_one/workers.py",
        "from .. import nowhere\n",
    ),
    (
        "app.py",
        "import os\nimport lib_one\nfrom lib_one import workers\nfrom lib_one import run\nfrom ns.part import thing\nimport lib_one.absent\nfrom tools import cli\n",
    ),
    ("ns/part.py", ""),
    ("tools/__init__.py", ""),
    ("tools/cli.py", "import app\n"),
];
