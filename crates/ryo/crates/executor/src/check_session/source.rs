//! The source adapter keeps the linked provider's native tree and symbol storage.
//! Both walks feed the same numbered node table, catalog rows and executor.
use super::{Result, l};
use ryo_language::domain::files::types::ProjectEntries;
use std::sync::Arc;

pub use ryo_language::inbound::use_cases::parse::named::FileRelation;
pub type Columns = Vec<ryo_relations::domain::relations::types::FactColumn>;

pub trait Source: Clone + Sync {
    fn source_words(&self) -> Option<l::SourceWords> { None }
    fn uses_project(&self) -> bool { false }
    fn declarations(&self) -> &'static [FileRelation] { &[] }

    /// The numbered visit that lends this language's files: every language has one, there is no default.
    fn scoped(&self) -> &dyn l::NumberedVisit;
    fn numbered_projections(&self) -> &'static [l::Relation] {
        self.scoped().projections()
    }


    fn name(&self) -> &str;
    fn identity(&self) -> (&str, &str);
    fn reads(&self, path: &str) -> bool;
    fn manifests(&self) -> Vec<&'static str>;
    fn words(&self) -> Vec<l::UnknownName>;
    fn kinds(&self) -> Vec<&'static str>;
    fn fields(&self) -> l::FieldNames { l::FieldNames(Vec::new()) }
    fn expands(&self, rel: l::Relation) -> bool;
    fn available(&self, rel: l::Relation) -> bool;
    /// Whether the language never answers `rel`, for whatever cause: static, so known before any step runs.
    fn absent(&self, rel: l::Relation) -> bool;
    fn entries(&self, req: &l::ProjectRequest, out: &mut ProjectEntries) -> Result<()>;
    /// The candidate entries the wanted relations need for the file `file` of `req.files` alone, from its own notes: the per-file half of
    /// `entries`. A language that does not answer a relation from one file's notes asks for nothing.
    fn file_entries(&self, _req: &l::ProjectRequest, _file: usize, _notes: &l::Bytes, _out: &mut ProjectEntries) -> Result<()> { Ok(()) }
    /// The rows of the wanted relations about the file `file` of `req.files`, from its own notes and the observation of the entries
    /// `file_entries` asked for; none when the language reads these relations only from every file's notes together.
    fn file_rows(
        &self,
        _req: &l::ProjectRequest,
        _file: usize,
        _notes: &l::Bytes,
        _observed: &l::ObservedEntries,
        _symbols: &mut l::Symbols,
        _rows: &mut l::contract::Collect,
    ) -> Result<Option<l::Resolved>> { Ok(None) }
    fn project(
        &self,
        req: &l::ProjectRequest,
        symbols: &mut l::Symbols,
        rows: &mut l::contract::Collect,
    ) -> Result<l::Resolved>;
}

/// The languages that read `path`, by index: the one a request names, else those whose recognizer reads it. The scheduler reads a
/// file by exactly one of them (none: unclaimed; several: refused) and the session gives the project environment by the same
/// list, so which language a file belongs to is decided in this one place.
pub fn routed<'a, L: Source>(languages: &'a [L], reader: Option<&'a str>, path: &'a str) -> impl Iterator<Item = (usize, &'a L)> {
    languages.iter().enumerate().filter(move |(_, language)| reader.map_or_else(|| language.reads(path), |name| name == language.name()))
}

/// Only a file of a language that uses the project takes the project environment; every other file keeps the
/// shared overlay. A request that names a reader gives the environment to that reader's files, as before, which also covers a
/// file read only because the request names it. It is asked of the languages `routed` names, the list the scheduler reads by.
pub fn takes_environment<L: Source>(languages: &[L], reader: Option<&str>, path: &str) -> bool {
    routed(languages, reader, path).any(|(_, language)| language.uses_project())
}

#[cfg(test)]
mod fake {
    use super::{Result, Source, l, ProjectEntries};
    /// The numbered visit of the language that reads nothing: it lends no file.
    static NOTHING: ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl = ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;

    /// A language that reads files by extension and either uses the project or does not.
    #[derive(Clone)]
    pub struct Fake { pub name: &'static str, pub extension: &'static str, pub project: bool }

    impl Source for Fake {
        fn uses_project(&self) -> bool { self.project }
        fn scoped(&self) -> &dyn l::NumberedVisit { &NOTHING }
        fn name(&self) -> &str { self.name }
        fn identity(&self) -> (&str, &str) { (self.name, "") }
        fn reads(&self, path: &str) -> bool { path.ends_with(self.extension) }
        fn manifests(&self) -> Vec<&'static str> { Vec::new() }
        fn words(&self) -> Vec<l::UnknownName> { Vec::new() }
        fn kinds(&self) -> Vec<&'static str> { Vec::new() }
        fn expands(&self, _: l::Relation) -> bool { false }
        fn available(&self, _: l::Relation) -> bool { true }
        fn absent(&self, _: l::Relation) -> bool { false }
        fn entries(&self, _: &l::ProjectRequest, _: &mut ProjectEntries) -> Result<()> { Ok(()) }
        fn project(&self, _: &l::ProjectRequest, _: &mut l::Symbols, _: &mut l::contract::Collect) -> Result<l::Resolved> { unreachable!() }
    }

}
#[cfg(test)]
pub use fake::Fake;

#[cfg(test)]
mod environment_scope_tests {
    use super::{Fake, takes_environment};

    const FILES: [&str; 6] = ["a.pyx", "b.pyx", "c.pyx", "x.rs", "y.rs", "z.md"];

    fn taking(languages: &[Fake], reader: Option<&str>) -> Vec<&'static str> {
        FILES.into_iter().filter(|path| takes_environment(languages, reader, path)).collect()
    }

    #[test]
    fn a_batch_with_one_project_language_gives_the_environment_to_its_files_only() {
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
        ];
        let files = taking(&languages, None);
        assert_eq!(files.len(), 3, "the environment is built for the three files of the project language");
        assert_eq!(files, ["a.pyx", "b.pyx", "c.pyx"]);
    }

    #[test]
    fn no_language_that_uses_the_project_means_no_environment() {
        let languages = [Fake { name: "plain", extension: ".rs", project: false }];
        assert!(taking(&languages, None).is_empty());
    }

    #[test]
    fn a_request_that_names_a_reader_keeps_the_environment_for_every_file() {
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
        ];
        assert_eq!(taking(&languages, Some("project")).len(), FILES.len());
        assert!(taking(&languages, Some("plain")).is_empty());
    }

    /// The session and the scheduler cannot disagree on a file's language: for every path the environment is taken exactly when
    /// a language the scheduler would read the file by uses the project, with and without a named reader, and a path two languages
    /// read (which the scheduler refuses) is routed to both.
    #[test]
    fn the_environment_follows_the_language_the_scheduler_routes_a_path_to() {
        use super::routed;
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
            Fake { name: "other", extension: ".x.rs", project: true },
        ];
        for path in ["a.pyx", "x.rs", "y.x.rs", "z.md", "dir/b.pyx", ""] {
            for reader in [None, Some("project"), Some("plain"), Some("other"), Some("absent")] {
                let routes: Vec<_> = routed(&languages, reader, path).map(|(index, _)| index).collect();
                let expected = routes.iter().any(|&index| languages[index].project);
                assert_eq!(takes_environment(&languages, reader, path), expected, "{path} read as {reader:?} goes to {routes:?}");
            }
        }
        assert_eq!(routed(&languages, None, "y.x.rs").map(|(index, _)| index).collect::<Vec<_>>(), [1, 2], "two recognizers read one path");
    }
}
