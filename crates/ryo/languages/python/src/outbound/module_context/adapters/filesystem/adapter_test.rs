//! The filesystem adapter runs the `module_context` port's contract: the shared cases, then the cases specific to it.

use super::adapter::ModuleContextFilesystemAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::module_context::fixtures::contract::{check, request};
use crate::outbound::module_context::interface::ModuleContextPort as _;
use ryo::languages::{ProjectError, Repository};

/// Which Given repository a row looks up.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Packages whose names hold dots.
    DottedNames,
    /// A package below a namespace directory that is not part of the package.
    NamespacePrefix,
}

/// The rows the lookup answers.
type Value = usize;
type Failure = ProjectError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Given, Value, Failure>] = &[
    Case {
        name: CaseName("given_raw_names_preserve_dotted_packages_modules_and_stub_extension"),
        word: Word::Succeeds,
        input: || Given::DottedNames,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("given_namespace_prefix_does_not_become_the_package_root_name"),
        word: Word::Succeeds,
        input: || Given::NamespacePrefix,
        expect: Expect::Value(4),
    },
];

/// The rows the adapter answers for a Given repository of these files.
fn rows(files: &[&str]) -> Result<Vec<crate::domain::module_context::types::ModuleIdentity>, ProjectError> {
    let contexts = ModuleContextFilesystemAdapter.resolve(&request(Repository::Given, files))?;
    assert!(!contexts.partial);
    Ok(contexts.rows)
}

fn subject(given: Given) -> Result<Value, Failure> {
    match given {
        Given::DottedNames => {
            let rows = rows(&[
                "pkg.with.dot/__init__.py",
                "pkg.with.dot/mod.name.pyi",
                "pkg.with.dot/sub.package/__init__.py",
                "pkg.with.dot/sub.package/worker.py",
                "standalone.dot/script.py",
            ])?;
            let observed = rows
                .iter()
                .map(|row| {
                    format!(
                        "{} {} {} {} {}",
                        row.file.0, row.path, row.module_name, row.package_name, row.package_root_name
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                observed,
                [
                    "0 pkg.with.dot.__init__ __init__ pkg.with.dot pkg.with.dot",
                    "1 pkg.with.dot.mod.name mod.name pkg.with.dot pkg.with.dot",
                    "2 pkg.with.dot.sub.package.__init__ __init__ sub.package pkg.with.dot",
                    "3 pkg.with.dot.sub.package.worker worker sub.package pkg.with.dot",
                ]
            );
            assert!(rows.iter().all(|row| !row.module_name.contains('/')
                && !row.package_name.contains('/')
                && !row.package_root_name.contains('/')));
            Ok(rows.len())
        }
        Given::NamespacePrefix => {
            let rows = rows(&[
                "namespace/pkg.root/__init__.py",
                "namespace/pkg.root/child.name/__init__.py",
                "namespace/pkg.root/child.name/__main__.py",
                "namespace/pkg.root/worker.pyi",
            ])?;
            for row in &rows {
                assert_eq!(row.package_root_name, "pkg.root");
                assert!(row.path.starts_with("pkg.root."));
                assert!(!row.path.contains("namespace"));
            }
            assert_eq!(rows[2].package_name, "child.name");
            assert_eq!(rows[2].module_name, "__main__");
            Ok(rows.len())
        }
    }
}

#[test]
fn succeeds() {
    check(&ModuleContextFilesystemAdapter);
    run(CASES, Word::Succeeds, subject);
}
