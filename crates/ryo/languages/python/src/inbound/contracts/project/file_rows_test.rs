//! The premise of the per-file project read: the rows of `project-namespace-membership` of a file, from a request that carries only that
//! file's notes (`file_entries`, `file_rows`), are the rows of that file from the request that carries every file's notes (`resolve`).
//! The engine reads the relation in the visit of each file on this premise; a file whose rows differ would make the relation Needed again.

use std::sync::Arc;

use super::contract::ProjectContract;
use crate::fixtures::projects::{IMPORTS, notes_of};
use crate::outbound::notes::factory::postcard;
use ryo::languages::{self, Project as _};

/// The observation the engine makes of each candidate entry: a file when the repository lists it, a directory when it lists a file
/// below it, nothing otherwise.
fn observed(candidates: &languages::ProjectEntries, files: &[languages::FilePath]) -> languages::ObservedEntries {
    languages::ObservedEntries(
        candidates
            .0
            .iter()
            .map(|path| languages::EntryObservation {
                path: path.clone(),
                state: if files.contains(path) {
                    languages::ObservedEntryState::File
                } else if files.iter().any(|file| file.0.starts_with(&format!("{}/", path.0))) {
                    languages::ObservedEntryState::Directory
                } else {
                    languages::ObservedEntryState::Missing
                },
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn contract() -> ProjectContract {
    ProjectContract::new(
        |_| Vec::new(),
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    )
}

/// The rows of one file as text: the root, the path, the membership and the kind it reads, sorted, with the causes it says it reads in part.
fn shown(symbols: &languages::Symbols, words: &[u64], per_row: usize) -> Vec<String> {
    let text = |word: u64| languages::text_of(symbols, languages::ByteSlice(&[]), languages::Symbol(u32::try_from(word).unwrap_or(u32::MAX))).map_or_else(String::new, |text| text.0.to_owned());
    let mut rows: Vec<String> = words.chunks(per_row).map(|row| row[1..].iter().map(|word| text(*word)).collect::<Vec<_>>().join(" ")).collect();
    rows.sort();
    rows
}

#[test]
fn a_file_read_alone_has_the_rows_it_has_in_the_whole_request() {
    let contract = contract();
    let paths: Vec<&str> = IMPORTS.iter().map(|(path, _)| *path).collect();
    let mut sorted: Vec<(&str, &str)> = IMPORTS.to_vec();
    sorted.sort_unstable();
    let files: Vec<languages::FilePath> = sorted.iter().map(|(path, _)| languages::FilePath((*path).into())).collect();
    let notes: Vec<languages::Bytes> = sorted.iter().map(|(path, text)| notes_of(path, text)).collect();
    assert!(paths.len() > 5);
    for search in [vec![languages::FilePath("".into())], vec![languages::FilePath("".into()), languages::FilePath("packages/lib_one/src".into())]] {
        let skeleton = languages::ProjectRequest {
            search_scope: languages::ProjectSearchScope::Declared(search),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
            repository: languages::Repository::Given,
            root: languages::RootPath("".into()),
            manifests: Vec::new(),
            files: files.clone(),
            notes: Vec::new(),
            wanted: vec![languages::Relation::ProjectNamespaceMembership],
        };
        // The whole request: every file's notes, the entries the contract asks for, observed.
        let mut whole = languages::ProjectRequest { notes: notes.clone(), ..skeleton.clone() };
        let mut candidates = languages::ProjectEntries::default();
        contract.project_entries(&whole, &mut candidates);
        whole.observed_entries = observed(&candidates, &files);
        let resolved = contract.resolve(&whole).expect("the project is read");
        let all: Vec<_> = resolved.derived.namespace_memberships.rows.clone();
        assert!(!all.is_empty(), "the fixture imports something under the declared roots");
        for (at, note) in notes.iter().enumerate() {
            let mut entries = languages::ProjectEntries::default();
            contract.file_entries(&skeleton, at, note, &mut entries);
            let own = observed(&entries, &files);
            let mut symbols = languages::Symbols::default();
            let mut out = languages::contract::Collect::default();
            let alone = contract.file_rows(&skeleton, at, note, &own, &mut symbols, &mut out).expect("the file is read alone");
            assert_eq!(alone.observed_entries, own.0.to_vec(), "the receipt is the observation it was given");
            let mut wanted: Vec<String> = all
                .iter()
                .filter(|row| usize::try_from(row.file.0).ok() == Some(at))
                .map(|row| format!("{} {} {} {}", row.root.0, row.path.0, match row.state {
                    crate::domain::project_namespace::types::MembershipState::Present => "present",
                    crate::domain::project_namespace::types::MembershipState::Absent => "absent",
                    crate::domain::project_namespace::types::MembershipState::Unknown => "unknown",
                }, match row.kind {
                    crate::domain::project_namespace::types::EntryKind::Module => "module",
                    crate::domain::project_namespace::types::EntryKind::Package => "package",
                    crate::domain::project_namespace::types::EntryKind::Namespace => "namespace",
                    crate::domain::project_namespace::types::EntryKind::Absent => "absent",
                    crate::domain::project_namespace::types::EntryKind::Unknown => "unknown",
                }))
                .collect();
            wanted.sort();
            assert_eq!(shown(&symbols, &out.words, 5), wanted, "file {} read alone", files[at].0);
        }
    }
}
