//! Small npm repositories and a manifest reader harness.

#![expect(
    clippy::expect_used,
    reason = "The project fixture states every expected cell together and fails immediately if its declared manifests cannot be read"
)]

use crate::domain::links::types::Linked;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::factory::{bun, npm, pnpm, tsconfig};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages::{self, Project as _};
use std::sync::Arc;

/// A workspace of npm, pnpm and TypeScript manifests, by path and text, that the project contract reads whole.
pub(crate) const WORKSPACE: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":{"packages":["packages/*","!packages/skip"]},"dependencies":{"core":"workspace:*","outside":"=4.6.0","alias":"npm:core@1"},"devDependencies":{"core":"*"},"peerDependencies":{"core":"*"},"optionalDependencies":{"util":"*"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    ("packages/core/index.ts", "export const value = 1;"),
    ("packages/util/package.json", r#"{"name":"util"}"#),
    (
        "packages/util/tsconfig.json",
        "{ // config\n\"references\": [{\"path\":\"../core\"}], }",
    ),
    ("packages/core/tsconfig.json", "{}"),
    ("packages/skip/package.json", r#"{"name":"skip"}"#),
    ("index.ts", "export {};"),
];

/// A repository whose files import one another every way the links read: relative specifiers, a JavaScript ending written for a
/// TypeScript file, a declaration file, a directory's `index`, an alias of `paths` and a path from `baseUrl`, a package of the repository
/// and one outside it, and two imports that name a file of the repository no file is.
pub(crate) const IMPORTS: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"app","workspaces":["packages/*"],"dependencies":{"core":"*"}}"#,
    ),
    (
        "tsconfig.json",
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@lib/*":["lib/*"],"@gone/*":["missing/*"]}}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    ("packages/core/index.ts", "export const value = 1;\n"),
    (
        "src/app.ts",
        "import { a } from './a';\nimport { b } from '../lib/b.js';\nimport { c } from '@lib/c';\nimport type { D } from './types';\nimport { value } from 'core';\nimport { readFile } from 'node:fs';\nexport * from './dir';\nimport { x } from './missing';\nimport { y } from '@gone/y';\nimport { u } from 'lib/u';\n",
    ),
    ("src/a.ts", "export const a = 1;\n"),
    ("lib/b.ts", ""),
    ("lib/c.tsx", ""),
    ("lib/u.ts", ""),
    ("src/types.d.ts", ""),
    ("src/dir/index.ts", "import { app } from '../app';\n"),
];

/// The manifest reading adapters the language reads the manifests of a run with, over the run's manifests, for the tests of the contracts over
/// the real parsers.
pub(crate) fn adapters(candidates: &[languages::FilePath]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![
        Arc::new(npm(candidates.to_vec())),
        Arc::new(pnpm(candidates.to_vec())),
        Arc::new(bun()),
        Arc::new(tsconfig()),
    ]
}

/// The project contract over the real adapters and notes, for the tests of the contract.
pub(crate) fn project() -> ProjectContract {
    ProjectContract::new(adapters, Arc::new(postcard()))
}

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
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Vec::new(),
        repository: languages::Repository::Disk,
        root: languages::RootPath(Box::default()),
        manifests: files
            .iter()
            .filter(|(path, _)| {
                path.ends_with("package.json")
                    || path.ends_with("tsconfig.json")
                    || path.ends_with("pnpm-workspace.yaml")
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

/// A sink for the rows a manifest relation writes, which keeps them and counts the partial answers.
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

/// What the item writes for the repository whose files are `files`, the run asking for the links between them.
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
    let resolved = project().resolve(&request).expect("project resolved");
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
