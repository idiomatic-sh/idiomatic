//! The rows the manifest relations must give for the project in `manifests/patched/`: `app` depends on `y` by version, which the
//! workspace's `[patch]` puts in `y`'s directory.

use ryo::languages::contract::{
    AnalyzedSourceExpectedRow, FilePackageExpectedRow, ManifestExpected,
    PackageDependencyExpectedRow, WorkspaceMemberExpectedRow, file, name,
};

/// The manifest rows the project must give, as the manifests contract compares them.
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("patched")
        .rows(&[
            AnalyzedSourceExpectedRow {
                path: file("app/src/lib.rs"),
            },
            AnalyzedSourceExpectedRow {
                path: file("y/src/lib.rs"),
            },
        ])
        .rows(&[PackageDependencyExpectedRow {
            source: file("app/Cargo.toml"),
            target: file("y/Cargo.toml"),
            binding: name("y"),
            package: name("y"),
            kind: name("normal"),
            condition: name(""),
            optional: false,
            origin: name("registry"),
            requirement: name("0.1"),
            exact: false,
            branch: name(""),
            tag: name(""),
            revision: name(""),
            interpreter: name(""),
            catalog: name(""),
            replacement: name("y"),
        }])
        .rows(&[
            FilePackageExpectedRow {
                path: file("Cargo.lock"),
                package: file("Cargo.toml"),
            },
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
                path: file("y/Cargo.toml"),
                package: file("y/Cargo.toml"),
            },
            FilePackageExpectedRow {
                path: file("y/src/lib.rs"),
                package: file("y/Cargo.toml"),
            },
        ])
        .rows(&[
            WorkspaceMemberExpectedRow {
                workspace: file("Cargo.toml"),
                package: file("app/Cargo.toml"),
            },
            WorkspaceMemberExpectedRow {
                workspace: file("Cargo.toml"),
                package: file("y/Cargo.toml"),
            },
        ])
}
