//! What the `project` contract promises: it names `pyproject.toml`, reads the packages the manifests declare through the port,
//! refuses a manifest the tool cannot read without stopping the run, and links the files only when a relation of the links is asked
//! for.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::service::reads;
use crate::fixtures::projects::notes_of;
use crate::outbound::manifest_reading::fixtures::fake::ManifestReadingFake;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::module_context::interface::ModuleContextPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages::{self, Project as _};

/// Whether the run asks for a relation of the links between the files.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// It asks for `source_dependency`.
    Links,
    /// It asks for none.
    Nothing,
    /// It asks for package dependency projection.
    Packages,
}

/// The files a repository holds, each with its path and its text, and what the run asks for.
type Input = (&'static [(&'static str, &'static str)], Asked);
/// How many manifests it names, how many packages the manifests declared, how many it refused and how many files it examined.
type Value = (usize, usize, usize, usize);
type Failure = languages::ProjectError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("names_one_manifest_and_a_repository_without_one_gives_no_package"),
        word: Word::Succeeds,
        input: || (&[], Asked::Nothing),
        expect: Expect::Value((1, 0, 0, 0)),
    },
    Case {
        name: CaseName("a_manifest_that_names_a_package_declares_one"),
        word: Word::Succeeds,
        input: || {
            (
                &[("pyproject.toml", "[project]\nname = \"demo\"\n")],
                Asked::Packages,
            )
        },
        expect: Expect::Value((1, 1, 0, 0)),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_is_refused_and_the_run_goes_on"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a/pyproject.toml", "[project\nname"),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Packages,
            )
        },
        expect: Expect::Value((1, 1, 1, 0)),
    },
    Case {
        name: CaseName("a_file_that_is_not_a_manifest_is_not_read"),
        word: Word::Succeeds,
        input: || {
            (
                &[("README.md", "[project]\nname = \"demo\"\n")],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((1, 0, 0, 0)),
    },
    Case {
        name: CaseName("the_python_files_are_examined_when_a_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a.py", ""),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Links,
            )
        },
        expect: Expect::Value((1, 1, 0, 1)),
    },
    Case {
        name: CaseName("the_files_are_not_linked_when_no_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a.py", ""),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Packages,
            )
        },
        expect: Expect::Value((1, 1, 0, 0)),
    },
];

/// The adapters of a run: the fake alone.
fn faked(_available: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![Arc::new(ManifestReadingFake)]
}

fn subject((files, asked): Input) -> Result<Value, Failure> {
    let contract = ProjectContract::new(
        faked,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    languages::contract::project(&contract);
    let resolved = contract.resolve(&languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Disk,
        root: languages::RootPath("".into()),
        manifests: files
            .iter()
            .filter(|(path, _)| {
                reads(languages::SourcePath(path)) == languages::Recognition::Declines
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
        wanted: match asked {
            Asked::Links => vec![languages::Relation::SourceDependency],
            Asked::Nothing => Vec::new(),
            Asked::Packages => vec![languages::Relation::PackageDependency],
        },
    })?;
    Ok((
        contract.manifests().len(),
        resolved.derived.packages.packages.len(),
        resolved.refused.len(),
        resolved.derived.analyzed.len(),
    ))
}

/// The readers of a run whose only manifest tool is pep621.
fn pep(_: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![Arc::new(
        crate::outbound::manifest_reading::factory::pep621(),
    )]
}

#[derive(Default)]
struct ReaderOutcome {
    rows: usize,
    partial: Vec<String>,
}
impl<Row> languages::Rows<Row> for ReaderOutcome {
    fn write(&mut self, _row: Row) {
        self.rows += 1;
    }
    fn partial(&mut self, cause: languages::ReportText<'_>, _missing: languages::Missing) {
        self.partial.push(cause.0.to_owned());
    }
    fn partial_at(
        &mut self,
        _file: languages::FileId,
        cause: languages::ReportText<'_>,
        missing: languages::Missing,
    ) {
        <Self as languages::Rows<Row>>::partial(self, cause, missing);
    }
}

/// A module context that counts the lookups it answers and checks the request it is given.
struct ContextProbe {
    inner: Arc<dyn ModuleContextPort>,
    seen: Arc<AtomicUsize>,
}
impl ModuleContextPort for ContextProbe {
    fn resolve(
        &self,
        request: &languages::ProjectRequest,
    ) -> Result<crate::domain::module_context::types::ModuleContexts, languages::ProjectError> {
        assert_eq!(request.manifests.len(), 1);
        assert_eq!(
            request.manifests[0].text.0.as_ref(),
            "not TOML: projection must not be attempted"
        );
        self.seen.fetch_add(1, Ordering::Relaxed);
        self.inner.resolve(request)
    }
}
/// The readers of a run that must build none for a module identity request.
fn no_projection(manifests: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    assert!(
        manifests.is_empty(),
        "only initial declared-name discovery may build these readers"
    );
    vec![Arc::new(ManifestReadingFake)]
}

fn primary_metadata_rejection_keeps_real_dependency_projection() {
    let contract = ProjectContract::new(
        pep,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    let text = "[project]\nname = 'example'\nversion = '1.0'\nauthors = [{email = 1}]\ndependencies = ['requests>=2']\n";
    let request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("".into()),
        files: vec![languages::FilePath("pyproject.toml".into())],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText(text.into()),
        }],
        notes: vec![],
        wanted: vec![
            languages::Relation::ManifestDiagnostic,
            languages::Relation::PackageDependency,
        ],
    };
    let resolved = contract
        .resolve(&request)
        .expect("pure manifest processing");
    assert!(resolved.refused.is_empty());
    assert_eq!(resolved.derived.packages.packages.len(), 1);
    assert_eq!(
        resolved.derived.packages.packages[0].declared.requirements[0]
            .written
            .0
            .as_ref(),
        "requests>=2"
    );
    assert_eq!(
        resolved.derived.manifest_validation.files[0].file,
        languages::FileId(0)
    );
    assert_eq!(
        resolved.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    assert!(resolved.derived.manifest_validation.incomplete.is_empty());
}

fn metadata_rejection_and_dependency_refusal_have_independent_applicability() {
    let contract = ProjectContract::new(
        pep,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    let mut request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("".into()),
        files: vec![languages::FilePath("pyproject.toml".into())],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText(
                "[project]\nname = 'example'\nversion = '1.0'\ndependencies = [1]\n".into(),
            ),
        }],
        notes: vec![],
        wanted: vec![
            languages::Relation::ManifestDiagnostic,
            languages::Relation::SyntaxError,
        ],
    };
    let metadata = contract
        .resolve(&request)
        .expect("metadata diagnostic is a completed observation");
    assert!(metadata.refused.is_empty());
    assert!(metadata.derived.packages.packages.is_empty());
    assert!(
        metadata.derived.packages.refused.is_empty(),
        "undemanded dependencies are not read"
    );
    assert_eq!(
        metadata.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    request.wanted.push(languages::Relation::PackageDependency);
    let both = contract
        .resolve(&request)
        .expect("dependency rejection remains partial");
    assert_eq!(both.refused.len(), 1);
    assert!(both.derived.packages.packages.is_empty());
    assert_eq!(
        both.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    use languages::ManifestItem as _;
    let mut symbols = languages::Symbols::default();
    let input = languages::ManifestInput {
        request: &request,
        derived: &both.derived,
    };
    let mut dependencies = ReaderOutcome::default();
    (crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem::READER.read)(&input, &mut symbols, &mut dependencies);
    assert_eq!(dependencies.rows, 0);
    assert!(
        !dependencies.partial.is_empty(),
        "rejected dependency output stays unknown"
    );
    let mut diagnostics = ReaderOutcome::default();
    (crate::domain::manifests::collections::manifests::manifest_diagnostic::item::ManifestDiagnosticItem::READER.read)(&input, &mut symbols, &mut diagnostics);
    assert_eq!(diagnostics.rows, 1);
    assert!(
        diagnostics.partial.is_empty(),
        "actual schema diagnostic is completely observed"
    );
    request.wanted = vec![languages::Relation::SyntaxError];
    let unrelated = contract
        .resolve(&request)
        .expect("unrequested projection cannot create a read failure");
    assert!(unrelated.refused.is_empty());
}

fn module_identity_only_preserves_inputs_without_projecting_or_validating() {
    let seen = Arc::new(AtomicUsize::new(0));
    let contract = ProjectContract::new(
        no_projection,
        Arc::new(postcard()),
        Arc::new(ContextProbe {
            inner: Arc::new(crate::outbound::module_context::factory::filesystem()),
            seen: Arc::clone(&seen),
        }),
    );
    let request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("project".into()),
        files: vec![languages::FilePath("module.py".into())],
        notes: vec![],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText("not TOML: projection must not be attempted".into()),
        }],
        wanted: vec![languages::Relation::ModuleIdentity],
    };
    let resolved = contract
        .resolve(&request)
        .expect("module context owns its original complete request");
    assert_eq!(seen.load(Ordering::Relaxed), 1);
    assert!(resolved.refused.is_empty());
    assert!(resolved.derived.packages.packages.is_empty());
    assert!(resolved.derived.manifest_validation.files.is_empty());
    assert!(resolved.derived.manifest_validation.incomplete.is_empty());
}

fn positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots() {
    use crate::domain::project_namespace::{
        constants::UNKNOWN_CONTEXT,
        service::memberships,
        types::{EntryKind, ImportedRoot, MembershipState},
    };
    use languages::ManifestItem as _;
    for (module, directory, incomplete, expected) in [
        (
            false,
            true,
            true,
            vec![EntryKind::Namespace, EntryKind::Unknown],
        ),
        (
            true,
            false,
            true,
            vec![EntryKind::Module, EntryKind::Unknown],
        ),
        (
            true,
            true,
            false,
            vec![EntryKind::Module, EntryKind::Namespace],
        ),
    ] {
        let roots = [ImportedRoot {
            file: languages::FileId(0),
            root: "acme".into(),
            path: "acme".into(),
        }];
        let search = [
            languages::FilePath("src".into()),
            languages::FilePath("lib".into()),
        ];
        let namespace_memberships =
            memberships(Some(&search), &roots, vec![], |candidate, file| {
                if candidate.0.starts_with("lib/") && incomplete {
                    None
                } else {
                    Some(match candidate.0.as_ref() {
                        "src/acme.py" => module && file,
                        "src/acme" => directory && !file,
                        _ => false,
                    })
                }
            });
        let derived = crate::domain::links::types::Linked {
            namespace_memberships,
            ..Default::default()
        };
        let request = languages::ProjectRequest {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
            repository: languages::Repository::Given,
            root: languages::RootPath("".into()),
            files: vec![languages::FilePath("source.py".into())],
            manifests: vec![],
            notes: vec![notes_of("source.py", "import acme\n")],
            wanted: vec![languages::Relation::ProjectNamespaceMembership],
        };
        assert_eq!(
            derived
                .namespace_memberships
                .rows
                .iter()
                .map(|row| row.kind)
                .collect::<Vec<_>>(),
            expected
        );
        let positives = derived
            .namespace_memberships
            .rows
            .iter()
            .filter(|row| row.kind != EntryKind::Unknown)
            .collect::<Vec<_>>();
        assert!(!positives.is_empty());
        assert!(
            positives
                .iter()
                .all(|row| row.state == MembershipState::Present)
        );
        let mut output = ReaderOutcome::default();
        let mut symbols = languages::Symbols::default();
        let input = languages::ManifestInput {
            request: &request,
            derived: &derived,
        };
        (crate::domain::manifests::collections::manifests::project_namespace_membership::item::ProjectNamespaceMembershipItem::READER.read)(&input, &mut symbols, &mut output);
        assert_eq!(output.rows, expected.len());
        assert_eq!(
            output.partial,
            if incomplete {
                vec![UNKNOWN_CONTEXT.to_owned()]
            } else {
                vec![]
            }
        );
    }
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `primary_metadata_rejection_keeps_real_dependency_projection`.
    PrimaryMetadataRejectionKeepsRealDependencyProjection,
    /// The case `metadata_rejection_and_dependency_refusal_have_independent_applicability`.
    MetadataRejectionAndDependencyRefusalHaveIndependentApplicability,
    /// The case `module_identity_only_preserves_inputs_without_projecting_or_validating`.
    ModuleIdentityOnlyPreservesInputsWithoutProjectingOrValidating,
    /// The case `positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots`.
    PositiveNamespaceFactsRetainActualReaderPartialForUnobservedRoots,
}

/// The rows of the cases whose bodies assert for themselves.
const BODIES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("primary_metadata_rejection_keeps_real_dependency_projection"),
        word: Word::Succeeds,
        input: || Body::PrimaryMetadataRejectionKeepsRealDependencyProjection,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("metadata_rejection_and_dependency_refusal_have_independent_applicability"),
        word: Word::Succeeds,
        input: || Body::MetadataRejectionAndDependencyRefusalHaveIndependentApplicability,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("module_identity_only_preserves_inputs_without_projecting_or_validating"),
        word: Word::Succeeds,
        input: || Body::ModuleIdentityOnlyPreservesInputsWithoutProjectingOrValidating,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots"),
        word: Word::Succeeds,
        input: || Body::PositiveNamespaceFactsRetainActualReaderPartialForUnobservedRoots,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::PrimaryMetadataRejectionKeepsRealDependencyProjection => {
            primary_metadata_rejection_keeps_real_dependency_projection()
        }
        Body::MetadataRejectionAndDependencyRefusalHaveIndependentApplicability => {
            metadata_rejection_and_dependency_refusal_have_independent_applicability()
        }
        Body::ModuleIdentityOnlyPreservesInputsWithoutProjectingOrValidating => {
            module_identity_only_preserves_inputs_without_projecting_or_validating()
        }
        Body::PositiveNamespaceFactsRetainActualReaderPartialForUnobservedRoots => {
            positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(BODIES, Word::Succeeds, body);
    run(CASES, Word::Succeeds, subject);
}
