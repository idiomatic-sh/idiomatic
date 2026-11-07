//! The test support of the `scaffolding` concept: the captures, param declarations, template entries and templates its cases
//! are stated with.

use std::collections::BTreeMap;

use crate::domain::captures::types::{CaptureName, CaptureValue, Captures};
use crate::domain::definitions::types::{Exactness, TemplateFile, TemplateText, Why};
use crate::domain::entries::types::{
    Alias, Count, Described, Description, EntryHeader, Imports, ParamDeclaration,
    ParamDeclarations, ParamName, ParamSource, ParamType, ParamValues, TemplateEntry,
};
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::selectors::types::{PathPattern, Selectors};

/// Captures of the pairs given.
#[must_use]
pub fn captures(pairs: &[(&str, &str)]) -> Captures {
    Captures(
        pairs
            .iter()
            .map(|(name, value)| (CaptureName((*name).into()), CaptureValue((*value).into())))
            .collect(),
    )
}

/// One declared param, taken from where `source` says.
#[must_use]
pub fn declared(param: &str, source: ParamSource) -> ParamDeclarations {
    ParamDeclarations(BTreeMap::from([(
        ParamName(param.into()),
        ParamDeclaration {
            kind: ParamType("string".into()),
            description: Described::Unsaid,
            source,
        },
    )]))
}

/// A template entry placing `file`, `count` times.
#[must_use]
pub fn entry(file: &str, count: Count) -> TemplateEntry {
    TemplateEntry {
        header: EntryHeader {
            package: PackageName("p".into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
            alias: Alias::Unnamed,
            description: Described::Unsaid,
            include: Selectors::default(),
            exclude: Selectors::default(),
            params: ParamValues::default(),
        },
        template: DefinitionName("t".into()),
        file: PathPattern(file.into()),
        count,
        imports: Imports::Unrestricted,
    }
}

/// A template whose inline content is `text`.
#[must_use]
pub fn template(text: &str) -> TemplateFile {
    TemplateFile {
        description: Description("d".into()),
        why: Why("w".into()),
        params: ParamDeclarations::default(),
        exact: Exactness::Contains,
        content: TemplateText(text.into()),
        scenarios: Vec::new(),
    }
}
