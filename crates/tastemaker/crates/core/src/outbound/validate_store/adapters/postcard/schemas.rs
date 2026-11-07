//! The shape of `validate`'s store on disk: postcard's bytes of the build that wrote it and of flat, compact tables, a
//! row a thing: `definitions`, a version that held each, and `definition_uses`, a version one of them uses each. Every
//! text is kept once, in `strings`, and a row names it by number.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::digests::types::Digest;
use crate::domain::files::types::Stamp;
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, PackageName, Release, VersionText,
};
use crate::domain::proofs::types::ProvenVersion;

/// The number a text is kept under in `strings`.
type Text = u32;

/// A version: what it is, by number (rule 0, spec 1, template 2), its package, name and version.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct VersionSchema {
    /// What it is.
    kind: u8,
    /// Its package.
    package: Text,
    /// Its name.
    name: Text,
    /// Its version.
    version: Text,
}

/// A row of `definitions`: a version that held, its digest, and the size and modification time of its files.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct DefinitionSchema {
    /// The version.
    version: VersionSchema,
    /// Its digest.
    digest: Text,
    /// Its files' sizes, its uses' with them, all told.
    size: u64,
    /// The latest time one of them was modified, in nanoseconds since the Unix epoch.
    modified: u64,
}

/// A row of `definition_uses`: the `definitions` row of a version, by number, and a version it uses.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct DefinitionUseSchema {
    /// The `definitions` row.
    definition: u32,
    /// The version it uses.
    uses: VersionSchema,
}

/// The store: the build that wrote it, every text, and the tables.
#[derive(Default, Serialize, Deserialize)]
pub(super) struct StoreSchema {
    /// The build that wrote it.
    build: Box<str>,
    /// Every text, each once.
    strings: Vec<Box<str>>,
    /// The versions that held.
    definitions: Vec<DefinitionSchema>,
    /// The versions each uses.
    definition_uses: Vec<DefinitionUseSchema>,
}

/// The `strings` table of a store being written: each text given its number once.
struct StringsSchema {
    /// The texts, in number order.
    strings: Vec<Box<str>>,
    /// Each text's number.
    numbers: HashMap<Box<str>, Text>,
}

impl StringsSchema {
    /// The number of `text`, given it when it has none.
    fn number(&mut self, text: &str) -> Text {
        if let Some(number) = self.numbers.get(text) {
            return *number;
        }
        let number = Text::try_from(self.strings.len()).unwrap_or(Text::MAX);
        self.strings.push(text.into());
        let _: Option<Text> = self.numbers.insert(text.into(), number);
        number
    }
}

/// What a kind is kept as.
const fn kind_number(kind: DefinitionKind) -> u8 {
    match kind {
        DefinitionKind::Rule => 0,
        DefinitionKind::Spec => 1,
        DefinitionKind::Template => 2,
    }
}

/// The kind a number names.
fn kind_of(number: u8) -> Option<DefinitionKind> {
    [
        DefinitionKind::Rule,
        DefinitionKind::Spec,
        DefinitionKind::Template,
    ]
    .into_iter()
    .find(|kind| kind_number(*kind) == number)
}

impl StoreSchema {
    /// The text numbered `number`.
    fn text(&self, number: Text) -> Option<&str> {
        self.strings
            .get(usize::try_from(number).ok()?) // ignore: idiomatic.arch/no-ok-on-result - a number past what usize holds names no text, and a row naming none reads the whole store as none
            .map(|text| &**text)
    }

    /// The release a version row names.
    fn release(&self, row: &VersionSchema) -> Option<Release> {
        Some(Release {
            package: PackageName(self.text(row.package)?.into()),
            kind: kind_of(row.kind)?,
            name: DefinitionName(self.text(row.name)?.into()),
            version: VersionText(self.text(row.version)?.into()),
            ownership: Ownership::Shared,
        })
    }

    /// The versions that held; none when a row does not read.
    fn definitions(&self) -> Option<Vec<ProvenVersion>> {
        let mut uses = vec![Vec::new(); self.definitions.len()];
        for row in &self.definition_uses {
            uses.get_mut(usize::try_from(row.definition).ok()?)? // ignore: idiomatic.arch/no-ok-on-result - a row past what usize holds names no definition, and a row that does not read reads the whole store as none (the doc above)
                .push(self.release(&row.uses)?);
        }
        self.definitions
            .iter()
            .zip(uses)
            .map(|(row, uses)| {
                Some(ProvenVersion {
                    release: self.release(&row.version)?,
                    stamp: Stamp {
                        size: crate::domain::files::types::FileSize(row.size),
                        modified: crate::domain::files::types::ModifiedTime(row.modified),
                    },
                    digest: Digest(self.text(row.digest)?.into()),
                    uses,
                })
            })
            .collect()
    }

    /// The store of `build` holding `rows`, each text kept once.
    fn with(build: &crate::domain::settings::types::Build, rows: &[ProvenVersion]) -> Self {
        let mut numbers = StringsSchema {
            strings: Vec::new(),
            numbers: HashMap::new(),
        };
        let version = |texts: &mut StringsSchema, release: &Release| VersionSchema {
            kind: kind_number(release.kind),
            package: texts.number(&release.package.0),
            name: texts.number(&release.name.0),
            version: texts.number(&release.version.0),
        };
        let mut definitions = Vec::with_capacity(rows.len());
        let mut definition_uses = Vec::new();
        for (at, row) in rows.iter().enumerate() {
            definitions.push(DefinitionSchema {
                version: version(&mut numbers, &row.release),
                digest: numbers.number(&row.digest.0),
                size: (row.stamp.size).0,
                modified: (row.stamp.modified).0,
            });
            for used in &row.uses {
                definition_uses.push(DefinitionUseSchema {
                    definition: u32::try_from(at).unwrap_or(u32::MAX),
                    uses: version(&mut numbers, used),
                });
            }
        }
        Self {
            build: build.0.clone(),
            strings: numbers.strings,
            definitions,
            definition_uses,
        }
    }
}

/// A kept store as a run reads it: its bytes, and the build that runs.
pub(super) struct ReadSchema<'a> {
    /// The bytes kept.
    pub bytes: &'a [u8],
    /// The build that runs.
    pub build: &'a crate::domain::settings::types::Build,
}

/// Why a kept store holds nothing for the run that reads it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReadSchemaError {
    /// Another build wrote it.
    OtherBuild,
    /// Its bytes are no store, or a row names a text or a row it does not hold.
    Unread,
}

impl TryFrom<ReadSchema<'_>> for Vec<ProvenVersion> {
    type Error = ReadSchemaError;

    fn try_from(read: ReadSchema<'_>) -> Result<Self, Self::Error> {
        let kept: StoreSchema =
            postcard::from_bytes(read.bytes).map_err(|_postcard_error| ReadSchemaError::Unread)?; // bytes that are no store are `Unread`: the store is rebuilt, never migrated
        if *kept.build != *read.build.0 {
            return Err(ReadSchemaError::OtherBuild);
        }
        kept.definitions().ok_or(ReadSchemaError::Unread)
    }
}

/// The versions the kept store `bytes` holds when `build` wrote it; none when another build wrote it or it does not read.
pub(super) fn definitions(
    bytes: &[u8],
    build: &crate::domain::settings::types::Build,
) -> Option<Vec<ProvenVersion>> {
    Vec::<ProvenVersion>::try_from(ReadSchema { bytes, build }).ok() // ignore: idiomatic.arch/no-ok-on-result - a store another build wrote, or that does not read, holds nothing for this run (the doc above)
}

/// The bytes of the store of `build` holding `rows`, each text kept once.
///
/// # Errors
///
/// When postcard cannot write them.
pub(super) fn bytes(
    build: &crate::domain::settings::types::Build,
    rows: &[ProvenVersion],
) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&StoreSchema::with(build, rows))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
