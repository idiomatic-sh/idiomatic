//! The rows the manifest relations must give for the project in `manifests/workspace/`.

use ryo::languages::contract::{
    AnalyzedSourceExpectedRow, FilePackageExpectedRow, ManifestDiagnosticExpectedRow,
    ManifestExpected, PackageDependencyExpectedRow, SourceDependencyExpectedRow,
    WorkspaceMemberExpectedRow, file, name, span,
};

#[expect(
    clippy::too_many_lines,
    reason = "the fixture states every expected row of every manifest relation together, one row per line"
)]
/// The rows every manifest relation must give for the project in `manifests/workspace/`.
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("workspace")
        .rows(&[ManifestDiagnosticExpectedRow {
            file: file("skip/go.mod"),
            span: span((1, 1), (1, 1)),
            location_kind: name("file-origin"),
            phase: name("parse"),
            schema: name("go-module"),
            code: name("invalid-go-mod"),
            message: name("the go.mod has no module directive"),
        }])
        .rows(&[
            AnalyzedSourceExpectedRow {
                path: file("app/internal/greet/greet.go"),
            },
            AnalyzedSourceExpectedRow {
                path: file("app/main.go"),
            },
            AnalyzedSourceExpectedRow {
                path: file("libs/core/core.go"),
            },
            AnalyzedSourceExpectedRow {
                path: file("libs/util/util.go"),
            },
        ])
        .rows(&[SourceDependencyExpectedRow {
            source: file("app/main.go"),
            target: file("app/internal/greet/greet.go"),
            span: span((6, 2), (6, 34)),
            written: name("\"example.com/app/internal/greet\""),
        }])
        .rows(&[
            PackageDependencyExpectedRow {
                source: file("app/go.mod"),
                target: file("libs/core/go.mod"),
                binding: name("example.com/core"),
                package: name("example.com/core"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("registry"),
                requirement: name("v1.2.0"),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/go.mod"),
                target: file("libs/util/go.mod"),
                binding: name("example.com/util"),
                package: name("example.com/util"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("registry"),
                requirement: name("v0.1.0"),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/go.mod"),
                target: file(""),
                binding: name("github.com/other/lib"),
                package: name("github.com/other/lib"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("registry"),
                requirement: name("v1.0.0"),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name("github.com/fork/lib v1.0.1"),
            },
            PackageDependencyExpectedRow {
                source: file("libs/core/go.mod"),
                target: file("libs/util/go.mod"),
                binding: name("example.com/util"),
                package: name("example.com/util"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("path"),
                requirement: name("v0.1.0"),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name("../util"),
            },
        ])
        .rows(&[
            WorkspaceMemberExpectedRow {
                workspace: file("go.work"),
                package: file("app/go.mod"),
            },
            WorkspaceMemberExpectedRow {
                workspace: file("go.work"),
                package: file("libs/core/go.mod"),
            },
            WorkspaceMemberExpectedRow {
                workspace: file("go.work"),
                package: file("libs/util/go.mod"),
            },
        ])
        .rows(&[
            FilePackageExpectedRow {
                path: file("app/go.mod"),
                package: file("app/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("app/internal/greet/greet.go"),
                package: file("app/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("app/main.go"),
                package: file("app/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("libs/core/core.go"),
                package: file("libs/core/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("libs/core/go.mod"),
                package: file("libs/core/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("libs/util/go.mod"),
                package: file("libs/util/go.mod"),
            },
            FilePackageExpectedRow {
                path: file("libs/util/util.go"),
                package: file("libs/util/go.mod"),
            },
        ])
}
