//! Builds the `facts` adapters. Every dependency arrives as a parameter from the edge that composes: the language that parsed the
//! file, the tree it built, the file's bytes and number. This is the one place the adapters are composed: a relation another
//! adapter makes (a semantic one, say) is added here and in its own adapter directory, and nowhere else.

use super::adapters::location::adapter::FactsLocationAdapter;
use super::adapters::semantic::adapter::FactsSemanticAdapter;
use super::adapters::syntax::adapter::FactsSyntaxAdapter;
use super::interface::Facts;
use ryo_language::domain::elements::types::Elements;
use ryo_language::inbound::use_cases::parse::types::ParseEngine;
use ryo_relations::domain::relations::types::FileId;

/// The facts of one parsed file: its tree and the relations its language's readers write (`syntax`), name binding and resolution
/// (`semantic`), and `source-location` from the file's line index (`location`).
#[must_use]
pub fn file<'a>(
    language: &'a dyn ParseEngine,
    elements: &'a mut Elements,
    source: &'a [u8],
    file: FileId,
) -> impl Facts + 'a {
    FactsLocationAdapter::new(
        FactsSemanticAdapter::new(FactsSyntaxAdapter::new(language, elements, source, file)),
        source,
    )
}
