//! The language port, implemented by the `ParseUseCase` and `ProjectUseCase` of each language the
//! edge linked, and the catalog of the `language` crate.

use super::constants::TWO_LANGUAGES_UNLISTED;
use crate::outbound::language::types::{Answers, LinkedNames, RelationList, UnknownReasons};
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, KindWord};

use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::semantics::interface::SemanticSession;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::types::ParseRequest;
use ryo_language::inbound::use_cases::project::types::ProjectRequest;
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::RowWriter;
use ryo_relations::domain::relations::types::{Bytes, Relation};

use crate::outbound::language::errors::LanguageError;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::LanguageHandles;
use crate::outbound::language::types::RoleWriting;
use crate::outbound::language::types::{
    Absence, FileReading, FileToRead, LinkedName, ManifestReading, NoteWriting, ProjectAnswer,
    ProjectInput, RefusedManifest, RelationAnswer,
};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::stored_facts::types::{FileNotes, LanguageNumber, ReadFailure};

/// The `use_case` language adapter.
#[derive(Debug)]
pub(crate) struct LanguageUseCaseAdapter {
    /// The languages linked, in the order they are listed and numbered in.
    languages: Vec<LanguageHandles>,
}

impl LanguageUseCaseAdapter {
    /// The adapter before any language is linked.
    pub(crate) fn new() -> Self {
        Self {
            languages: Vec::new(),
        }
    }

    /// The adapter with one more language linked, after those already linked: the order it lists and numbers them in.
    pub(crate) fn with(mut self, language: LanguageHandles) -> Self {
        self.languages.push(language);
        self
    }
}

fn parse_request(file: FileToRead<'_>) -> ParseRequest<'_> {
    ParseRequest {
        path: SourcePath(&file.path.0),
        file: file.file,
        bytes: ByteSlice(&file.bytes.0),
        fields: match file.roles {
            RoleWriting::Wanted => {
                ryo_language::inbound::use_cases::parse::types::FieldRequest::Wanted
            }
            RoleWriting::Unwanted => {
                ryo_language::inbound::use_cases::parse::types::FieldRequest::Unwanted
            }
        },
        notes: match file.notes {
            NoteWriting::Wanted => {
                ryo_language::inbound::use_cases::parse::types::NoteRequest::Wanted
            }
            NoteWriting::Unwanted => {
                ryo_language::inbound::use_cases::parse::types::NoteRequest::Unwanted
            }
        },
    }
}

impl LanguagePort for LanguageUseCaseAdapter {
    fn relations(&self) -> Result<RelationList, LanguageError> {
        Ok(RelationList(all().to_vec()))
    }

    fn linked(&self) -> LinkedNames {
        LinkedNames(
            self.languages
                .iter()
                .map(|linked| LinkedName(format!("{} {}", linked.name.0, linked.build.0).into()))
                .collect(),
        )
    }

    fn unknown_reasons(&self) -> UnknownReasons {
        let mut reasons: Vec<UnknownName> = Vec::new();
        for each in self
            .languages
            .iter()
            .flat_map(|linked| linked.parse.unknown_reasons())
        {
            if !reasons.contains(each) {
                reasons.push(*each);
            }
        }
        UnknownReasons(reasons)
    }

    fn kind_vocabularies(&self) -> KindVocabularies {
        KindVocabularies(
            self.languages
                .iter()
                .map(|linked| KindVocabulary {
                    language: linked.name.clone(),
                    kinds: linked
                        .parse
                        .kind_names()
                        .0
                        .into_iter()
                        .map(|name| KindWord(Box::from(name.0)))
                        .collect(),
                })
                .collect(),
        )
    }

    fn supports(&self, relation: Relation) -> Answers {
        Answers(
            self.languages
                .iter()
                .map(|linked| RelationAnswer {
                    language: linked.name.clone(),
                    absence: linked
                        .parse
                        .absence_of(relation)
                        .or_else(|| linked.project.absence_of(relation))
                        .map_or(Absence::Reads, Absence::Because),
                    unknowns: linked.parse.unknown_reasons(),
                })
                .collect(),
        )
    }

    fn reads_manifest(&self, path: &FilePath) -> ManifestReading {
        let name = path.0.rsplit('/').next().unwrap_or(&path.0);
        let reads = self.languages.iter().any(|linked| {
            linked
                .project
                .manifest_names()
                .iter()
                .any(|manifest| manifest.0 == name)
        });
        if reads {
            ManifestReading::Read
        } else {
            ManifestReading::NotRead
        }
    }

    fn parse(
        &self,
        file: FileToRead<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> FileReading {
        let request = parse_request(file);
        if let Some(selected) = file.source_language {
            let mut matches = self
                .languages
                .iter()
                .enumerate()
                .filter(|(_, language)| language.name == *selected);
            let Some((number, language)) = matches.next() else {
                return FileReading::Failed(ReadFailure(
                    format!("explicit source provider `{}` is not linked", selected.0).into(),
                ));
            };
            if matches.next().is_some() {
                return FileReading::Failed(ReadFailure(
                    format!("explicit source provider `{}` is ambiguous", selected.0).into(),
                ));
            }
            return match language.parse.read_file_explicit(
                request,
                table,
                symbols,
                file.wanted,
                out,
            ) {
                Ok(()) => FileReading::Read(LanguageNumber(number)),
                Err(error) => FileReading::Failed(ReadFailure(
                    format!("explicit source provider `{}`: {error}", selected.0).into(),
                )),
            };
        }
        let mut failure: Option<ReadFailure> = None;
        for (number, language) in self.languages.iter().enumerate() {
            if language.parse.recognition(&request)
                == ryo_language::inbound::use_cases::parse::types::Recognition::Declines
            {
                continue;
            }
            match language
                .parse
                .read_file(request, table, symbols, file.wanted, out)
            {
                Ok(()) => return FileReading::Read(LanguageNumber(number)),
                Err(ParseError::Unsupported) => {}
                Err(other) => failure = Some(ReadFailure(other.to_string().into())),
            }
        }
        failure.map_or_else(
            || {
                if self.reads_manifest(file.path) == ManifestReading::Read {
                    FileReading::Manifest
                } else {
                    FileReading::Unsupported
                }
            },
            FileReading::Failed,
        )
    }

    fn begin_semantics(
        &self,
        language: LanguageNumber,
        file: FileToRead<'_>,
        tree: &Elements,
    ) -> Option<Box<dyn SemanticSession>> {
        self.languages
            .get(language.0)
            .and_then(|handle| handle.parse.begin_semantics(&parse_request(file), tree))
    }

    fn project(
        &self,
        input: &ProjectInput<'_>,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<ProjectAnswer, LanguageError> {
        let mut refused = Vec::new();
        let mut unlisted = Vec::new();
        let mut consulted = Vec::new();
        for (number, language) in self.languages.iter().enumerate() {
            let request = ProjectRequest {
                root: input.root.clone(),
                repository: input.repository,
                manifests: input.manifests.to_vec(),
                files: input.files.iter().map(|each| each.path.clone()).collect(),
                notes: input
                    .notes
                    .0
                    .iter()
                    .map(|each| match each {
                        // A language reads only the notes it wrote.
                        Some(FileNotes {
                            language: written_by,
                            notes,
                        }) if written_by.0 == number => Bytes(notes.0.clone()),
                        Some(_) | None => Bytes::default(),
                    })
                    .collect(),
                wanted: input.wanted.to_vec(),
            };
            let bytes = language
                .project
                .project(&request, symbols, out)
                .map_err(|error| LanguageError::Manifest {
                    reason: error.to_string(),
                })?;
            consulted.extend(bytes.consulted);
            // The numbers after the files the run lists are one language's to give: two would number different manifests alike. Two
            // languages that read the same manifests name the same ones in the same order, which is one numbering.
            if !bytes.unlisted.is_empty() {
                let named: Vec<FilePath> = bytes.unlisted;
                if !unlisted.is_empty() && unlisted != named {
                    return Err(LanguageError::Manifest {
                        reason: TWO_LANGUAGES_UNLISTED.to_owned(),
                    });
                }
                unlisted = named;
            }
            refused.extend(bytes.refused.into_iter().map(|each| RefusedManifest {
                language: language.name.clone(),
                path: each.path,
                reason: each.reason,
            }));
        }
        // A manifest the rows are about that the run does not list was read from the disk: it is consulted too.
        consulted.extend(unlisted.iter().cloned());
        consulted.sort_unstable();
        consulted.dedup();
        Ok(ProjectAnswer {
            refused,
            unlisted,
            consulted,
        })
    }
}
