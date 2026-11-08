//! The rows the manifest relations must give for the project in `manifests/packages/`.

use ryo::languages::contract::{
    AnalyzedSourceExpectedRow, FilePackageExpectedRow, ManifestExpected,
    PackageDependencyExpectedRow, SourceDependencyExpectedRow, WorkspaceMemberExpectedRow, file,
    name, span,
};

/// The manifest rows the project must give, as the manifests contract compares them.
#[expect(
    clippy::too_many_lines,
    reason = "the fixture states every expected row of every manifest relation together, one row per line"
)]
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("packages")
        .rows(&[
            AnalyzedSourceExpectedRow {
                path: file("app/src/lib.rs"),
            },
            AnalyzedSourceExpectedRow {
                path: file("app/src/util.rs"),
            },
            AnalyzedSourceExpectedRow {
                path: file("support/src/lib.rs"),
            },
            AnalyzedSourceExpectedRow {
                path: file("tool/src/lib.rs"),
            },
        ])
        .rows(&[
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file(""),
                binding: name("followed"),
                package: name("followed"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("git"),
                requirement: name(""),
                exact: false,
                branch: name("main"),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file(""),
                binding: name("shared"),
                package: name("shared"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("git"),
                requirement: name(""),
                exact: false,
                branch: name(""),
                tag: name("v1.0.0"),
                revision: name(""),
                interpreter: name(""),
                catalog: name("workspace = true"),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file("support/Cargo.toml"),
                binding: name("helper"),
                package: name("support"),
                kind: name("normal"),
                condition: name(""),
                optional: true,
                origin: name("path"),
                requirement: name("0.1"),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file(""),
                binding: name("remote"),
                package: name("remote"),
                kind: name("normal"),
                condition: name(""),
                optional: false,
                origin: name("git"),
                requirement: name(""),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name("0123456"),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file("tool/Cargo.toml"),
                binding: name("tool"),
                package: name("tool"),
                kind: name("build"),
                condition: name("cfg(unix)"),
                optional: false,
                origin: name("path"),
                requirement: name(""),
                exact: false,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
            PackageDependencyExpectedRow {
                source: file("app/Cargo.toml"),
                target: file("tool/Cargo.toml"),
                binding: name("tool"),
                package: name("tool"),
                kind: name("dev"),
                condition: name(""),
                optional: false,
                origin: name("path"),
                requirement: name("=0.1.0"),
                exact: true,
                branch: name(""),
                tag: name(""),
                revision: name(""),
                interpreter: name(""),
                catalog: name(""),
                replacement: name(""),
            },
        ])
        .rows(&[SourceDependencyExpectedRow {
            source: file("app/src/lib.rs"),
            target: file("app/src/util.rs"),
            span: span((4, 5), (4, 17)),
            written: name("util::double"),
        },
        SourceDependencyExpectedRow {
            source: file("app/src/lib.rs"),
            target: file("support/src/lib.rs"),
            span: span((7, 17), (7, 30)),
            written: name("helper::ready"),
        }])
        .rows(&[
            FilePackageExpectedRow {
                path: file("Cargo.toml"),
                package: file("Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("app/Cargo.toml"),
                package: file("app/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("app/src/lib.rs"),
                package: file("app/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("app/src/util.rs"),
                package: file("app/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("support/Cargo.toml"),
                package: file("support/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("support/src/lib.rs"),
                package: file("support/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("tool/Cargo.toml"),
                package: file("tool/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("tool/src/lib.rs"),
                package: file("tool/Cargo.toml"),
            },
        ])
        .rows(&[
            WorkspaceMemberExpectedRow {
                workspace: file("Cargo.toml"),
                package: file("app/Cargo.toml"),
            },
            WorkspaceMemberExpectedRow {
                workspace: file("Cargo.toml"),
                package: file("support/Cargo.toml"),
            },
            WorkspaceMemberExpectedRow {
                workspace: file("Cargo.toml"),
                package: file("tool/Cargo.toml"),
            },
        ])
}
