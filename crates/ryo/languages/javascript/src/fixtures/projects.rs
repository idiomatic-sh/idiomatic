//! Small repositories whose files import one another, a reader harness for the items of the links, and the rows the project fixture
//! must give.

#![expect(
    clippy::expect_used,
    reason = "a fixture whose repository the project cannot resolve has nothing to read: the panic is the failure"
)]

use crate::domain::links::types::Linked;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::factory::{bun, npm, pnpm};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages::{self, Project as _};
use std::sync::Arc;

/// A workspace of npm and pnpm manifests, by path and text, that the project contract reads whole.
pub(crate) const WORKSPACE: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":{"packages":["packages/*","!packages/skip"]},"dependencies":{"core":"workspace:*","outside":"=4.6.0","alias":"npm:core@1"},"devDependencies":{"core":"*"},"peerDependencies":{"core":"*"},"optionalDependencies":{"util":"*"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    ("packages/core/index.js", "export const value = 1;"),
    ("packages/util/package.json", r#"{"name":"util"}"#),
    ("packages/skip/package.json", r#"{"name":"skip"}"#),
    ("index.js", "export {};"),
];

/// A repository whose files import one another every way the links read: a relative specifier with its ending and without, a
/// directory's `index`, a `require(...)` and an `import(...)`, a file of another language, a package and a module of Node, and two
/// imports that name a file of the repository no file is.
pub(crate) const IMPORTS: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"app"}"#),
    (
        "src/app.js",
        "import { a } from './a';\nimport { b } from '../lib/b.js';\nconst c = require('../lib/c');\nexport * from './dir';\nimport { react } from 'react';\nimport { readFile } from 'node:fs';\nimport view from './view.vue';\nconst m = import('./missing');\nimport { internal } from '#internal';\n",
    ),
    ("src/a.mjs", "export const a = 1;\n"),
    ("lib/b.js", ""),
    ("lib/c.cjs", "module.exports = {};\n"),
    ("src/dir/index.jsx", "import { app } from '../app';\n"),
    (
        "src/view.vue",
        "<template><p>hi</p></template>\n<script>\nexport default {};\n</script>\n",
    ),
];

/// A repository whose files import nothing.
pub(crate) const NOTHING: &[(&str, &str)] = &[("index.js", "export {};\n"), ("README.md", "")];

/// The notes the language's own parse leaves of the file at `path` whose text is `text`; none for a file it does not read.
#[must_use]
pub(crate) fn notes_of(path: &str, text: &str) -> languages::Bytes {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let built = languages::contract::build(
        crate::linked().parse_engine().as_ref(),
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

/// The request of a run over the repository whose files are `files`, each with its path and its text, that asks for the relations
/// `wanted`: the language's own parse notes each file.
pub(crate) fn request(
    files: &[(&str, &str)],
    wanted: Vec<languages::Relation>,
) -> languages::ProjectRequest {
    let mut files = files.to_vec();
    files.sort_by_key(|(path, _)| *path);
    languages::ProjectRequest {
        repository: languages::Repository::Disk,
        root: languages::RootPath(Box::default()),
        manifests: files
            .iter()
            .filter(|(path, _)| {
                path.ends_with("package.json") || path.ends_with("pnpm-workspace.yaml")
            })
            .map(|(path, text)| languages::ManifestFile {
                path: languages::FilePath((*path).into()),
                text: languages::ManifestText((*text).into()),
            })
            .collect(),
        files: files
            .iter()
            .map(|(path, _)| languages::FilePath((*path).into()))
            .collect(),
        notes: files
            .iter()
            .map(|(path, text)| notes_of(path, text))
            .collect(),
        wanted,
    }
}

/// The rows an item writes, and how many causes made it read the relation in part.
pub(crate) struct Written<Row> {
    /// The rows written, in order.
    pub rows: Vec<Row>,
    /// How many times the relation said its answer was partial.
    pub partial: usize,
}

impl<Row> languages::Rows<Row> for Written<Row> {
    fn write(&mut self, row: Row) {
        self.rows.push(row);
    }
    fn partial_at(
        &mut self,
        _file: languages::FileId,
        _cause: languages::ReportText<'_>,
        _missing: languages::Missing,
    ) {
        self.partial += 1;
    }
    fn partial(&mut self, _cause: languages::ReportText<'_>, _missing: languages::Missing) {
        self.partial += 1;
    }
}

/// What the item writes for the repository whose files are `files`, the run asking for the links between them, and the names it wrote.
pub(crate) fn read<Item>(
    files: &[(&str, &str)],
) -> (
    Written<<Item::Relation as languages::CatalogRelation>::Row>,
    languages::Symbols,
)
where
    Item: languages::ManifestItem<Derived = Linked>,
{
    let request = request(
        files,
        vec![
            languages::Relation::SourceDependency,
            languages::Relation::AnalyzedSource,
        ],
    );
    let project = ProjectContract::new(adapters(), Arc::new(postcard()));
    let resolved = project.resolve(&request).expect("project resolved");
    let mut rows = Written {
        rows: Vec::new(),
        partial: 0,
    };
    let mut symbols = languages::Symbols::default();
    (Item::READER.read)(
        &languages::ManifestInput {
            request: &request,
            derived: &resolved.derived,
        },
        &mut symbols,
        &mut rows,
    );
    (rows, symbols)
}

/// The text `symbol` stands for, or nothing when it stands for none.
pub(crate) fn shown(symbols: &languages::Symbols, symbol: languages::Symbol) -> String {
    languages::text_of(symbols, languages::ByteSlice(&[]), symbol)
        .map_or_else(String::new, |text| text.0.to_owned())
}

/// The manifest reading adapters the language reads manifests with, for the tests of the contracts over the real parsers.
pub(crate) fn adapters() -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![Arc::new(npm()), Arc::new(pnpm()), Arc::new(bun())]
}

/// The rows the project fixture in `manifests/imports/` must give.
pub(crate) fn expected() -> languages::contract::ManifestExpected {
    use languages::contract::{
        AnalyzedSourceExpectedRow, ManifestExpected, SourceDependencyExpectedRow, file, name, span,
    };
    ManifestExpected::of("imports")
        .rows(&[
            AnalyzedSourceExpectedRow {
                path: file("index.js"),
            },
            AnalyzedSourceExpectedRow {
                path: file("lib/helper.js"),
            },
            AnalyzedSourceExpectedRow {
                path: file("lib/util.cjs"),
            },
        ])
        .rows(&[
            SourceDependencyExpectedRow {
                source: file("index.js"),
                target: file("lib/helper.js"),
                span: span((1, 1), (1, 39)),
                written: name("import{helper}from'./lib/helper';"),
            },
            SourceDependencyExpectedRow {
                source: file("index.js"),
                target: file("lib/util.cjs"),
                span: span((2, 14), (2, 39)),
                written: name("require('./lib/util.cjs')"),
            },
        ])
}
