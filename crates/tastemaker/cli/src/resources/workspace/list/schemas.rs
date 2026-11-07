//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use tastemaker_core::domain::packages::constants::{TEMPLATE_WORD, TEMPLATES};
use tastemaker_core::domain::packages::types::DefinitionKind;
use tastemaker_core::inbound::use_cases::list::types::{
    Kinds, ListReport, ListRequest, ListedKind, SearchTerm,
};

use tastemaker_core::domain::topics::constants::{OPERATOR_WORD, RELATION_WORD};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ListSchemaError {
    /// The kind is not one of the five.
    #[error("`{0}` is not a kind: rules, specs, templates, relations or operators")]
    Kind(String),
}

/// The arguments as the command line gives them, offered to the conversion.
struct ListArgumentsSchema<'a> {
    /// Which kind: `rules`, `specs`, `templates`, or ryo's `relations` or `operators`; every definition when left out.
    kind: Option<String>,
    /// Only the definitions whose name or description holds this text.
    term: Option<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<ListArgumentsSchema<'_>> for ListRequest {
    type Error = ListSchemaError;

    fn try_from(schema: ListArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let mut term = schema.term;
        let kinds = match schema.kind.as_deref() {
            None => Kinds::All,
            Some("rules" | "rule") => Kinds::Only(DefinitionKind::Rule),
            Some("specs" | "spec") => Kinds::Only(DefinitionKind::Spec),
            Some(TEMPLATES | TEMPLATE_WORD) => Kinds::Only(DefinitionKind::Template),
            Some("relations" | "relation") => Kinds::Relations,
            Some("operators" | "operator") => Kinds::Operators,
            // One word that is no kind is the term: `list length` lists every kind.
            Some(other) if term.is_none() => {
                term = Some(other.to_owned());
                Kinds::All
            }
            Some(other) => return Err(ListSchemaError::Kind(other.to_owned())),
        };
        Ok(Self {
            start: edge.start.clone(),
            kinds,
            term: SearchTerm(term.unwrap_or_default().into()),
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `Kind` for a kind that is not one of the six.
pub(super) fn request(
    kind: Option<String>,
    term: Option<String>,
    edge: &crate::edge::Edge,
) -> Result<ListRequest, ListSchemaError> {
    ListArgumentsSchema { kind, term, edge }.try_into()
}

/// One definition listed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct ListedSchema {
    /// `rule`, `spec`, `template`, `relation` or `operator`.
    pub kind: String,
    /// The package that holds it; a relation or an operator is ryo's and has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// Its name.
    pub name: String,
    /// Its latest version; a relation or an operator has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// What it is.
    pub description: String,
}

/// What `list` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The definitions.
    pub listed: Vec<ListedSchema>,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for listed in &self.listed {
            let named = match &listed.package {
                Some(package) => format!("{package}/{}", listed.name),
                None => listed.name.clone(),
            };
            match &listed.version {
                Some(version) => {
                    writeln!(
                        f,
                        "{} {named}@{version}  {}",
                        listed.kind, listed.description
                    )?;
                }
                None => writeln!(f, "{} {named}  {}", listed.kind, listed.description)?,
            }
        }
        Ok(())
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ListReport) -> AnswerSchema {
    AnswerSchema {
        listed: report
            .listed
            .into_iter()
            .map(|listed| ListedSchema {
                kind: match listed.kind {
                    ListedKind::Definition(kind) => {
                        tastemaker_core::domain::packages::service::kind_word(kind)
                            .0
                            .to_owned()
                    }
                    ListedKind::Relation => RELATION_WORD.to_owned(),
                    ListedKind::Operator => OPERATOR_WORD.to_owned(),
                },
                package: (!listed.package.0.is_empty()).then(|| listed.package.0.to_string()),
                name: listed.name.0.to_string(),
                version: (!listed.version.0.is_empty()).then(|| listed.version.0.to_string()),
                description: listed.description.0.to_string(),
            })
            .collect(),
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for ListSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::Kind(_) => crate::outcome::remedy::Next::Run("tm list".to_owned()),
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

impl crate::output::Encoded for AnswerSchema {
    fn json(
        &self,
        to: &mut dyn std::io::Write,
        spacing: crate::output::Spacing,
    ) -> std::io::Result<()> {
        match spacing {
            crate::output::Spacing::Compact => serde_json::to_writer(to, self),
            crate::output::Spacing::Indented => serde_json::to_writer_pretty(to, self),
        }
        .map_err(std::io::Error::from)
    }
}
